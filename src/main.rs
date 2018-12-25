extern crate gl;
extern crate sdl2;

use sdl2::event::Event;
use sdl2::video::GLProfile;

// Dimensions of the voxel space
const L: usize = 160;
const W: usize = 160;
const H: usize = 160;
const VOXEL_VERTEX_BUF_SIZE: usize = 6 * 6 * 4;
const VOXELS_VERTEX_BUF_SIZE: usize = VOXEL_VERTEX_BUF_SIZE * L * W * H;

struct GameState {
    // true = sand, false = air
    voxels: [[[bool; L]; W]; H],
}

impl GameState {
    fn new() -> GameState {
        GameState{ voxels: [[[false; L]; W]; H] }
    }
}

fn init() -> (sdl2::video::Window, sdl2::EventPump, sdl2::video::GLContext) {
    let sdl_ctx = sdl2::init().unwrap();
    let video_subsystem = sdl_ctx.video().unwrap();

    let window = video_subsystem
        .window("sandvox", 800, 600)
        .resizable()
        .opengl()
        .build()
        .unwrap();

    let gl_attr = video_subsystem.gl_attr();
    gl_attr.set_context_profile(GLProfile::Core);
    gl_attr.set_context_version(3, 3);

    let gl_ctx = window.gl_create_context().unwrap();
    window.gl_make_current(&gl_ctx).unwrap();

    gl::load_with(|name| video_subsystem.gl_get_proc_address(name) as *const _);

    let event_pump = sdl_ctx.event_pump().unwrap();

    (window, event_pump, gl_ctx)
}

fn handle_input(event_pump: &mut sdl2::EventPump) {
    for event in event_pump.poll_iter() {
        match event {
            Event::Quit { .. } => std::process::exit(0),
            _ => {}
        }
    }
}

// Generate a vertex buffer for a voxel at (x, y, z)
fn gen_voxel_vertex_buf(x: u8, y: u8, z: u8, w: u8) -> [u8; VOXEL_VERTEX_BUF_SIZE] {
    [
        // From -x
        x,     y,     z,     w,
        x,     y,     z + 1, w,
        x,     y + 1, z,     w,
        x,     y + 1, z,     w,
        x,     y,     z + 1, w,
        x,     y + 1, z + 1, w,

        // From +x
        x + 1, y,     z,     w,
        x + 1, y,     z + 1, w,
        x + 1, y + 1, z,     w,
        x + 1, y + 1, z,     w,
        x + 1, y,     z + 1, w,
        x + 1, y + 1, z + 1, w,

        // From -y
        x,     y,     z,     w,
        x,     y,     z + 1, w,
        x + 1, y,     z,     w,
        x + 1, y,     z,     w,
        x,     y,     z + 1, w,
        x + 1, y,     z + 1, w,

        // From +y
        x,     y + 1, z,     w,
        x,     y + 1, z + 1, w,
        x + 1, y + 1, z,     w,
        x + 1, y + 1, z,     w,
        x,     y + 1, z + 1, w,
        x + 1, y + 1, z + 1, w,

        // From -z
        x,     y,     z,     w,
        x,     y + 1, z,     w,
        x + 1, y,     z,     w,
        x + 1, y,     z,     w,
        x,     y + 1, z,     w,
        x + 1, y + 1, z,     w,

        // From +z
        x,     y,     z + 1, w,
        x,     y + 1, z + 1, w,
        x + 1, y,     z + 1, w,
        x + 1, y,     z + 1, w,
        x,     y + 1, z + 1, w,
        x + 1, y + 1, z + 1, w,
    ]
}

fn gen_voxels_vertex_buf(game_state: &GameState) -> [u8; VOXELS_VERTEX_BUF_SIZE] {
    let buf = [0; VOXELS_VERTEX_BUF_SIZE];
    buf
}

fn render(window: &sdl2::video::Window) {
    unsafe {
        gl::ClearColor(1.0, 1.0, 1.0, 1.0);
        gl::Clear(gl::COLOR_BUFFER_BIT);
    }
    window.gl_swap_window();
}

fn main() {
    // The GL context is returned even though it is unused, because things break otherwise
    let (window, mut event_pump, _gl_ctx) = init();
    let mut game_state = GameState::new();

    loop {
        handle_input(&mut event_pump);
        render(&window);
    }
}
