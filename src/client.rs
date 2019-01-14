use glium::texture::srgb_cubemap::SrgbCubemap;
use glium::{Display, Program};

use glium::glutin::{ContextBuilder, EventsLoop, MouseButton, VirtualKeyCode, WindowBuilder};

use cgmath::{Point3, Vector2, Vector3};

use std::collections::HashMap;

use render::{VoxInd, VoxelVertex};
use {input, physics, render};

pub struct Graphics {
    pub display: Display,
    pub evs: EventsLoop,
    pub cubemap: SrgbCubemap,
    // GLSL shader programs
    pub basic_prog: Program,
    pub sky_prog: Program,
    pub voxel_prog: Program,
}

pub struct Player {
    pub pos: Point3<f32>, // Position of the player's eyes
    pub angle: Vector2<f32>,
    pub velocity: Vector3<f32>,
    pub flying: bool,
}

// A block directly in the player's line of sight
#[derive(Copy, Clone)]
pub struct SightBlock {
    pub pos: Point3<VoxInd>,
    pub new_pos: Point3<VoxInd>, // Position of new block created from right-clicking
}

#[derive(Copy, Clone, PartialEq)]
pub enum VoxelType {
    Air,
    Sand,
    Boundary,
}

pub struct GameState {
    pub running: bool,
    pub paused: bool,
    pub frame: u32,
    pub player: Player,
    pub sight_block: Option<SightBlock>,
    pub voxels: Box<[[[VoxelType; VOX_MAX_Z]; VOX_MAX_Y]; VOX_MAX_X]>,
    pub voxels_mesh: Vec<VoxelVertex>,
    pub dirty: bool,
    pub keys_down: HashMap<VirtualKeyCode, bool>,
    pub mouse_btns_down: HashMap<MouseButton, bool>,
}

pub struct Client {
    pub gfx: Graphics,
    pub state: GameState,
}

pub const VOX_MAX_X: usize = 120;
pub const VOX_MAX_Y: usize = 120;
pub const VOX_MAX_Z: usize = 120;

const GAME_NAME: &str = "Sandvox";
const WIN_W: u32 = 800;
const WIN_H: u32 = 600;
const INIT_POS: Point3<f32> = Point3 {
    x: 0.0,
    y: 1.5, // TODO: Each voxel is 1 cm and the camera is 1.5 m above ground
    z: 0.0,
};

impl Client {
    // Create a window, initialize OpenGL, compile the GLSL shaders, and initialize a client struct
    pub fn init() -> Client {
        let win_size = (WIN_W, WIN_H).into();
        let win = WindowBuilder::new()
            .with_dimensions(win_size)
            .with_title(GAME_NAME);
        let ctx = ContextBuilder::new().with_depth_buffer(24);
        let evs = EventsLoop::new();
        let display = Display::new(win, ctx, &evs).unwrap();
        let cubemap = render::make_skybox_cubemap(&display);
        // Compile program from GLSL shaders
        let basic_prog = Program::from_source(
            &display,
            include_str!("shaders/basic_vert.glsl"),
            include_str!("shaders/basic_frag.glsl"),
            None,
        )
        .unwrap();
        let sky_prog = Program::from_source(
            &display,
            include_str!("shaders/sky_vert.glsl"),
            include_str!("shaders/sky_frag.glsl"),
            None,
        )
        .unwrap();
        let voxel_prog = Program::from_source(
            &display,
            include_str!("shaders/voxel_vert.glsl"),
            include_str!("shaders/voxel_frag.glsl"),
            None,
        )
        .unwrap();

        let gfx = Graphics {
            display,
            evs,
            cubemap,
            basic_prog,
            sky_prog,
            voxel_prog,
        };
        let player = Player {
            pos: INIT_POS,
            angle: Vector2::new(0.0, 0.0),
            velocity: Vector3::new(0.0, 0.0, 0.0),
            flying: true,
        };
        let state = GameState {
            running: true,
            paused: true,
            frame: 0,
            player,
            sight_block: None,
            voxels: make_test_world(),
            voxels_mesh: Vec::new(),
            dirty: true,
            keys_down: HashMap::new(),
            mouse_btns_down: HashMap::new(),
        };
        Client { gfx, state }
    }
}

// Create an initial diagonal stripe test world
// TODO: Remove this
fn make_test_world() -> Box<[[[VoxelType; VOX_MAX_Z]; VOX_MAX_Y]; VOX_MAX_X]> {
    let mut voxels = Box::new([[[VoxelType::Air; VOX_MAX_Z]; VOX_MAX_Y]; VOX_MAX_X]);
    for x in 0..VOX_MAX_X {
        for y in 0..VOX_MAX_Y {
            for z in 0..VOX_MAX_Z {
                if x == y && y == z {
                    voxels[x][y][z] = VoxelType::Sand;
                }
            }
        }
    }
    voxels
}

// Pause/unpause the game
pub fn set_pause(state: &mut GameState, display: &Display, paused: bool) {
    let grab = !paused;
    display.gl_window().window().grab_cursor(grab).unwrap();
    display.gl_window().window().hide_cursor(grab);
    state.paused = paused;
}

// Handle state updates when paused
fn do_paused(client: &mut Client) {
    // Unpause
    if input::mouse_btn_down(&client.state, MouseButton::Left) {
        set_pause(&mut client.state, &client.gfx.display, false);
    }
}

// Update the game state for the current frame
// NB: This isn't the only place where the game state is modified
pub fn update(client: &mut Client, dt: f32) {
    if client.state.paused {
        do_paused(client);
    } else {
        input::do_keys_down(client);
        physics::do_player_physics(&mut client.state.player, dt);
        physics::do_sandfall(&mut client.state);
        client.state.sight_block = render::get_sight_block(&client.state);
    }
}
