use glium::glutin::{
    DeviceEvent, ElementState, Event, KeyboardInput, MouseButton, VirtualKeyCode, WindowEvent,
};

use cgmath::prelude::*;
use cgmath::Vector3;

use clamp::clamp;

use std::f32::consts::PI;
use std::time::SystemTime;

use client::{Client, GameState, Graphics, PlayerState, SightBlock, VoxelType};
use {client, physics};

const TURN_SPEED: f32 = 0.01;
const DOUBLE_PRESS_THRESH: f32 = 0.3; // TODO: Is this a good value?

fn handle_mouse_input(state: &mut GameState, mouse_state: ElementState, btn: MouseButton) {
    let down = mouse_state == ElementState::Pressed;
    state.mouse_btns_down.insert(btn, down);
}

fn handle_window_event(ev: &WindowEvent, state: &mut GameState) {
    match ev {
        WindowEvent::CloseRequested => state.running = false,
        WindowEvent::MouseInput {
            state: mouse_state,
            button,
            ..
        } => handle_mouse_input(state, *mouse_state, *button),
        _ => {}
    }
}

// Handle the forward key being pressed. Check/set the double-tap-to-run timer.
fn do_press_forward(state: &mut GameState) {
    if state.player.state == PlayerState::Normal {
        if let Some(time) = state.timers.run_press_timer {
            let dt = client::get_time_delta(&time);
            if dt < DOUBLE_PRESS_THRESH {
                state.player.state = PlayerState::Running;
                state.timers.since_run_timer = Some(SystemTime::now());
            }
        }
        state.timers.run_press_timer = Some(SystemTime::now());
    }
}

// Change game state based on a keypress. This is needed because `do_keys_down()` only knows if a
// key is currently down.
fn do_key_press(key: VirtualKeyCode, state: &mut GameState) {
    match key {
        VirtualKeyCode::Tab => physics::toggle_flight(state),
        VirtualKeyCode::W => do_press_forward(state),
        _ => {}
    }
}

// Handle release of the forward key. Disable running if enabled.
fn do_release_forward(state: &mut GameState) {
    if state.player.state == PlayerState::Running {
        state.player.state = PlayerState::Normal;
        state.timers.since_run_timer = Some(SystemTime::now());
    }
}

// Change game state based on a key release. This is needed because `do_keys_down()` only knows if
// a key is currently down.
fn do_key_release(key: VirtualKeyCode, state: &mut GameState) {
    match key {
        VirtualKeyCode::W => do_release_forward(state),
        _ => {}
    }
}

fn handle_keyboard_input(inp: &KeyboardInput, state: &mut GameState) {
    let down = inp.state == ElementState::Pressed;
    if let Some(key) = inp.virtual_keycode {
        // TODO: Check for pause
        if down {
            do_key_press(key, state);
        } else {
            do_key_release(key, state);
        }
        // Log whether a key was pressed/released such that `do_keys_down()` knows if keys are down
        state.keys_down.insert(key, down);
    }
}

fn handle_device_event(ev: &DeviceEvent, state: &mut GameState) {
    match ev {
        // Change the player direction on mouse motion
        DeviceEvent::MouseMotion {
            delta: (dx, dy), ..
        } if !state.paused => {
            state.player.angle.x -= *dx as f32 * TURN_SPEED;
            state.player.angle.y -= *dy as f32 * TURN_SPEED;
            // Prevent the player from looking too high/low
            state.player.angle.y = clamp(-PI / 2.0, state.player.angle.y, PI / 2.0);
        }
        DeviceEvent::Key(inp) => handle_keyboard_input(inp, state),
        _ => {}
    }
}

// Try to convert the event to a UI event to pass to the UI library for internal handling
fn handle_ui_event(ev: Event, gfx: &mut Graphics) {
    if let Some(ui_ev) = conrod_winit::convert_event(ev, gfx.display.gl_window().window()) {
        gfx.ui.ui.handle_event(ui_ev);
    }
}

// Dispatch an event
fn handle_event(ev: Event, gfx: &mut Graphics, state: &mut GameState) {
    match ev {
        Event::WindowEvent { event: ref ev, .. } => handle_window_event(ev, state),
        Event::DeviceEvent { event: ref ev, .. } => handle_device_event(ev, state),
        _ => {}
    }
    handle_ui_event(ev, gfx);
}

// Process all the input events and modify state accordingly
pub fn do_input(Client { evs, gfx, state }: &mut Client) {
    evs.poll_events(|ev| handle_event(ev, gfx, state));
}

fn key_down(state: &GameState, key: VirtualKeyCode) -> bool {
    *state.keys_down.get(&key).unwrap_or(&false)
}

pub fn mouse_btn_down(state: &GameState, btn: MouseButton) -> bool {
    *state.mouse_btns_down.get(&btn).unwrap_or(&false)
}

// Process down keys to change the game state
pub fn do_keys_down(client: &mut Client) {
    let (forward, right, _) = physics::compute_dir_vectors(client.state.player.angle);
    // Discard the y component to prevent the player from floating when they walk forward while
    // looking up. The vectors are normalized to keep the speed constant.
    let forward = Vector3::new(forward.x, 0.0, forward.z).normalize();
    let right = right.normalize();
    let move_speed = physics::get_move_speed(client.state.player.state);

    // TODO: Make this clearer
    client.state.player.velocity.x = 0.0;
    client.state.player.velocity.z = 0.0;
    if !physics::player_in_freefall(&client.state) {
        // Jump/fly up
        client.state.player.velocity.y = if key_down(&client.state, VirtualKeyCode::Space) {
            move_speed
        } else {
            0.0
        }
    }
    // Move forward
    if key_down(&client.state, VirtualKeyCode::W) {
        client.state.player.velocity += forward * move_speed
    }
    // Move backward
    if key_down(&client.state, VirtualKeyCode::R) {
        client.state.player.velocity -= forward * move_speed
    }
    // Move left
    if key_down(&client.state, VirtualKeyCode::A) {
        client.state.player.velocity -= right * move_speed
    }
    // Move right
    if key_down(&client.state, VirtualKeyCode::S) {
        client.state.player.velocity += right * move_speed
    }
    // Move down
    if key_down(&client.state, VirtualKeyCode::LShift)
        && client.state.player.state == PlayerState::Flying
    {
        client.state.player.velocity.y = -move_speed
    }

    // Pause game
    if key_down(&client.state, VirtualKeyCode::Escape) {
        client::set_pause(&mut client.state, &client.gfx.display, true);
    }

    // Destroy sand
    if mouse_btn_down(&client.state, MouseButton::Left) {
        if let Some(SightBlock { pos, .. }) = client.state.sight_block {
            physics::put_voxel(&mut client.state, pos, VoxelType::Air);
        }
    }

    // Create sand
    if mouse_btn_down(&client.state, MouseButton::Right) {
        if let Some(SightBlock { new_pos, .. }) = client.state.sight_block {
            physics::put_voxel(&mut client.state, new_pos, VoxelType::Sand);
        }
    }
}
