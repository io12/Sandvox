#[macro_use]
extern crate gfx;
extern crate gfx_device_gl;
extern crate gfx_window_glutin;
extern crate glutin;

use gfx::format::DepthStencil;
use gfx::format::Rgba8;
use gfx::handle::DepthStencilView;
use gfx::handle::RenderTargetView;

use gfx_window_glutin as gfx_glutin;

use glutin::ContextBuilder;
use glutin::Event;
use glutin::EventsLoop;
use glutin::GlWindow;
use glutin::WindowBuilder;
use glutin::WindowEvent;

use gfx_device_gl::Device;
use gfx_device_gl::Factory;
use gfx_device_gl::Resources;

use std::boxed::Box;

const L: usize = 160;
const W: usize = 160;
const H: usize = 160;
const VOX_SIZE: f32 = 1.0 / 32.0;
const CUBE_VERTICES: [Vertex; 24] = [
    // top (0, 0, 1)
    Vertex::new([-1, -1, 1], [0, 0]),
    Vertex::new([1, -1, 1], [1, 0]),
    Vertex::new([1, 1, 1], [1, 1]),
    Vertex::new([-1, 1, 1], [0, 1]),
    // bottom (0, 0, -1)
    Vertex::new([-1, 1, -1], [1, 0]),
    Vertex::new([1, 1, -1], [0, 0]),
    Vertex::new([1, -1, -1], [0, 1]),
    Vertex::new([-1, -1, -1], [1, 1]),
    // right (1, 0, 0)
    Vertex::new([1, -1, -1], [0, 0]),
    Vertex::new([1, 1, -1], [1, 0]),
    Vertex::new([1, 1, 1], [1, 1]),
    Vertex::new([1, -1, 1], [0, 1]),
    // left (-1, 0, 0)
    Vertex::new([-1, -1, 1], [1, 0]),
    Vertex::new([-1, 1, 1], [0, 0]),
    Vertex::new([-1, 1, -1], [0, 1]),
    Vertex::new([-1, -1, -1], [1, 1]),
    // front (0, 1, 0)
    Vertex::new([1, 1, -1], [1, 0]),
    Vertex::new([-1, 1, -1], [0, 0]),
    Vertex::new([-1, 1, 1], [0, 1]),
    Vertex::new([1, 1, 1], [1, 1]),
    // back (0, -1, 0)
    Vertex::new([1, -1, 1], [0, 0]),
    Vertex::new([-1, -1, 1], [1, 0]),
    Vertex::new([-1, -1, -1], [1, 1]),
    Vertex::new([1, -1, -1], [0, 1]),
];
const CUBE_INDEX_DATA: [u16; 36] = [
    0, 1, 2, 2, 3, 0, // top
    4, 5, 6, 6, 7, 4, // bottom
    8, 9, 10, 10, 11, 8, // right
    12, 13, 14, 14, 15, 12, // left
    16, 17, 18, 18, 19, 16, // front
    20, 21, 22, 22, 23, 20, // back
];

struct Graphics {
    evs: EventsLoop,
    win: GlWindow,
    dev: Device,
    factory: Factory,
    color_view: RenderTargetView<Resources, Rgba8>,
    depth_view: DepthStencilView<Resources, DepthStencil>,
}

struct GameState {
    running: bool,
    // true = sand, false = air
    voxels: Box<[[[bool; H]; W]; L]>,
    // Dirty flag to check if the voxels have changed
    dirty: bool,
}

struct Client {
    gfx: Graphics,
    state: GameState,
}

gfx_defines! {
    vertex Vertex {
        pos: [i8; 4] = "a_Pos",
        tex_coord: [i8; 2] = "a_TexCoord",
    }

    constant Locals {
        transform: [[f32; 4]; 4] = "u_Transform",
    }

    pipeline pipe {
        locals: gfx::ConstantBuffer<Locals> = "Locals",
        color: gfx::TextureSampler<[f32; 4]> = "t_Color",
        out_color: gfx::RenderTarget<Rgba8> = "Target0",
        out_depth: gfx::DepthTarget<DepthStencil> =
            gfx::preset::depth::LESS_EQUAL_WRITE,
    }
}

impl Vertex {
    fn new(p: [i8; 3], t: [i8; 2]) -> Vertex {
        Vertex {
            pos: [p[0], p[1], p[2], 1],
            tex_coord: t,
        }
    }
}

fn handle_window_event(ev: &WindowEvent, state: &mut GameState) {
    match ev {
        WindowEvent::CloseRequested => state.running = false,
        _ => {}
    }
}

fn handle_event(ev: &Event, state: &mut GameState) {
    match ev {
        Event::WindowEvent { event: ev, .. } => handle_window_event(&ev, state),
        _ => {}
    }
}

fn do_input(evs: &mut EventsLoop, state: &mut GameState) {
    evs.poll_events(|ev| handle_event(&ev, state));
}

impl Client {
    fn init() -> Client {
        let evs = EventsLoop::new();
        let win_builder = WindowBuilder::new().with_title("SandVox");
        let ctx_builder = ContextBuilder::new().with_vsync(true);
        let (win, dev, factory, color_view, depth_view) =
            gfx_glutin::init::<Rgba8, DepthStencil>(win_builder, ctx_builder, &evs).unwrap();
        let pso = factory
            .create_pipeline_simple(
                include_bytes!("shaders/vert.glsl"),
                include_bytes!("shaders/frag.glsl"),
                pipe::new(),
            )
            .unwrap();
        let graphics = Graphics {
            evs,
            win,
            dev,
            factory,
            color_view,
            depth_view,
        };
        let state = GameState {
            running: true,
            voxels: Box::new([[[false; H]; W]; L]),
            dirty: false,
        };
        Client {
            gfx: graphics,
            state,
        }
    }

    /*
    fn reset_world(&mut self) {
        // TODO: Remove this test world
        for x in 0..L {
            for y in 0..W {
                for z in 0..H {
                    self.voxels[x][y][z] = x == y && x == z;
                }
            }
        }
        self.dirty = true;
    }

    fn update_state(&mut self) {
        // Make sand fall
        for x in 0..L {
            for y in 0..W {
                for z in 0..H {
                    // TODO: Swapping might not be the best way
                    let vox = self.voxels[x][y][z];
                    let low_vox = if y > 0 {
                        self.voxels[x][y - 1][z]
                    } else {
                        true
                    };
                    if vox && !low_vox {
                        // Swap sand blocks
                        self.voxels[x][y][z] = low_vox;
                        self.voxels[x][y - 1][z] = vox;
                        self.dirty = true;
                    }
                }
            }
        }
    }

    fn make_voxel_mesh(&mut self, x: usize, y: usize, z: usize) -> Mesh {
        let x = x as f32 * VOX_SIZE;
        let y = y as f32 * VOX_SIZE;
        let z = z as f32 * VOX_SIZE;
        let geo = Geometry::cuboid(VOX_SIZE, VOX_SIZE, VOX_SIZE);
        let material = three::material::Basic {
            color: 0xFFFF00,
            ..Default::default()
        };
        let mesh = self.win.factory.mesh(geo, material);
        mesh.set_position([x, y, z]);
        mesh
    }
    */

    fn render(&mut self) {
        // TODO: Think of a cleaner way to do this
        if self.state.dirty {
            // Add voxels
            for x in 0..L {
                for y in 0..W {
                    for z in 0..H {
                        if self.state.voxels[x][y][z] {
                            //let mesh = self.make_voxel_mesh(x, y, z);
                            //self.win.scene.add(mesh);
                        }
                    }
                }
            }
            self.state.dirty = false;
        }
        self.gfx.win.swap_buffers().unwrap();
    }
}

fn main() {
    let mut client = Client::init();

    while client.state.running {
        do_input(&mut client.gfx.evs, &mut client.state);
        client.render();
    }
}
