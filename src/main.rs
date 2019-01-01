#[macro_use]
extern crate glium;
extern crate cgmath;

use glium::index::{NoIndices, PrimitiveType};
use glium::{glutin, Depth, Display, DrawParameters, Program, Surface, VertexBuffer};

use glutin::{
    ContextBuilder, DeviceEvent, ElementState, Event, EventsLoop, KeyboardInput, VirtualKeyCode,
    WindowBuilder, WindowEvent,
};

use cgmath::conv::array4x4;
use cgmath::prelude::*;
use cgmath::{perspective, Deg, Euler, Matrix4, Point3, Quaternion, Rad, Vector2, Vector3};

struct Graphics {
    display: Display,
    evs: EventsLoop,
    program: Program, // GLSL shader program
}

#[derive(Debug)]
struct Player {
    pos: Point3<f32>,
    angle: Vector2<f32>,
}

struct GameState {
    running: bool,
    player: Player,
    voxels: Box<[[[bool; VOX_H]; VOX_W]; VOX_L]>,
    dirty: bool,
}

struct Client {
    gfx: Graphics,
    state: GameState,
}

implement_vertex!(Vertex, pos, color);
#[derive(Clone, Copy)]
struct Vertex {
    pos: [f32; 3],
    color: [f32; 3],
}

const VOX_L: usize = 160;
const VOX_W: usize = 160;
const VOX_H: usize = 160;
const WIN_W: u32 = 800;
const WIN_H: u32 = 600;
const TURN_SPEED: f32 = 0.01;
const FOV: Deg<f32> = Deg(60.0);

impl Vertex {
    fn new(pos: [f32; 3], color: [f32; 3]) -> Vertex {
        Vertex { pos, color }
    }
}

impl Client {
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
            voxels: Box::new([[[false; VOX_H]; VOX_W]; VOX_L]),
            dirty: false,
        };
        Client { gfx, state }
    }
}

fn handle_window_event(ev: &WindowEvent, state: &mut GameState) {
    match ev {
        WindowEvent::CloseRequested => state.running = false,
        _ => {}
    }
}

fn handle_keyboard_input(inp: &KeyboardInput, state: &mut GameState) {
    if inp.state != ElementState::Pressed {
        return;
    }
    match inp.virtual_keycode {
        Some(VirtualKeyCode::W) => state.player.pos.z -= 1.0,
        Some(VirtualKeyCode::A) => state.player.pos.x -= 1.0,
        Some(VirtualKeyCode::R) => state.player.pos.z += 1.0,
        Some(VirtualKeyCode::S) => state.player.pos.x += 1.0,
        Some(VirtualKeyCode::Space) => state.player.pos.y += 1.0,
        Some(VirtualKeyCode::LShift) => state.player.pos.y -= 1.0,
        Some(_) | None => {}
    }
}

fn handle_device_event(ev: &DeviceEvent, state: &mut GameState) {
    match ev {
        // Change the player direction on mouse motion
        DeviceEvent::MouseMotion {
            delta: (dx, dy), ..
        } => {
            state.player.angle -= Vector2 {
                x: *dx as f32,
                y: *dy as f32,
            } * TURN_SPEED
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

// Compute the transformation matrix
fn compute_matrix(player: &Player, gfx: &Graphics) -> Matrix4<f32> {
    let forward = Quaternion::from(Euler {
        x: Rad(player.angle.y),
        y: Rad(player.angle.x),
        z: Rad(0.0),
    })
    .rotate_vector(Vector3::new(0.0, 0.0, -1.0));
    let right = forward.cross(Vector3::new(0.0, 1.0, 0.0));
    let up = right.cross(forward);
    let win_size = gfx.display.gl_window().window().get_inner_size().unwrap();
    let aspect_ratio = (win_size.width / win_size.height) as f32;
    let proj = perspective(FOV, aspect_ratio, 0.1, 100.0);
    let view = Matrix4::look_at_dir(player.pos, forward, up);
    proj * view
}

fn render(gfx: &mut Graphics, state: &GameState) {
    // Create a cube mesh
    // TODO: Make this mesh a global
    // TODO: Order vertices correctly for face culling
    let vbuf = VertexBuffer::new(
        &gfx.display,
        &[
            // View from -x
            Vertex::new([0.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
            Vertex::new([0.0, 0.0, 1.0], [0.0, 1.0, 0.0]),
            Vertex::new([0.0, 1.0, 0.0], [0.0, 1.0, 1.0]),
            Vertex::new([0.0, 1.0, 0.0], [1.0, 0.0, 0.0]),
            Vertex::new([0.0, 0.0, 1.0], [1.0, 0.0, 1.0]),
            Vertex::new([0.0, 1.0, 1.0], [1.0, 1.0, 0.0]),
            // View from +x
            Vertex::new([1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
            Vertex::new([1.0, 0.0, 1.0], [0.0, 1.0, 0.0]),
            Vertex::new([1.0, 1.0, 0.0], [0.0, 1.0, 1.0]),
            Vertex::new([1.0, 1.0, 0.0], [1.0, 0.0, 0.0]),
            Vertex::new([1.0, 0.0, 1.0], [1.0, 0.0, 1.0]),
            Vertex::new([1.0, 1.0, 1.0], [1.0, 1.0, 0.0]),
            // View from -y
            Vertex::new([0.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
            Vertex::new([0.0, 0.0, 1.0], [0.0, 1.0, 0.0]),
            Vertex::new([1.0, 0.0, 0.0], [0.0, 1.0, 1.0]),
            Vertex::new([1.0, 0.0, 0.0], [1.0, 0.0, 0.0]),
            Vertex::new([0.0, 0.0, 1.0], [1.0, 0.0, 1.0]),
            Vertex::new([1.0, 0.0, 1.0], [1.0, 1.0, 0.0]),
            // View from +y
            Vertex::new([0.0, 1.0, 0.0], [0.0, 0.0, 1.0]),
            Vertex::new([0.0, 1.0, 1.0], [0.0, 1.0, 0.0]),
            Vertex::new([1.0, 1.0, 0.0], [0.0, 1.0, 1.0]),
            Vertex::new([1.0, 1.0, 0.0], [1.0, 0.0, 0.0]),
            Vertex::new([0.0, 1.0, 1.0], [1.0, 0.0, 1.0]),
            Vertex::new([1.0, 1.0, 1.0], [1.0, 1.0, 0.0]),
            // View from -z
            Vertex::new([0.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
            Vertex::new([0.0, 1.0, 0.0], [0.0, 1.0, 0.0]),
            Vertex::new([1.0, 0.0, 0.0], [0.0, 1.0, 1.0]),
            Vertex::new([1.0, 0.0, 0.0], [1.0, 0.0, 0.0]),
            Vertex::new([0.0, 1.0, 0.0], [1.0, 0.0, 1.0]),
            Vertex::new([1.0, 1.0, 0.0], [1.0, 1.0, 0.0]),
            // View from +z
            Vertex::new([0.0, 0.0, 1.0], [0.0, 0.0, 1.0]),
            Vertex::new([0.0, 1.0, 1.0], [0.0, 1.0, 0.0]),
            Vertex::new([1.0, 0.0, 1.0], [0.0, 1.0, 1.0]),
            Vertex::new([1.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
            Vertex::new([0.0, 1.0, 1.0], [1.0, 0.0, 1.0]),
            Vertex::new([1.0, 1.0, 1.0], [1.0, 1.0, 0.0]),
        ],
    )
    .unwrap();
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

fn main() {
    let mut client = Client::init();

    while client.state.running {
        do_input(&mut client.gfx, &mut client.state);
        render(&mut client.gfx, &client.state);
    }
}
