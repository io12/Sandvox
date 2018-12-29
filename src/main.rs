#[macro_use]
extern crate glium;

use glium::index::PrimitiveType;
use glium::{glutin, Surface};

struct Graphics {
    display: glium::Display,
    evs: glutin::EventsLoop,
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

const L: usize = 160;
const W: usize = 160;
const H: usize = 160;

impl Client {
    fn init() -> Client {
        let win = glutin::WindowBuilder::new();
        let ctx = glutin::ContextBuilder::new();
        let evs = glutin::EventsLoop::new();
        let display = glium::Display::new(win, ctx, &evs).unwrap();
        let gfx = Graphics { display, evs };
        let state = GameState {
            running: true,
            voxels: Box::new([[[false; H]; W]; L]),
            dirty: false,
        };
        Client { gfx, state }
    }
}

// TODO: Destructing can possibly be used here and in other places
fn do_input(gfx: &mut Graphics, state: &mut GameState) {
    gfx.evs.poll_events(|ev| match ev {
        glutin::Event::WindowEvent { event: ev, .. } => match ev {
            // Break from the main loop when the window is closed.
            glutin::WindowEvent::CloseRequested => state.running = false,
            // Redraw the triangle when the window is resized.
            //glutin::WindowEvent::Resized(..) => draw(),
            _ => {}
        },
        _ => {}
    });
}

fn render(gfx: &mut Graphics) {
    let vertex_buffer = {
        #[derive(Copy, Clone)]
        struct Vertex {
            position: [f32; 2],
            color: [f32; 3],
        }

        implement_vertex!(Vertex, position, color);

        glium::VertexBuffer::new(
            &gfx.display,
            &[
                Vertex {
                    position: [-0.5, -0.5],
                    color: [0.0, 1.0, 0.0],
                },
                Vertex {
                    position: [0.0, 0.5],
                    color: [0.0, 0.0, 1.0],
                },
                Vertex {
                    position: [0.5, -0.5],
                    color: [1.0, 0.0, 0.0],
                },
            ],
        )
        .unwrap()
    };

    // building the index buffer
    let index_buffer =
        glium::IndexBuffer::new(&gfx.display, PrimitiveType::TrianglesList, &[0u16, 1, 2]).unwrap();

    // compiling shaders and linking them together
    let program = glium::Program::from_source(
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
