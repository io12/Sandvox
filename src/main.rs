#[macro_use]
extern crate glium;
extern crate cgmath;
extern crate clamp;
extern crate image;

use glium::framebuffer::SimpleFrameBuffer;
use glium::index::{NoIndices, PrimitiveType};
use glium::texture::{CubeLayer, Cubemap, RawImage2d};
use glium::uniforms::MagnifySamplerFilter;
use glium::{
    glutin, Depth, Display, DrawParameters, Frame, Program, Surface, Texture2d, VertexBuffer,
};

use glutin::dpi::LogicalSize;
use glutin::{
    ContextBuilder, DeviceEvent, ElementState, Event, EventsLoop, KeyboardInput, MouseButton,
    VirtualKeyCode, WindowBuilder, WindowEvent,
};

use cgmath::conv::array4x4;
use cgmath::prelude::*;
use cgmath::{ortho, perspective, Deg, Euler, Matrix4, Point3, Quaternion, Rad, Vector2, Vector3};

use clamp::clamp;

use image::RgbaImage;

use std::collections::HashMap;
use std::f32::consts::PI;
use std::time::{Duration, SystemTime};

type VoxInd = i8;

struct Graphics {
    display: Display,
    evs: EventsLoop,
    // GLSL shader programs
    voxel_prog: Program,
    line_prog: Program,
    sky_prog: Program,
}

struct Player {
    pos: Point3<f32>,
    angle: Vector2<f32>,
}

// A block directly in the player's line of sight
// TODO: This isn't needed because `pos` is unused now
#[derive(Copy, Clone)]
struct SightBlock {
    pos: Point3<VoxInd>,
    new_pos: Point3<VoxInd>, // Position of new block created from right-clicking
}

struct GameState {
    running: bool,
    paused: bool,
    frame: u32,
    player: Player,
    sight_block: Option<SightBlock>,
    voxels: Box<[[[bool; VOX_H]; VOX_W]; VOX_L]>,
    voxels_mesh: Vec<VoxelVertex>,
    dirty: bool,
    keys_down: HashMap<VirtualKeyCode, bool>,
    mouse_btns_down: HashMap<MouseButton, bool>,
}

struct Client {
    gfx: Graphics,
    state: GameState,
}

implement_vertex!(VoxelVertex, pos, color);
#[derive(Clone, Copy)]
struct VoxelVertex {
    pos: [VoxInd; 3],
    color: [VoxInd; 3],
}

implement_vertex!(WireframeVertex, pos);
#[derive(Clone, Copy)]
struct WireframeVertex {
    pos: [VoxInd; 3],
}

implement_vertex!(CrosshairsVertex, pos);
#[derive(Clone, Copy)]
struct CrosshairsVertex {
    pos: [f32; 3],
}

implement_vertex!(SkyboxVertex, pos);
#[derive(Clone, Copy)]
struct SkyboxVertex {
    pos: [f32; 3],
}

const VOX_L: usize = 120;
const VOX_W: usize = 120;
const VOX_H: usize = 120;
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
const BLOCK_SEL_DIST: usize = 200;
const RAYCAST_STEP: f32 = 0.1;
const SKYBOX_SIZE: f32 = 1.0;

impl VoxelVertex {
    fn new(pos: [VoxInd; 3], color: [VoxInd; 3]) -> Self {
        Self { pos, color }
    }
}
impl WireframeVertex {
    fn new(x: VoxInd, y: VoxInd, z: VoxInd) -> Self {
        Self { pos: [x, y, z] }
    }
}
impl CrosshairsVertex {
    fn new(x: f32, y: f32) -> Self {
        Self { pos: [x, y, 0.0] }
    }
}
impl SkyboxVertex {
    fn new(x: f32, y: f32, z: f32) -> Self {
        Self { pos: [x, y, z] }
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
        let voxel_prog = Program::from_source(
            &display,
            include_str!("shaders/voxel_vert.glsl"),
            include_str!("shaders/voxel_frag.glsl"),
            None,
        )
        .unwrap();
        let line_prog = Program::from_source(
            &display,
            include_str!("shaders/line_vert.glsl"),
            include_str!("shaders/line_frag.glsl"),
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

        let gfx = Graphics {
            display,
            evs,
            voxel_prog,
            line_prog,
            sky_prog,
        };
        let player = Player {
            pos: INIT_POS,
            angle: Vector2::new(0.0, 0.0),
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

fn handle_mouse_input(state: &mut GameState, mouse_state: ElementState, btn: MouseButton) {
    let down = mouse_state == ElementState::Pressed;
    state.mouse_btns_down.insert(btn, down);
}

fn handle_window_event(ev: &WindowEvent, state: &mut GameState) {
    match ev {
        WindowEvent::CloseRequested => state.running = false,
        WindowEvent::MouseInput {
            state: mouse_state,
            button,
            ..
        } => handle_mouse_input(state, *mouse_state, *button),
        _ => {}
    }
}

// Log whether a key was pressed/released such that `do_movement()` knows if keys are down
fn handle_keyboard_input(inp: &KeyboardInput, state: &mut GameState) {
    let down = inp.state == ElementState::Pressed;
    if let Some(key) = inp.virtual_keycode {
        state.keys_down.insert(key, down);
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
fn handle_event(ev: &Event, state: &mut GameState) {
    match ev {
        Event::WindowEvent { event: ev, .. } => handle_window_event(&ev, state),
        Event::DeviceEvent { event: ev, .. } => handle_device_event(&ev, state),
        _ => {}
    }
}

// TODO: Destructing can possibly be used here and in other places
fn do_input(evs: &mut EventsLoop, state: &mut GameState) {
    evs.poll_events(|ev| handle_event(&ev, state));
}

fn key_down(state: &GameState, key: VirtualKeyCode) -> bool {
    *state.keys_down.get(&key).unwrap_or(&false)
}

fn mouse_btn_down(state: &GameState, btn: MouseButton) -> bool {
    *state.mouse_btns_down.get(&btn).unwrap_or(&false)
}

// Process down keys to move the player
fn do_movement(client: &mut Client, dt: f32) {
    let (forward, right, _) = compute_dir_vectors(&client.state.player.angle);
    // Discard the y component to prevent the player from floating when they walk forward while
    // looking up. The vectors are normalized to keep the speed constant.
    let forward = Vector3::new(forward.x, 0.0, forward.z).normalize();
    let right = right.normalize();

    // Multiply by the time delta so speed of motion is constant (even if framerate isn't)

    // Move forward
    if key_down(&client.state, VirtualKeyCode::W) {
        client.state.player.pos += forward * dt * MOVE_SPEED
    }
    // Move backward
    if key_down(&client.state, VirtualKeyCode::R) {
        client.state.player.pos -= forward * dt * MOVE_SPEED
    }
    // Move left
    if key_down(&client.state, VirtualKeyCode::A) {
        client.state.player.pos -= right * dt * MOVE_SPEED
    }
    // Move right
    if key_down(&client.state, VirtualKeyCode::S) {
        client.state.player.pos += right * dt * MOVE_SPEED
    }
    // Move up
    if key_down(&client.state, VirtualKeyCode::Space) {
        client.state.player.pos.y += dt * MOVE_SPEED
    }
    // Move down
    if key_down(&client.state, VirtualKeyCode::LShift) {
        client.state.player.pos.y -= dt * MOVE_SPEED
    }

    // Pause game
    if key_down(&client.state, VirtualKeyCode::Escape) {
        set_pause(&mut client.state, &client.gfx.display, true);
    }

    // Destroy sand
    if mouse_btn_down(&client.state, MouseButton::Left) {
        if let Some(SightBlock { pos, .. }) = client.state.sight_block {
            put_voxel(&mut client.state, pos, false);
        }
    }

    // Create sand
    if mouse_btn_down(&client.state, MouseButton::Right) {
        if let Some(SightBlock { new_pos, .. }) = client.state.sight_block {
            put_voxel(&mut client.state, new_pos, true);
        }
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

// Handle state updates when paused
fn do_paused(client: &mut Client) {
    // Unpause
    if mouse_btn_down(&client.state, MouseButton::Left) {
        set_pause(&mut client.state, &client.gfx.display, false);
    }
}

// Update the game state for the current frame
// NB: This isn't the only place where the game state is modified
fn update_state(client: &mut Client, dt: f32) {
    if client.state.paused {
        do_paused(client);
    } else {
        do_movement(client, dt);
        do_sandfall(&mut client.state);
        client.state.sight_block = get_sight_block(&client.state);
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
fn compute_voxel_matrix(player: &Player, gfx: &Graphics) -> Matrix4<f32> {
    let (forward, _, up) = compute_dir_vectors(&player.angle);
    let aspect_ratio = get_aspect_ratio(gfx);
    let proj = perspective(FOV, aspect_ratio, 0.1, 1000.0);
    let view = Matrix4::look_at_dir(player.pos, forward, up);
    proj * view
}

// Make a mesh of the voxel world
fn make_voxels_mesh(state: &GameState) -> Vec<VoxelVertex> {
    // TODO: Make this mesh a global
    let cube_vertices = [
        VoxelVertex::new([0, 0, 0], [0, 0, 1]),
        VoxelVertex::new([0, 0, 1], [0, 1, 0]),
        VoxelVertex::new([0, 1, 1], [1, 0, 0]),
        VoxelVertex::new([1, 1, 0], [1, 0, 0]),
        VoxelVertex::new([0, 0, 0], [0, 1, 0]),
        VoxelVertex::new([0, 1, 0], [0, 0, 1]),
        VoxelVertex::new([1, 0, 1], [0, 1, 0]),
        VoxelVertex::new([0, 0, 0], [1, 0, 0]),
        VoxelVertex::new([1, 0, 0], [0, 1, 0]),
        VoxelVertex::new([1, 1, 0], [0, 0, 1]),
        VoxelVertex::new([1, 0, 0], [0, 1, 0]),
        VoxelVertex::new([0, 0, 0], [1, 0, 0]),
        VoxelVertex::new([0, 0, 0], [0, 1, 0]),
        VoxelVertex::new([0, 1, 1], [0, 0, 1]),
        VoxelVertex::new([0, 1, 0], [0, 1, 0]),
        VoxelVertex::new([1, 0, 1], [1, 0, 0]),
        VoxelVertex::new([0, 0, 1], [0, 1, 0]),
        VoxelVertex::new([0, 0, 0], [0, 0, 1]),
        VoxelVertex::new([0, 1, 1], [0, 1, 0]),
        VoxelVertex::new([0, 0, 1], [1, 0, 0]),
        VoxelVertex::new([1, 0, 1], [0, 1, 0]),
        VoxelVertex::new([1, 1, 1], [0, 0, 1]),
        VoxelVertex::new([1, 0, 0], [0, 1, 0]),
        VoxelVertex::new([1, 1, 0], [1, 0, 0]),
        VoxelVertex::new([1, 0, 0], [0, 1, 0]),
        VoxelVertex::new([1, 1, 1], [0, 0, 1]),
        VoxelVertex::new([1, 0, 1], [0, 1, 0]),
        VoxelVertex::new([1, 1, 1], [1, 0, 0]),
        VoxelVertex::new([1, 1, 0], [0, 1, 0]),
        VoxelVertex::new([0, 1, 0], [0, 0, 1]),
        VoxelVertex::new([1, 1, 1], [0, 1, 0]),
        VoxelVertex::new([0, 1, 0], [1, 0, 0]),
        VoxelVertex::new([0, 1, 1], [0, 1, 0]),
        VoxelVertex::new([1, 1, 1], [0, 0, 1]),
        VoxelVertex::new([0, 1, 1], [0, 1, 0]),
        VoxelVertex::new([1, 0, 1], [1, 0, 0]),
    ];

    let mut mesh = Vec::new();
    // Iterate through all the voxels, creating a cube mesh for each
    for x in 0..VOX_L {
        for y in 0..VOX_W {
            for z in 0..VOX_H {
                if state.voxels[x][y][z] {
                    for v in cube_vertices.iter() {
                        mesh.push(VoxelVertex::new(
                            [
                                v.pos[0] + x as VoxInd,
                                v.pos[1] + y as VoxInd,
                                v.pos[2] + z as VoxInd,
                            ],
                            v.color,
                        ));
                    }
                }
            }
        }
    }
    mesh
}

// Determine if the voxel at `pos` is a boundary (one voxel outside the voxel grid)
fn boundary_at_pos(pos: Point3<f32>) -> bool {
    pos.x as i32 == -1 || pos.y as i32 == -1 || pos.z as i32 == -1
}

// Determine if there is a voxel at `pos`, returning `None` when the position isn't within the
// bounds of the voxel grid
fn voxel_at_opt(state: &GameState, pos: Point3<f32>) -> Option<bool> {
    if pos.x >= 0.0 && pos.y >= 0.0 && pos.z >= 0.0 {
        Some(
            *state
                .voxels
                .get(pos.x as usize)?
                .get(pos.y as usize)?
                .get(pos.z as usize)?,
        )
    } else if boundary_at_pos(pos) {
        Some(true)
    } else {
        None
    }
}

// Determine if the is a voxel at `pos`, returning `false` when the position is out of bounds
fn voxel_at(state: &GameState, pos: Point3<f32>) -> bool {
    voxel_at_opt(state, pos).unwrap_or(false)
}

// Set a voxel at a coordinate, returning `None` if out-of-bounds
fn put_voxel(state: &mut GameState, pos: Point3<VoxInd>, val: bool) -> Option<()> {
    *state
        .voxels
        .get_mut(pos.x as usize)?
        .get_mut(pos.y as usize)?
        .get_mut(pos.z as usize)? = val;
    state.dirty = true;
    Some(())
}

// Get the block in the player's line of sight. This is the box that a wireframe is drawn around
// and is modified by left/right clicks. This function returns `None` if no voxel is in the
// player's line of sight.
fn get_sight_block(state: &GameState) -> Option<SightBlock> {
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
            let x = pos.x as VoxInd;
            let y = pos.y as VoxInd;
            let z = pos.z as VoxInd;
            let prev_x = prev_pos.x as VoxInd;
            let prev_y = prev_pos.y as VoxInd;
            let prev_z = prev_pos.z as VoxInd;
            let new_pos = if x > prev_x {
                Point3::new(x - 1, y, z)
            } else if x < prev_x {
                Point3::new(x + 1, y, z)
            } else if y > prev_y {
                Point3::new(x, y - 1, z)
            } else if y < prev_y {
                Point3::new(x, y + 1, z)
            } else if z > prev_z {
                Point3::new(x, y, z - 1)
            } else if z < prev_z {
                Point3::new(x, y, z + 1)
            } else {
                // All the previous vs current coords are equal; the player is inside a block
                Point3::new(x, y - 1, z)
            };
            return Some(SightBlock {
                pos: Point3::new(x, y, z),
                new_pos,
            });
        }
    }
    None
}

// Create a line wireframe mesh for the voxel in the player's line of sight. The return type is an
// `Option` because there might not be a voxel in the line of sight.
fn make_wireframe_mesh(state: &GameState) -> Option<[WireframeVertex; 48]> {
    let Point3 { x, y, z } = state.sight_block?.new_pos;
    // Array of lines (not triangles)
    Some([
        // From -x
        WireframeVertex::new(x, y, z),
        WireframeVertex::new(x, y + 1, z),
        WireframeVertex::new(x, y + 1, z),
        WireframeVertex::new(x, y + 1, z + 1),
        WireframeVertex::new(x, y + 1, z + 1),
        WireframeVertex::new(x, y, z + 1),
        WireframeVertex::new(x, y, z + 1),
        WireframeVertex::new(x, y, z),
        // From +x
        WireframeVertex::new(x + 1, y, z),
        WireframeVertex::new(x + 1, y + 1, z),
        WireframeVertex::new(x + 1, y + 1, z),
        WireframeVertex::new(x + 1, y + 1, z + 1),
        WireframeVertex::new(x + 1, y + 1, z + 1),
        WireframeVertex::new(x + 1, y, z + 1),
        WireframeVertex::new(x + 1, y, z + 1),
        WireframeVertex::new(x + 1, y, z),
        // From -y
        WireframeVertex::new(x, y, z),
        WireframeVertex::new(x + 1, y, z),
        WireframeVertex::new(x + 1, y, z),
        WireframeVertex::new(x + 1, y, z + 1),
        WireframeVertex::new(x + 1, y, z + 1),
        WireframeVertex::new(x, y, z + 1),
        WireframeVertex::new(x, y, z + 1),
        WireframeVertex::new(x, y, z),
        // From +y
        WireframeVertex::new(x, y + 1, z),
        WireframeVertex::new(x + 1, y + 1, z),
        WireframeVertex::new(x + 1, y + 1, z),
        WireframeVertex::new(x + 1, y + 1, z + 1),
        WireframeVertex::new(x + 1, y + 1, z + 1),
        WireframeVertex::new(x, y + 1, z + 1),
        WireframeVertex::new(x, y + 1, z + 1),
        WireframeVertex::new(x, y + 1, z),
        // From -z
        WireframeVertex::new(x, y, z),
        WireframeVertex::new(x + 1, y, z),
        WireframeVertex::new(x + 1, y, z),
        WireframeVertex::new(x + 1, y + 1, z),
        WireframeVertex::new(x + 1, y + 1, z),
        WireframeVertex::new(x, y + 1, z),
        WireframeVertex::new(x, y + 1, z),
        WireframeVertex::new(x, y, z),
        // From +z
        WireframeVertex::new(x, y, z + 1),
        WireframeVertex::new(x + 1, y, z + 1),
        WireframeVertex::new(x + 1, y, z + 1),
        WireframeVertex::new(x + 1, y + 1, z + 1),
        WireframeVertex::new(x + 1, y + 1, z + 1),
        WireframeVertex::new(x, y + 1, z + 1),
        WireframeVertex::new(x, y + 1, z + 1),
        WireframeVertex::new(x, y, z + 1),
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
        .draw(&vbuf, &ibuf, &gfx.voxel_prog, &uniforms, &params)
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
            .draw(&vbuf, &ibuf, &gfx.line_prog, &uniforms, &params)
            .unwrap();
    }
}

// Make a crosshairs mesh based on the window dimenions
fn make_crosshairs_mesh() -> [CrosshairsVertex; 4] {
    let sz = CROSSHAIRS_SIZE;
    [
        CrosshairsVertex::new(-sz, 0.0),
        CrosshairsVertex::new(sz, 0.0),
        CrosshairsVertex::new(0.0, -sz),
        CrosshairsVertex::new(0.0, sz),
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
fn render_crosshairs(gfx: &Graphics, matrix: Matrix4<f32>, target: &mut Frame) {
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
        .draw(&vbuf, &ibuf, &gfx.line_prog, &uniforms, &params)
        .unwrap();
}

fn make_skybox_mesh() -> [SkyboxVertex; 36] {
    let sz = SKYBOX_SIZE / 2.0;
    [
        // Front
        SkyboxVertex::new(-sz, -sz, sz), // 0
        SkyboxVertex::new(sz, sz, sz),   // 2
        SkyboxVertex::new(sz, -sz, sz),  // 1
        SkyboxVertex::new(-sz, -sz, sz), // 0
        SkyboxVertex::new(-sz, sz, sz),  // 3
        SkyboxVertex::new(sz, sz, sz),   // 2
        // Right
        SkyboxVertex::new(sz, -sz, sz),  // 4
        SkyboxVertex::new(sz, sz, -sz),  // 6
        SkyboxVertex::new(sz, -sz, -sz), // 5
        SkyboxVertex::new(sz, -sz, sz),  // 4
        SkyboxVertex::new(sz, sz, sz),   // 7
        SkyboxVertex::new(sz, sz, -sz),  // 6
        // Back
        SkyboxVertex::new(-sz, -sz, -sz), // 8
        SkyboxVertex::new(sz, sz, -sz),   // 10
        SkyboxVertex::new(-sz, sz, -sz),  // 9
        SkyboxVertex::new(-sz, -sz, -sz), // 8
        SkyboxVertex::new(sz, -sz, -sz),  // 11
        SkyboxVertex::new(sz, sz, -sz),   // 10
        // Left
        SkyboxVertex::new(-sz, -sz, sz),  // 12
        SkyboxVertex::new(-sz, sz, -sz),  // 14
        SkyboxVertex::new(-sz, sz, sz),   // 13
        SkyboxVertex::new(-sz, -sz, sz),  // 12
        SkyboxVertex::new(-sz, -sz, -sz), // 15
        SkyboxVertex::new(-sz, sz, -sz),  // 14
        // Bottom
        SkyboxVertex::new(-sz, -sz, sz),  // 16
        SkyboxVertex::new(sz, -sz, -sz),  // 18
        SkyboxVertex::new(-sz, -sz, -sz), // 17
        SkyboxVertex::new(-sz, -sz, sz),  // 16
        SkyboxVertex::new(sz, -sz, sz),   // 19
        SkyboxVertex::new(sz, -sz, -sz),  // 18
        // Top
        SkyboxVertex::new(-sz, sz, sz),  // 20
        SkyboxVertex::new(sz, sz, -sz),  // 22
        SkyboxVertex::new(sz, sz, sz),   // 21
        SkyboxVertex::new(-sz, sz, sz),  // 20
        SkyboxVertex::new(-sz, sz, -sz), // 23
        SkyboxVertex::new(sz, sz, -sz),  // 22
    ]
}

fn compute_skybox_matrix(player: &Player, gfx: &Graphics) -> Matrix4<f32> {
    let (forward, _, up) = compute_dir_vectors(&player.angle);
    let aspect_ratio = get_aspect_ratio(gfx);
    let proj = perspective(FOV, aspect_ratio, 0.1, 1000.0);
    let view = Matrix4::look_at_dir(Point3::new(0.0, 0.0, 0.0), forward, up);
    proj * view
}

fn make_skybox_cubemap(gfx: &Graphics, imgs: &[RgbaImage; 6]) -> Cubemap {
    let (w, h) = imgs[0].dimensions();
    let cubemap = Cubemap::empty(&gfx.display, w).unwrap();
    let imgs = imgs
        .iter()
        .map(|img| RawImage2d::from_raw_rgba_reversed(&img.clone().into_raw(), (w, h)));
    {
        let framebufs = [
            CubeLayer::PositiveX,
            CubeLayer::NegativeX,
            CubeLayer::PositiveY,
            CubeLayer::NegativeY,
            CubeLayer::PositiveZ,
            CubeLayer::NegativeZ,
        ]
        .iter()
        .map(|layer| {
            SimpleFrameBuffer::new(&gfx.display, cubemap.main_level().image(*layer)).unwrap()
        });
        let texture_positions = imgs.map(|img| Texture2d::new(&gfx.display, img).unwrap());
        for (tex_pos, framebuf) in texture_positions.zip(framebufs) {
            tex_pos.as_surface().blit_whole_color_to(
                &framebuf,
                &glium::BlitTarget {
                    left: 0,
                    bottom: 0,
                    width: w as i32,
                    height: h as i32,
                },
                MagnifySamplerFilter::Linear,
            );
        }
    }
    cubemap
}

// Load an image from a byte array. Also xy-flip the texture for OpenGL.
fn image_from_bytes(bytes: &'static [u8], flip: bool) -> RgbaImage {
    let img = image::load(std::io::Cursor::new(bytes), image::JPEG).unwrap();
    if flip { img.flipv().fliph() } else { img }.to_rgba()
}

fn render_skybox(gfx: &Graphics, matrix: Matrix4<f32>, target: &mut Frame) {
    let mesh = make_skybox_mesh();
    let vbuf = VertexBuffer::new(&gfx.display, &mesh).unwrap();
    // Do not use an index buffer
    let ibuf = NoIndices(PrimitiveType::TrianglesList);

    let cubemap = make_skybox_cubemap(
        gfx,
        &[
            image_from_bytes(include_bytes!("../assets/isle_ft.jpg"), true),
            image_from_bytes(include_bytes!("../assets/isle_bk.jpg"), true),
            image_from_bytes(include_bytes!("../assets/isle_up.jpg"), false),
            image_from_bytes(include_bytes!("../assets/isle_dn.jpg"), false),
            image_from_bytes(include_bytes!("../assets/isle_lf.jpg"), true),
            image_from_bytes(include_bytes!("../assets/isle_rt.jpg"), true),
        ],
    );

    let uniforms = uniform! {
        matrix: array4x4(matrix),
        cubemap: cubemap.sampled().magnify_filter(MagnifySamplerFilter::Linear),
    };

    let params = DrawParameters {
        // TODO: backface_culling: glium::draw_parameters::BackfaceCullingMode::CullClockwise,
        ..Default::default()
    };
    target
        .draw(&vbuf, &ibuf, &gfx.sky_prog, &uniforms, &params)
        .unwrap();
}

// Create meshes for the game objects and render them with OpenGL
fn render(gfx: &mut Graphics, state: &mut GameState) {
    let vox_matrix = compute_voxel_matrix(&state.player, gfx);
    let matrix_2d = compute_2d_matrix(gfx);
    let skybox_matrix = compute_skybox_matrix(&state.player, gfx);

    let mut target = gfx.display.draw();
    // Initialize rendering
    target.clear_color_and_depth((0.0, 0.0, 0.0, 1.0), 1.0);

    // Render each component
    render_skybox(gfx, skybox_matrix, &mut target);
    render_voxels(gfx, state, vox_matrix, &mut target);
    render_wireframe(gfx, state, vox_matrix, &mut target);
    render_crosshairs(gfx, matrix_2d, &mut target);

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
        do_input(&mut client.gfx.evs, &mut client.state);
        update_state(&mut client, dt);
        render(&mut client.gfx, &mut client.state);
        client.state.frame += 1;
    }
}
