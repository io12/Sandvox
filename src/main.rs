#[macro_use]
extern crate glium;

use glium::index::PrimitiveType;
use glium::{glutin, Display, IndexBuffer, Program, Surface, VertexBuffer};

use glutin::{ContextBuilder, Event, EventsLoop, WindowBuilder, WindowEvent};

struct Graphics {
    display: Display,
    evs: EventsLoop,
}

struct GameState {
    running: bool,
    voxels: Box<[[[bool; H]; W]; L]>,
    dirty: bool,
}

struct Client {
    gfx: Graphics,
    state: GameState,
}

implement_vertex!(Vertex, pos, color);
#[derive(Clone, Copy)]
struct Vertex {
    pos: [f32; 2],
    color: [f32; 3],
}

const L: usize = 160;
const W: usize = 160;
const H: usize = 160;

impl Client {
    fn init() -> Client {
        let win = WindowBuilder::new();
        let ctx = ContextBuilder::new();
        let evs = EventsLoop::new();
        let display = Display::new(win, ctx, &evs).unwrap();
        let gfx = Graphics { display, evs };
        let state = GameState {
            running: true,
            voxels: Box::new([[[false; H]; W]; L]),
            dirty: false,
        };
        Client { gfx, state }
    }
}

fn handle_window_event(ev: &WindowEvent, state: &mut GameState) {
    match ev {
        // Break from the main loop when the window is closed.
        WindowEvent::CloseRequested => state.running = false,
        // Redraw the triangle when the window is resized.
        //WindowEvent::Resized(..) => draw(),
        _ => {}
    }
}

fn handle_event(ev: &Event, state: &mut GameState) {
    match ev {
        Event::WindowEvent { event: ev, .. } => handle_window_event(&ev, state),
        _ => {}
    }
}

// TODO: Destructing can possibly be used here and in other places
fn do_input(gfx: &mut Graphics, state: &mut GameState) {
    gfx.evs.poll_events(|ev| handle_event(&ev, state));
}

fn render(gfx: &mut Graphics) {
    let vertex_buffer = {
        VertexBuffer::new(
            &gfx.display,
            &[
                Vertex {
                    pos: [-0.5, -0.5],
                    color: [0.0, 1.0, 0.0],
                },
                Vertex {
                    pos: [0.0, 0.5],
                    color: [0.0, 0.0, 1.0],
                },
                Vertex {
                    pos: [0.5, -0.5],
                    color: [1.0, 0.0, 0.0],
                },
            ],
        )
        .unwrap()
    };

    // building the index buffer
    let index_buffer =
        IndexBuffer::new(&gfx.display, PrimitiveType::TrianglesList, &[0u16, 1, 2]).unwrap();

    // compiling shaders and linking them together
    let program = Program::from_source(
        &gfx.display,
        include_str!("shaders/vert.glsl"),
        include_str!("shaders/frag.glsl"),
        None,
    )
    .unwrap();

    // Here we draw the black background and triangle to the screen using the previously
    // initialised resources.
    //
    // In this case we use a closure for simplicity, however keep in mind that most serious
    // applications should probably use a function that takes the resources as an argument.
    let draw = || {
        // building the uniforms
        let uniforms = uniform! {
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0f32]
            ]
        };

        // drawing a frame
        let mut target = gfx.display.draw();
        target.clear_color(0.0, 0.0, 0.0, 0.0);
        target
            .draw(
                &vertex_buffer,
                &index_buffer,
                &program,
                &uniforms,
                &Default::default(),
            )
            .unwrap();
        target.finish().unwrap();
    };

    // Draw the triangle to the screen.
    draw();
}

fn main() {
    let mut client = Client::init();
    // building the vertex buffer, which contains all the vertices that we will draw

    while client.state.running {
        do_input(&mut client.gfx, &mut client.state);
        render(&mut client.gfx);
    }
}
