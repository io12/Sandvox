#[macro_use]
extern crate glium;
extern crate cgmath;
extern crate clamp;

use glium::index::{NoIndices, PrimitiveType};
use glium::{glutin, Depth, Display, DrawParameters, Frame, Program, Surface, VertexBuffer};

use glutin::dpi::LogicalSize;
use glutin::{
    ContextBuilder, DeviceEvent, ElementState, Event, EventsLoop, KeyboardInput, MouseButton,
    VirtualKeyCode, WindowBuilder, WindowEvent,
};

use cgmath::conv::array4x4;
use cgmath::prelude::*;
use cgmath::{ortho, perspective, Deg, Euler, Matrix4, Point3, Quaternion, Rad, Vector2, Vector3};

use clamp::clamp;

use std::collections::HashMap;
use std::f32::consts::PI;
use std::time::{Duration, SystemTime};

// A direction along an axis
enum Dir {
    PosX,
    NegX,
    PosY,
    NegY,
    PosZ,
    NegZ,
}

struct Graphics {
    display: Display,
    evs: EventsLoop,
    program: Program, // GLSL shader program
}

struct Player {
    pos: Point3<f32>,
    angle: Vector2<f32>,
}

struct GameState {
    running: bool,
    paused: bool,
    frame: u32,
    player: Player,
    voxels: Box<[[[bool; VOX_H]; VOX_W]; VOX_L]>,
    voxels_mesh: Vec<VertexU8>,
    dirty: bool,
    keys_pressed: HashMap<VirtualKeyCode, bool>,
}

struct Client {
    gfx: Graphics,
    state: GameState,
}

implement_vertex!(VertexU8, pos, color);
#[derive(Clone, Copy)]
struct VertexU8 {
    pos: [u8; 3],
    color: [u8; 3],
}

implement_vertex!(VertexF32, pos, color);
#[derive(Clone, Copy)]
struct VertexF32 {
    pos: [f32; 3],
    color: [f32; 3],
}

const VOX_L: usize = 160;
const VOX_W: usize = 160;
const VOX_H: usize = 160;
const WIN_W: u32 = 800;
const WIN_H: u32 = 600;
const TURN_SPEED: f32 = 0.01;
const MOVE_SPEED: f32 = 0.01;
const FOV: Deg<f32> = Deg(60.0);
const INIT_POS: Point3<f32> = Point3 {
    x: 0.0,
    y: 1.5, // TODO: Each voxel is 1 cm and the camera is 1.5 m above ground
    z: 0.0,
};
const CROSSHAIRS_SIZE: f32 = 15.0;
const BLOCK_SEL_DIST: usize = 100;
const RAYCAST_STEP: f32 = 0.1;

impl VertexU8 {
    fn new(pos: [u8; 3], color: [u8; 3]) -> Self {
        Self { pos, color }
    }
}

impl VertexF32 {
    fn new(pos: [f32; 3], color: [f32; 3]) -> Self {
        Self { pos, color }
    }
}

impl Client {
    // Create a window, initialize OpenGL, compile the GLSL shaders, and initialize a client struct
    fn init() -> Client {
        let win_size = (WIN_W, WIN_H).into();
        let win = WindowBuilder::new().with_dimensions(win_size);
        let ctx = ContextBuilder::new().with_depth_buffer(24);
        let evs = EventsLoop::new();
        let display = Display::new(win, ctx, &evs).unwrap();
        // Compile program from GLSL shaders
        let program = Program::from_source(
            &display,
            include_str!("shaders/vert.glsl"),
            include_str!("shaders/frag.glsl"),
            None,
        )
        .unwrap();

        let gfx = Graphics {
            display,
            evs,
            program,
        };
        let player = Player {
            pos: INIT_POS,
            angle: Vector2::new(0.0, 0.0),
        };
        let mut state = GameState {
            running: true,
            paused: true,
            frame: 0,
            player,
            voxels: make_test_world(),
            voxels_mesh: Vec::new(),
            dirty: true,
            keys_pressed: HashMap::new(),
        };
        set_pause(&mut state, &gfx.display, false);
        Client { gfx, state }
    }
}

// Create an initial diagonal stripe test world
// TODO: Remove this
fn make_test_world() -> Box<[[[bool; VOX_H]; VOX_W]; VOX_L]> {
    let mut voxels = Box::new([[[false; VOX_H]; VOX_W]; VOX_L]);
    for x in 0..VOX_L {
        for y in 0..VOX_W {
            for z in 0..VOX_H {
                if x == y && y == z {
                    voxels[x][y][z] = true;
                }
            }
        }
    }
    voxels
}

// Pause/unpause the game
// TODO: Dim screen on pause
fn set_pause(state: &mut GameState, display: &Display, paused: bool) {
    let grab = !paused;
    display.gl_window().window().grab_cursor(grab).unwrap();
    display.gl_window().window().hide_cursor(grab);
    state.paused = paused;
}

fn handle_mouse_input(
    state: &mut GameState,
    display: &Display,
    mouse_state: ElementState,
    btn: MouseButton,
) {
    if mouse_state != ElementState::Pressed {
        return;
    }
    match btn {
        MouseButton::Left => {
            if state.paused {
                set_pause(state, display, false);
            } else {
                // TODO: Destroy sand
            }
        }
        _ => {}
    }
}

fn handle_window_event(ev: &WindowEvent, display: &Display, state: &mut GameState) {
    match ev {
        WindowEvent::CloseRequested => state.running = false,
        WindowEvent::MouseInput {
            state: mouse_state,
            button,
            ..
        } => handle_mouse_input(state, display, *mouse_state, *button),
        _ => {}
    }
}

// Log whether a key was pressed/released such that `do_movement` knows if keys are held
fn handle_keyboard_input(inp: &KeyboardInput, state: &mut GameState) {
    if let Some(key) = inp.virtual_keycode {
        match inp.state {
            ElementState::Pressed => state.keys_pressed.insert(key, true),
            ElementState::Released => state.keys_pressed.insert(key, false),
        };
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
fn handle_event(ev: &Event, display: &Display, state: &mut GameState) {
    match ev {
        Event::WindowEvent { event: ev, .. } => handle_window_event(&ev, display, state),
        Event::DeviceEvent { event: ev, .. } => handle_device_event(&ev, state),
        _ => {}
    }
}

// TODO: Destructing can possibly be used here and in other places
fn do_input(gfx: &mut Graphics, state: &mut GameState) {
    let display = &gfx.display;
    gfx.evs.poll_events(|ev| handle_event(&ev, display, state));
}

fn key_pressed(state: &GameState, key: VirtualKeyCode) -> bool {
    *state.keys_pressed.get(&key).unwrap_or(&false)
}

// Process pressed keys to move the player
fn do_movement(client: &mut Client, dt: f32) {
    let (forward, right, _) = compute_dir_vectors(&client.state.player.angle);
    // Discard the y component to prevent the player from floating when they walk forward while
    // looking up. The vectors are normalized to keep the speed constant.
    let forward = Vector3::new(forward.x, 0.0, forward.z).normalize();
    let right = right.normalize();

    // Multiply by the time delta so speed of motion is constant (even if framerate isn't)

    // Move forward
    if key_pressed(&client.state, VirtualKeyCode::W) {
        client.state.player.pos += forward * dt * MOVE_SPEED
    }
    // Move backward
    if key_pressed(&client.state, VirtualKeyCode::R) {
        client.state.player.pos -= forward * dt * MOVE_SPEED
    }
    // Move left
    if key_pressed(&client.state, VirtualKeyCode::A) {
        client.state.player.pos -= right * dt * MOVE_SPEED
    }
    // Move right
    if key_pressed(&client.state, VirtualKeyCode::S) {
        client.state.player.pos += right * dt * MOVE_SPEED
    }
    // Move up
    if key_pressed(&client.state, VirtualKeyCode::Space) {
        client.state.player.pos.y += dt * MOVE_SPEED
    }
    // Move down
    if key_pressed(&client.state, VirtualKeyCode::LShift) {
        client.state.player.pos.y -= dt * MOVE_SPEED
    }

    if key_pressed(&client.state, VirtualKeyCode::Escape) {
        set_pause(&mut client.state, &client.gfx.display, true);
    }
}

// Propagate the voxels downwards (gravity)
// TODO: Somehow use `dt` here
fn do_sandfall(state: &mut GameState) {
    if state.frame % 10 == 0 {
        // TODO: Find a better way to iterate over voxels
        for x in 0..VOX_L {
            for y in 0..VOX_W {
                for z in 0..VOX_H {
                    // TODO: Make this less boilerplate
                    if state.voxels[x][y][z] && y > 0 && !state.voxels[x][y - 1][z] {
                        state.voxels[x][y][z] = false;
                        state.voxels[x][y - 1][z] = true;
                        state.dirty = true;
                    }
                }
            }
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

// Get the pixel size
fn get_win_size(gfx: &Graphics) -> LogicalSize {
    gfx.display.gl_window().window().get_inner_size().unwrap()
}

// Get the aspect ratio of the window
fn get_aspect_ratio(gfx: &Graphics) -> f32 {
    let LogicalSize { width, height } = get_win_size(gfx);
    (width / height) as f32
}

// Compute the transformation matrix. Each vertex is multiplied by the matrix so it renders in the
// correct position relative to the player.
fn compute_matrix(player: &Player, gfx: &Graphics) -> Matrix4<f32> {
    let (forward, _, up) = compute_dir_vectors(&player.angle);
    let aspect_ratio = get_aspect_ratio(gfx);
    let proj = perspective(FOV, aspect_ratio, 0.1, 1000.0);
    let view = Matrix4::look_at_dir(player.pos, forward, up);
    proj * view
}

// Make a mesh of the voxel world
fn make_voxels_mesh(state: &GameState) -> Vec<VertexU8> {
    // TODO: Make this mesh a global
    let cube_vertices = [
        VertexU8::new([0, 0, 0], [0, 0, 1]),
        VertexU8::new([0, 0, 1], [0, 1, 0]),
        VertexU8::new([0, 1, 1], [1, 0, 0]),
        VertexU8::new([1, 1, 0], [1, 0, 0]),
        VertexU8::new([0, 0, 0], [0, 1, 0]),
        VertexU8::new([0, 1, 0], [0, 0, 1]),
        VertexU8::new([1, 0, 1], [0, 1, 0]),
        VertexU8::new([0, 0, 0], [1, 0, 0]),
        VertexU8::new([1, 0, 0], [0, 1, 0]),
        VertexU8::new([1, 1, 0], [0, 0, 1]),
        VertexU8::new([1, 0, 0], [0, 1, 0]),
        VertexU8::new([0, 0, 0], [1, 0, 0]),
        VertexU8::new([0, 0, 0], [0, 1, 0]),
        VertexU8::new([0, 1, 1], [0, 0, 1]),
        VertexU8::new([0, 1, 0], [0, 1, 0]),
        VertexU8::new([1, 0, 1], [1, 0, 0]),
        VertexU8::new([0, 0, 1], [0, 1, 0]),
        VertexU8::new([0, 0, 0], [0, 0, 1]),
        VertexU8::new([0, 1, 1], [0, 1, 0]),
        VertexU8::new([0, 0, 1], [1, 0, 0]),
        VertexU8::new([1, 0, 1], [0, 1, 0]),
        VertexU8::new([1, 1, 1], [0, 0, 1]),
        VertexU8::new([1, 0, 0], [0, 1, 0]),
        VertexU8::new([1, 1, 0], [1, 0, 0]),
        VertexU8::new([1, 0, 0], [0, 1, 0]),
        VertexU8::new([1, 1, 1], [0, 0, 1]),
        VertexU8::new([1, 0, 1], [0, 1, 0]),
        VertexU8::new([1, 1, 1], [1, 0, 0]),
        VertexU8::new([1, 1, 0], [0, 1, 0]),
        VertexU8::new([0, 1, 0], [0, 0, 1]),
        VertexU8::new([1, 1, 1], [0, 1, 0]),
        VertexU8::new([0, 1, 0], [1, 0, 0]),
        VertexU8::new([0, 1, 1], [0, 1, 0]),
        VertexU8::new([1, 1, 1], [0, 0, 1]),
        VertexU8::new([0, 1, 1], [0, 1, 0]),
        VertexU8::new([1, 0, 1], [1, 0, 0]),
    ];

    let mut mesh = Vec::new();
    // Iterate through all the voxels, creating a cube mesh for each
    for x in 0..VOX_L {
        for y in 0..VOX_W {
            for z in 0..VOX_H {
                if state.voxels[x][y][z] {
                    for v in cube_vertices.iter() {
                        mesh.push(VertexU8::new(
                            [v.pos[0] + x as u8, v.pos[1] + y as u8, v.pos[2] + z as u8],
                            v.color,
                        ));
                    }
                }
            }
        }
    }
    mesh
}

// Determine if there is a voxel at `pos`, returning `None` when the position isn't within the
// bounds of the voxel grid
fn voxel_at_opt(state: &GameState, pos: Point3<f32>) -> Option<bool> {
    Some(
        *state
            .voxels
            .get(pos.x as usize)?
            .get(pos.y as usize)?
            .get(pos.z as usize)?,
    )
}

// Determine if the is a voxel at `pos`, returning `false` when the position is out of bounds
fn voxel_at(state: &GameState, pos: Point3<f32>) -> bool {
    voxel_at_opt(state, pos).unwrap_or(false)
}

// Get the coordinates of the block the player is looking directly at and the direction of the face
// being viewed. This is the box that a wireframe is drawn around and is modified by left/right
// clicks. This function returns `None` if no voxel is in the player's line of sight.
//
// TODO: Test if this is accurate
fn get_sight_block(state: &GameState) -> Option<(Point3<u8>, Dir)> {
    let forward = compute_forward_vector(&state.player.angle);
    let mut pos = state.player.pos;
    // Raycasting
    for _ in 0..BLOCK_SEL_DIST {
        let prev_pos = pos;
        pos += forward * RAYCAST_STEP;
        if voxel_at(state, pos) {
            // Now that the voxel is known, compute the face being observed. Because voxel_at()
            // returned true this iteration, but not last time, comparing integer coords can
            // determine the face.
            let x = pos.x as u32;
            let y = pos.y as u32;
            let z = pos.z as u32;
            let prev_x = prev_pos.x as u32;
            let prev_y = prev_pos.y as u32;
            let prev_z = prev_pos.z as u32;
            let face = if x > prev_x {
                Dir::PosX
            } else if x < prev_x {
                Dir::NegX
            } else if y > prev_y {
                Dir::PosY
            } else if y < prev_y {
                Dir::NegY
            } else if z > prev_z {
                Dir::PosZ
            } else if z < prev_z {
                Dir::NegZ
            } else {
                // All the previous vs current coords are equal, which isn't possible when
                // voxel_at() returns `true` for the first time
                unreachable!()
            };
            return Some((pos.cast()?, face));
        }
    }
    None
}

// Create a line wireframe mesh for the voxel in the player's line of sight. The return type is an
// `Option` because there might not be a voxel in the line of sight.
fn make_wireframe_mesh(state: &GameState) -> Option<[VertexU8; 48]> {
    let (Point3 { x, y, z }, face) = get_sight_block(state)?;
    let color = [1, 1, 1];
    // Array of lines (not triangles)
    Some([
        // From -x
        VertexU8::new([x, y, z], color),
        VertexU8::new([x, y + 1, z], color),
        VertexU8::new([x, y + 1, z], color),
        VertexU8::new([x, y + 1, z + 1], color),
        VertexU8::new([x, y + 1, z + 1], color),
        VertexU8::new([x, y, z + 1], color),
        VertexU8::new([x, y, z + 1], color),
        VertexU8::new([x, y, z], color),
        // From +x
        VertexU8::new([x + 1, y, z], color),
        VertexU8::new([x + 1, y + 1, z], color),
        VertexU8::new([x + 1, y + 1, z], color),
        VertexU8::new([x + 1, y + 1, z + 1], color),
        VertexU8::new([x + 1, y + 1, z + 1], color),
        VertexU8::new([x + 1, y, z + 1], color),
        VertexU8::new([x + 1, y, z + 1], color),
        VertexU8::new([x + 1, y, z], color),
        // From -y
        VertexU8::new([x, y, z], color),
        VertexU8::new([x + 1, y, z], color),
        VertexU8::new([x + 1, y, z], color),
        VertexU8::new([x + 1, y, z + 1], color),
        VertexU8::new([x + 1, y, z + 1], color),
        VertexU8::new([x, y, z + 1], color),
        VertexU8::new([x, y, z + 1], color),
        VertexU8::new([x, y, z], color),
        // From +y
        VertexU8::new([x, y + 1, z], color),
        VertexU8::new([x + 1, y + 1, z], color),
        VertexU8::new([x + 1, y + 1, z], color),
        VertexU8::new([x + 1, y + 1, z + 1], color),
        VertexU8::new([x + 1, y + 1, z + 1], color),
        VertexU8::new([x, y + 1, z + 1], color),
        VertexU8::new([x, y + 1, z + 1], color),
        VertexU8::new([x, y + 1, z], color),
        // From -z
        VertexU8::new([x, y, z], color),
        VertexU8::new([x + 1, y, z], color),
        VertexU8::new([x + 1, y, z], color),
        VertexU8::new([x + 1, y + 1, z], color),
        VertexU8::new([x + 1, y + 1, z], color),
        VertexU8::new([x, y + 1, z], color),
        VertexU8::new([x, y + 1, z], color),
        VertexU8::new([x, y, z], color),
        // From +z
        VertexU8::new([x, y, z + 1], color),
        VertexU8::new([x + 1, y, z + 1], color),
        VertexU8::new([x + 1, y, z + 1], color),
        VertexU8::new([x + 1, y + 1, z + 1], color),
        VertexU8::new([x + 1, y + 1, z + 1], color),
        VertexU8::new([x, y + 1, z + 1], color),
        VertexU8::new([x, y + 1, z + 1], color),
        VertexU8::new([x, y, z + 1], color),
    ])
}

// Make a new mesh of the voxels, but only if the world changed since the last frame
fn maybe_make_voxels_mesh(state: &mut GameState) {
    if state.dirty {
        state.voxels_mesh = make_voxels_mesh(state);
        state.dirty = false;
    }
}

// Render the voxel grid, creating a new mesh if the world changed since the last frame
fn render_voxels(
    gfx: &mut Graphics,
    state: &mut GameState,
    matrix: Matrix4<f32>,
    target: &mut Frame,
) {
    let uniforms = uniform! {
        matrix: array4x4(matrix)
    };
    maybe_make_voxels_mesh(state);
    let vbuf = VertexBuffer::new(&gfx.display, &state.voxels_mesh).unwrap();
    // Do not use an index buffer
    let ibuf = NoIndices(PrimitiveType::TrianglesList);

    let params = DrawParameters {
        depth: Depth {
            test: glium::draw_parameters::DepthTest::IfLess,
            write: true,
            ..Default::default()
        },
        backface_culling: glium::draw_parameters::BackfaceCullingMode::CullClockwise,
        ..Default::default()
    };
    target
        .draw(&vbuf, &ibuf, &gfx.program, &uniforms, &params)
        .unwrap();
}

// Render a wireframe around the voxel in the player's line of sight, but only if there is a voxel
// in the player's line of sight.
fn render_wireframe(gfx: &Graphics, state: &GameState, matrix: Matrix4<f32>, target: &mut Frame) {
    if let Some(mesh) = make_wireframe_mesh(state) {
        let uniforms = uniform! {
            matrix: array4x4(matrix)
        };
        let vbuf = VertexBuffer::new(&gfx.display, &mesh).unwrap();
        // Do not use an index buffer
        let ibuf = NoIndices(PrimitiveType::LinesList);
        let params = DrawParameters {
            line_width: Some(5.0),
            ..Default::default()
        };
        target
            .draw(&vbuf, &ibuf, &gfx.program, &uniforms, &params)
            .unwrap();
    }
}

// Make a crosshairs mesh based on the window dimenions
fn make_crosshairs_mesh() -> [VertexF32; 4] {
    let color = [1.0, 1.0, 1.0];
    let sz = CROSSHAIRS_SIZE;
    [
        VertexF32::new([-sz, 0.0, 0.0], color),
        VertexF32::new([sz, 0.0, 0.0], color),
        VertexF32::new([0.0, -sz, 0.0], color),
        VertexF32::new([0.0, sz, 0.0], color),
    ]
}

// Based on the window size, compute the transformation matrix for 2D objects (such as a HUD)
fn compute_2d_matrix(gfx: &Graphics) -> Matrix4<f32> {
    let LogicalSize { width, height } = get_win_size(gfx);
    let w = width as f32;
    let h = height as f32;
    ortho(-w, w, -h, h, -1.0, 1.0)
}

// Generate a crosshairs mesh and render it
fn render_crosshairs(gfx: &Graphics, target: &mut Frame) {
    let matrix = compute_2d_matrix(gfx);
    let uniforms = uniform! {
        matrix: array4x4(matrix)
    };
    let mesh = make_crosshairs_mesh();
    let vbuf = VertexBuffer::new(&gfx.display, &mesh).unwrap();
    // Do not use an index buffer
    let ibuf = NoIndices(PrimitiveType::LinesList);
    let params = DrawParameters {
        line_width: Some(5.0),
        ..Default::default()
    };
    target
        .draw(&vbuf, &ibuf, &gfx.program, &uniforms, &params)
        .unwrap();
}

// Create meshes for the game objects and render them with OpenGL
fn render(gfx: &mut Graphics, state: &mut GameState) {
    let matrix = compute_matrix(&state.player, gfx);
    let mut target = gfx.display.draw();
    // Initialize rendering
    target.clear_color_and_depth((0.0, 0.0, 0.0, 1.0), 1.0);

    // Render each component
    render_voxels(gfx, state, matrix, &mut target);
    render_wireframe(gfx, state, matrix, &mut target);
    render_crosshairs(gfx, &mut target);

    // Swap buffers to finalize rendering
    target.finish().unwrap();
}

// Get the time since `prev_time` in milliseconds
fn get_time_delta(prev_time: &SystemTime) -> f32 {
    let elapsed = prev_time.elapsed().unwrap_or(Duration::new(0, 0));
    elapsed.as_secs() as f32 * 1000.0 + elapsed.subsec_millis() as f32
}

fn main() {
    let mut client = Client::init();

    // Time of the previous frame
    let mut prev_time = SystemTime::now();
    // Gameloop
    while client.state.running {
        let dt = get_time_delta(&prev_time);
        prev_time = SystemTime::now();
        do_input(&mut client.gfx, &mut client.state);
        if !client.state.paused {
            do_movement(&mut client, dt);
            do_sandfall(&mut client.state);
        }
        render(&mut client.gfx, &mut client.state);
        client.state.frame += 1;
    }
}
