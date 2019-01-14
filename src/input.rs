use glium::glutin::{
    DeviceEvent, ElementState, Event, EventsLoop, KeyboardInput, MouseButton, VirtualKeyCode,
    WindowEvent,
};

use cgmath::prelude::*;
use cgmath::{Euler, Quaternion, Rad, Vector2, Vector3};

use clamp::clamp;

use std::f32::consts::PI;

use client::{Client, GameState, SightBlock, VoxelType};
use {client, physics};

const TURN_SPEED: f32 = 0.01;
const FLY_SPEED: f32 = 30.0; // In m/s
const WALK_SPEED: f32 = 5.0; // In m/s

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

// Change game state based on a keypress. This is needed because `do_keys_down()` only knows if a
// key is currently down.
fn do_key_press(key: VirtualKeyCode, state: &mut GameState) {
    match key {
        VirtualKeyCode::Tab => state.player.flying = !state.player.flying,
        _ => {}
    }
}

fn handle_keyboard_input(inp: &KeyboardInput, state: &mut GameState) {
    let down = inp.state == ElementState::Pressed;
    if let Some(key) = inp.virtual_keycode {
        if down {
            do_key_press(key, state);
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

// Dispatch an event
fn handle_event(ev: &Event, state: &mut GameState) {
    match ev {
        Event::WindowEvent { event: ev, .. } => handle_window_event(&ev, state),
        Event::DeviceEvent { event: ev, .. } => handle_device_event(&ev, state),
        _ => {}
    }
}

// TODO: Destructing can possibly be used here and in other places
pub fn do_input(evs: &mut EventsLoop, state: &mut GameState) {
    evs.poll_events(|ev| handle_event(&ev, state));
}

fn key_down(state: &GameState, key: VirtualKeyCode) -> bool {
    *state.keys_down.get(&key).unwrap_or(&false)
}

pub fn mouse_btn_down(state: &GameState, btn: MouseButton) -> bool {
    *state.mouse_btns_down.get(&btn).unwrap_or(&false)
}

// Process down keys to change the game state
pub fn do_keys_down(client: &mut Client) {
    let (forward, right, _) = compute_dir_vectors(&client.state.player.angle);
    // Discard the y component to prevent the player from floating when they walk forward while
    // looking up. The vectors are normalized to keep the speed constant.
    let forward = Vector3::new(forward.x, 0.0, forward.z).normalize();
    let right = right.normalize();
    let move_speed = if client.state.player.flying {
        FLY_SPEED
    } else {
        WALK_SPEED
    };

    // TODO: Make this clearer
    if !physics::player_in_freefall(&client.state) {
        client.state.player.velocity = Vector3::new(0.0, 0.0, 0.0);
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
        // Jump/fly up
        if key_down(&client.state, VirtualKeyCode::Space) {
            client.state.player.velocity.y = move_speed
        }
        // Move down
        if key_down(&client.state, VirtualKeyCode::LShift) && client.state.player.flying {
            client.state.player.velocity.y = -move_speed
        }
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

// Calculate the forward vector based on the player angle
fn compute_forward_vector(angle: &Vector2<f32>) -> Vector3<f32> {
    // The initial vector is rotated on each axis individually, because doing both rotations at
    // once causes issues.
    // TODO: Find a better way to do this
    Quaternion::from(Euler {
        x: Rad(0.0),
        y: Rad(angle.x),
        z: Rad(0.0),
    })
    .rotate_vector(
        Quaternion::from(Euler {
            x: Rad(angle.y),
            y: Rad(0.0),
            z: Rad(0.0),
        })
        .rotate_vector(Vector3::new(0.0, 0.0, -1.0)),
    )
}

// Compute the (forward, right, up) vectors for the player angle
fn compute_dir_vectors(angle: &Vector2<f32>) -> (Vector3<f32>, Vector3<f32>, Vector3<f32>) {
    let forward = compute_forward_vector(angle);
    let right = forward.cross(Vector3::new(0.0, 1.0, 0.0));
    let up = right.cross(forward);
    (forward, right, up)
}
