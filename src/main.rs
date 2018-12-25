extern crate gl;
extern crate sdl2;

use std::str;
use std::fs;
use std::ffi::CString;

use sdl2::event::Event;
use sdl2::video::GLProfile;
use sdl2::video::Window;

// Dimensions of the voxel space
const L: usize = 160;
const W: usize = 160;
const H: usize = 160;
const VOXEL_VERTEX_BUF_SIZE: usize = 6 * 6 * 4;
const VOXELS_VERTEX_BUF_SIZE: usize = VOXEL_VERTEX_BUF_SIZE * L * W * H;

struct Graphics {
    window: sdl2::video::Window,
    event_pump: sdl2::EventPump,
    // The GL context is included even though it is unused, because things break otherwise
    gl_ctx: sdl2::GLContext,
}

struct GameState {
    // 1 = sand, 0 = air
    voxels: [[[u8; L]; W]; H],
}

struct Client {
    gfx: Graphics,
    state: GameState,
}

impl GameState {
    fn new() -> GameState {
        GameState {
            voxels: [[[0; L]; W]; H],
        }
    }
}

fn load_shader(shader_type: gl::glEnum, path: &str) -> gl::GLuint {
    let shader = gl::CreateShader(shader_type);
    let src = CString::new(fs::read(path).unwrap());
    gl::ShaderSource(shader, 1, &src.as_ptr(), ptr::null());
    gl::CompileShader(shader);
    let mut success = gl::False;
    gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success);
    if success == gl::TRUE as GLint {
        return shader
    }
    // Panic with error log
    let mut log_len = 0;
    gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut log_len);
    log_len -= 1; // Don't store the null terminator
    let log = Vec::with_capacity(log_len as usize);
    gl::GetShaderInfoLog(shader, log_len, ptr::null_mut(), log.as_mut_ptr() as *mut GLchar);
    let log = str::from_utf8(&log).unwrap();
    panic!("Shader {} failed to compile: {}", path, log);
}

fn load_program(path_vert: &str, path_frag: &str) -> gl::GLuint {
    let vert_shader = load_shader(gl::VERTEX_SHADER, path_vert);
    let frag_shader = load_shader(gl::FRAGMENT_SHADER, path_frag);
    let prog = gl::CreateProgram();
    gl::AttachShader(prog, vert_shader);
    gl::AttachShader(prog, frag_shader);
    gl::LinkProgram(prog);
    // Check for linking errors
    let mut success = gl::FALSE as GLint;
    gl::GetProgramiv(prog, gl::LINK_STATUS, &mut success);
    if success == gl::TRUE as GLint {
        gl::DeleteShader(vert_shader);
        gl::DeleteShader(frag_shader);
        return prog;
    }
    // TODO: Panic with error log
    panic!();
}

fn init_shaders() {
    let prog = load_program("shaders/vert.glsl", "shaders/frag.glsl");
    let pos = gl::GetAttribLocation(prog, "position");
    gl::UseProgram(prog);
}

fn init_gfx() -> Graphics {
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

    init_shaders(); // TODO

    Graphics{window, event_pump, gl_ctx}
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
        x,
        y,
        z,
        w,
        x,
        y,
        z + 1,
        w,
        x,
        y + 1,
        z,
        w,
        x,
        y + 1,
        z,
        w,
        x,
        y,
        z + 1,
        w,
        x,
        y + 1,
        z + 1,
        w,
        // From +x
        x + 1,
        y,
        z,
        w,
        x + 1,
        y,
        z + 1,
        w,
        x + 1,
        y + 1,
        z,
        w,
        x + 1,
        y + 1,
        z,
        w,
        x + 1,
        y,
        z + 1,
        w,
        x + 1,
        y + 1,
        z + 1,
        w,
        // From -y
        x,
        y,
        z,
        w,
        x,
        y,
        z + 1,
        w,
        x + 1,
        y,
        z,
        w,
        x + 1,
        y,
        z,
        w,
        x,
        y,
        z + 1,
        w,
        x + 1,
        y,
        z + 1,
        w,
        // From +y
        x,
        y + 1,
        z,
        w,
        x,
        y + 1,
        z + 1,
        w,
        x + 1,
        y + 1,
        z,
        w,
        x + 1,
        y + 1,
        z,
        w,
        x,
        y + 1,
        z + 1,
        w,
        x + 1,
        y + 1,
        z + 1,
        w,
        // From -z
        x,
        y,
        z,
        w,
        x,
        y + 1,
        z,
        w,
        x + 1,
        y,
        z,
        w,
        x + 1,
        y,
        z,
        w,
        x,
        y + 1,
        z,
        w,
        x + 1,
        y + 1,
        z,
        w,
        // From +z
        x,
        y,
        z + 1,
        w,
        x,
        y + 1,
        z + 1,
        w,
        x + 1,
        y,
        z + 1,
        w,
        x + 1,
        y,
        z + 1,
        w,
        x,
        y + 1,
        z + 1,
        w,
        x + 1,
        y + 1,
        z + 1,
        w,
    ]
}

fn gen_voxels_vertex_buf(game_state: &GameState) -> [u8; VOXELS_VERTEX_BUF_SIZE] {
    let buf = [0; VOXELS_VERTEX_BUF_SIZE];
    let mut j = 0;
    for i in 0..L * W * H {
        let x = (i % L) as u8;
        let y = (i / L % W) as u8;
        let z = (i / L / W % H) as u8;
        let val = game_state.voxels[x][y][z];
        buf[j..VOXEL_VERTEX_BUF_SIZE].clone_from_slice(&gen_voxel_vertex_buf(x, y, z, val));
        j += VOXEL_VERTEX_BUF_SIZE;
    }
    buf
}

fn render(client: &Client) {
    let vertex_buf = gen_voxels_vertex_buf(&client.state);
    unsafe {
        gl::ClearColor(1.0, 1.0, 1.0, 1.0);
        gl::Clear(gl::COLOR_BUFFER_BIT);
    }
    client.gfx.window.gl_swap_window();
}

fn main() {
    let client = Client{
        gfx: init_gfx(),
        state: GameState::new(),
    };

    loop {
        handle_input(&mut client.gfx.event_pump);
        render(&client);
    }
}
