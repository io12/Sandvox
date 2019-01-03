#[macro_use]
extern crate glium;
extern crate cgmath;
extern crate clamp;

use glium::index::{NoIndices, PrimitiveType};
use glium::{glutin, Depth, Display, DrawParameters, Program, Surface, VertexBuffer};

use glutin::{
    ContextBuilder, DeviceEvent, ElementState, Event, EventsLoop, KeyboardInput, VirtualKeyCode,
    WindowBuilder, WindowEvent,
};

use cgmath::conv::array4x4;
use cgmath::prelude::*;
use cgmath::{perspective, Deg, Euler, Matrix4, Point3, Quaternion, Rad, Vector2, Vector3};

use clamp::clamp;

use std::collections::HashMap;
use std::f32::consts::PI;
use std::time::{Duration, SystemTime};

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
    player: Player,
    voxels: Box<[[[bool; VOX_H]; VOX_W]; VOX_L]>,
    mesh: Vec<Vertex>,
    dirty: bool,
    keys_pressed: HashMap<VirtualKeyCode, bool>,
}

struct Client {
    gfx: Graphics,
    state: GameState,
}

implement_vertex!(Vertex, pos, color);
#[derive(Clone, Copy)]
struct Vertex {
    pos: [u8; 3],
    color: [u8; 3],
}

const VOX_L: usize = 160;
const VOX_W: usize = 160;
const VOX_H: usize = 160;
const WIN_W: u32 = 800;
const WIN_H: u32 = 600;
const TURN_SPEED: f32 = 0.01;
const MOVE_SPEED: f32 = 0.01;
const FOV: Deg<f32> = Deg(60.0);

impl Vertex {
    fn new(pos: [u8; 3], color: [u8; 3]) -> Vertex {
        Vertex { pos, color }
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
            pos: Point3::new(0.0, 0.0, 0.0),
            angle: Vector2::new(0.0, 0.0),
        };
        let state = GameState {
            running: true,
            player,
            voxels: make_test_world(),
            mesh: Vec::new(),
            dirty: true,
            keys_pressed: HashMap::new(),
        };
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

fn handle_window_event(ev: &WindowEvent, state: &mut GameState) {
    match ev {
        WindowEvent::CloseRequested => state.running = false,
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
        } => {
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
fn do_input(gfx: &mut Graphics, state: &mut GameState) {
    gfx.evs.poll_events(|ev| handle_event(&ev, state));
}

fn key_pressed(state: &mut GameState, key: VirtualKeyCode) -> bool {
    *state.keys_pressed.get(&key).unwrap_or(&false)
}

fn do_movement(state: &mut GameState, dt: f32) {
    let (forward, right, _) = compute_dir_vectors(&state.player.angle);
    // Discard the y component to prevent the player from floating when they walk forward while
    // looking up. The vectors are normalized to keep the speed constant.
    let forward = Vector3::new(forward.x, 0.0, forward.z).normalize();
    let right = right.normalize();

    // Multiply by the time delta so speed of motion is constant (even if framerate isn't)

    // Move forward
    if key_pressed(state, VirtualKeyCode::W) {
        state.player.pos += forward * dt * MOVE_SPEED
    }
    // Move backward
    if key_pressed(state, VirtualKeyCode::R) {
        state.player.pos -= forward * dt * MOVE_SPEED
    }
    // Move left
    if key_pressed(state, VirtualKeyCode::A) {
        state.player.pos -= right * dt * MOVE_SPEED
    }
    // Move right
    if key_pressed(state, VirtualKeyCode::S) {
        state.player.pos += right * dt * MOVE_SPEED
    }
    // Move up
    if key_pressed(state, VirtualKeyCode::Space) {
        state.player.pos.y += dt * MOVE_SPEED
    }
    // Move down
    if key_pressed(state, VirtualKeyCode::LShift) {
        state.player.pos.y -= dt * MOVE_SPEED
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

// Compute the transformation matrix. Each vertex is multiplied by the matrix so it renders in the
// correct position relative to the player.
fn compute_matrix(player: &Player, gfx: &Graphics) -> Matrix4<f32> {
    let (forward, _, up) = compute_dir_vectors(&player.angle);
    let win_size = gfx.display.gl_window().window().get_inner_size().unwrap();
    let aspect_ratio = (win_size.width / win_size.height) as f32;
    let proj = perspective(FOV, aspect_ratio, 0.1, 100.0);
    let view = Matrix4::look_at_dir(player.pos, forward, up);
    proj * view
}

// Make a mesh of the voxel world
fn make_mesh(state: &GameState) -> Vec<Vertex> {
    // TODO: Make this mesh a global
    let cube_vertices = [
        Vertex::new([0, 0, 0], [0, 0, 1]),
        Vertex::new([0, 0, 1], [0, 1, 0]),
        Vertex::new([0, 1, 1], [1, 0, 0]),
        Vertex::new([1, 1, 0], [1, 0, 0]),
        Vertex::new([0, 0, 0], [0, 1, 0]),
        Vertex::new([0, 1, 0], [0, 0, 1]),
        Vertex::new([1, 0, 1], [0, 1, 0]),
        Vertex::new([0, 0, 0], [1, 0, 0]),
        Vertex::new([1, 0, 0], [0, 1, 0]),
        Vertex::new([1, 1, 0], [0, 0, 1]),
        Vertex::new([1, 0, 0], [0, 1, 0]),
        Vertex::new([0, 0, 0], [1, 0, 0]),
        Vertex::new([0, 0, 0], [0, 1, 0]),
        Vertex::new([0, 1, 1], [0, 0, 1]),
        Vertex::new([0, 1, 0], [0, 1, 0]),
        Vertex::new([1, 0, 1], [1, 0, 0]),
        Vertex::new([0, 0, 1], [0, 1, 0]),
        Vertex::new([0, 0, 0], [0, 0, 1]),
        Vertex::new([0, 1, 1], [0, 1, 0]),
        Vertex::new([0, 0, 1], [1, 0, 0]),
        Vertex::new([1, 0, 1], [0, 1, 0]),
        Vertex::new([1, 1, 1], [0, 0, 1]),
        Vertex::new([1, 0, 0], [0, 1, 0]),
        Vertex::new([1, 1, 0], [1, 0, 0]),
        Vertex::new([1, 0, 0], [0, 1, 0]),
        Vertex::new([1, 1, 1], [0, 0, 1]),
        Vertex::new([1, 0, 1], [0, 1, 0]),
        Vertex::new([1, 1, 1], [1, 0, 0]),
        Vertex::new([1, 1, 0], [0, 1, 0]),
        Vertex::new([0, 1, 0], [0, 0, 1]),
        Vertex::new([1, 1, 1], [0, 1, 0]),
        Vertex::new([0, 1, 0], [1, 0, 0]),
        Vertex::new([0, 1, 1], [0, 1, 0]),
        Vertex::new([1, 1, 1], [0, 0, 1]),
        Vertex::new([0, 1, 1], [0, 1, 0]),
        Vertex::new([1, 0, 1], [1, 0, 0]),
    ];

    let mut mesh = Vec::new();
    // Iterate through all the voxels, creating a cube mesh for each
    for x in 0..VOX_L {
        for y in 0..VOX_W {
            for z in 0..VOX_H {
                if state.voxels[x][y][z] {
                    for v in cube_vertices.iter() {
                        mesh.push(Vertex::new(
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

fn render(gfx: &mut Graphics, state: &mut GameState) {
    if state.dirty {
        state.mesh = make_mesh(state);
        state.dirty = false;
    }
    let vbuf = VertexBuffer::new(&gfx.display, &state.mesh).unwrap();
    // Do not use an index buffer
    let ibuf = NoIndices(PrimitiveType::TrianglesList);

    let matrix = compute_matrix(&state.player, gfx);
    let uniforms = uniform! {
        matrix: array4x4(matrix)
    };

    // TODO: Move this somewhere
    let params = DrawParameters {
        depth: Depth {
            test: glium::draw_parameters::DepthTest::IfLess,
            write: true,
            ..Default::default()
        },
        backface_culling: glium::draw_parameters::BackfaceCullingMode::CullClockwise,
        ..Default::default()
    };
    let mut target = gfx.display.draw();
    target.clear_color_and_depth((0.0, 0.0, 0.0, 1.0), 1.0);
    target
        .draw(&vbuf, &ibuf, &gfx.program, &uniforms, &params)
        .unwrap();
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
        do_movement(&mut client.state, dt);
        render(&mut client.gfx, &mut client.state);
    }
}
