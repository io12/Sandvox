use glium::texture::srgb_cubemap::SrgbCubemap;
use glium::texture::Texture2d;
use glium::{Display, Program};

use glium::glutin::dpi::LogicalSize;
use glium::glutin::{ContextBuilder, EventsLoop, MouseButton, VirtualKeyCode, WindowBuilder};

use conrod_core::text::Font;

use cgmath::{Point3, Vector2, Vector3};

use nd_iter::iter_3d;

use rand::prelude::*;
use rand_xorshift::XorShiftRng;

use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use render::{VoxInd, VoxelVertex};
use {input, physics, render};

pub struct Ui {
    pub ui: conrod_core::Ui,
    pub image_map: conrod_core::image::Map<Texture2d>,
    pub renderer: conrod_glium::Renderer,
}

pub struct Graphics {
    pub display: Display,
    pub cubemap: SrgbCubemap,
    // GLSL shader programs
    pub basic_prog: Program,
    pub sky_prog: Program,
    pub voxel_prog: Program,
    pub ui: Ui,
}

#[derive(Copy, Clone, PartialEq)]
pub enum PlayerState {
    Normal,
    Running,
    Flying,
}

pub struct Player {
    pub pos: Point3<f32>, // Position of the player's eyes
    pub angle: Vector2<f32>,
    pub velocity: Vector3<f32>,
    pub state: PlayerState,
}

// A block directly in the player's line of sight
#[derive(Copy, Clone)]
pub struct SightBlock {
    pub pos: Point3<VoxInd>,
    pub new_pos: Point3<VoxInd>, // Position of new block created from right-clicking
}

#[derive(Clone, Copy)]
pub enum Voxel {
    Air,
    Boundary,
    Sand(VoxelShade),
}

pub type VoxelGrid = Box<[[[Voxel; VOX_MAX_Z]; VOX_MAX_Y]; VOX_MAX_X]>;
pub type VoxelShade = u8;

pub struct GameTimers {
    // TODO: Maybe don't use SystemTime?
    pub run_press_timer: Option<SystemTime>, // Time since foward press to track double presses for running
    pub since_run_timer: Option<SystemTime>, // Time since start/stop running, for FOV fading
}

pub struct GameState {
    pub running: bool,
    pub paused: bool,
    pub frame: u32,
    pub player: Player,
    pub sight_block: Option<SightBlock>,
    pub voxels: VoxelGrid,
    pub voxels_mesh: Vec<VoxelVertex>,
    pub dirty: bool,
    pub keys_down: HashMap<VirtualKeyCode, bool>,
    pub mouse_btns_down: HashMap<MouseButton, bool>,
    pub rng: XorShiftRng,
    pub timers: GameTimers,
}

pub struct Client {
    pub evs: EventsLoop,
    pub gfx: Graphics,
    pub state: GameState,
}

pub const VOX_MAX_X: usize = 50;
pub const VOX_MAX_Y: usize = 50;
pub const VOX_MAX_Z: usize = 50;

const GAME_NAME: &str = "Sandvox";
const WIN_W: u32 = 800;
const WIN_H: u32 = 600;
const INIT_POS: Point3<f32> = Point3 {
    x: 0.0,
    y: 1.5, // TODO: Each voxel is 1 cm and the camera is 1.5 m above ground
    z: 0.0,
};

impl Ui {
    fn init(win_size: LogicalSize, display: &Display) -> Self {
        let mut ui = conrod_core::UiBuilder::new([win_size.width, win_size.height]).build();
        let font_bytes: &[u8] = include_bytes!("../assets/font/EBGaramond-Medium.ttf");
        ui.fonts.insert(Font::from_bytes(font_bytes).unwrap());
        Ui {
            ui,
            image_map: conrod_core::image::Map::new(),
            renderer: conrod_glium::Renderer::new(display).unwrap(),
        }
    }
}

impl Graphics {
    // Create a window, initialize OpenGL, and compile the GLSL shaders
    fn init(evs: &EventsLoop) -> Self {
        let win_size = (WIN_W, WIN_H).into();
        let win = WindowBuilder::new()
            .with_dimensions(win_size)
            .with_title(GAME_NAME);
        let ctx = ContextBuilder::new().with_depth_buffer(24);
        let display = Display::new(win, ctx, evs).unwrap();
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
        let ui = Ui::init(win_size, &display);

        Graphics {
            display,
            cubemap,
            basic_prog,
            sky_prog,
            voxel_prog,
            ui,
        }
    }
}

impl GameTimers {
    // Initialize the game timers
    fn init() -> Self {
        GameTimers {
            run_press_timer: None,
            since_run_timer: None,
        }
    }
}

impl Voxel {
    pub fn is_air(&self) -> bool {
        match *self {
            Voxel::Air => true,
            _ => false,
        }
    }
}

impl GameState {
    // Initialize the game state object
    fn init() -> Self {
        let mut rng = SeedableRng::seed_from_u64(0);
        GameState {
            running: true,
            paused: true,
            frame: 0,
            player: Player {
                pos: INIT_POS,
                angle: Vector2::new(0.0, 0.0),
                velocity: Vector3::new(0.0, 0.0, 0.0),
                state: PlayerState::Normal,
            },
            sight_block: None,
            voxels: make_test_world(&mut rng),
            voxels_mesh: Vec::new(),
            dirty: true,
            keys_down: HashMap::new(),
            mouse_btns_down: HashMap::new(),
            rng,
            timers: GameTimers::init(),
        }
    }
}

impl Client {
    // Initialize the game client (event loop, window creation, OpenGL, game state)
    pub fn init() -> Self {
        let evs = EventsLoop::new();
        let gfx = Graphics::init(&evs);
        let state = GameState::init();
        Client { evs, gfx, state }
    }
}

// Create an initial diagonal stripe test world
// TODO: Remove this
fn make_test_world<R: Rng>(rng: &mut R) -> VoxelGrid {
    let mut voxels = Box::new([[[Voxel::Air; VOX_MAX_Z]; VOX_MAX_Y]; VOX_MAX_X]);
    for (x, y, z) in iter_3d(0..VOX_MAX_X, 0..VOX_MAX_Y, 0..VOX_MAX_Z) {
        if x == y && y == z {
            // TODO: Use random instead of coord cast
            voxels[x][y][z] = Voxel::Sand(rng.gen());
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

// Get the time since `prev_time` in seconds
pub fn get_time_delta(prev_time: &SystemTime) -> f32 {
    let elapsed = prev_time.elapsed().unwrap_or_else(|_| Duration::new(0, 0));
    elapsed.as_secs() as f32 + elapsed.subsec_millis() as f32 / 1000.0
}
