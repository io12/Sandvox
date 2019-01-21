use glium::framebuffer::SimpleFrameBuffer;
use glium::glutin::dpi::LogicalSize;
use glium::index::{NoIndices, PrimitiveType};
use glium::texture::srgb_cubemap::SrgbCubemap;
use glium::texture::{CubeLayer, RawImage2d};
use glium::uniforms::MagnifySamplerFilter;
use glium::{Blend, Depth, Display, DrawParameters, Frame, Surface, Texture2d, VertexBuffer};

use conrod_core::{Colorable, Positionable, Widget};

use cgmath::conv::array4x4;
use cgmath::prelude::*;
use cgmath::{ortho, perspective, Deg, Matrix4, Point3};

use image::RgbaImage;

use nd_iter::iter_3d;

use client::{GameState, Graphics, Player, SightBlock, VoxelType, VOX_MAX_X, VOX_MAX_Y, VOX_MAX_Z};
use physics;

pub type VoxInd = i8;

implement_vertex!(VoxelVertex, pos, voxel_type);
#[derive(Clone, Copy)]
pub struct VoxelVertex {
    pos: [VoxInd; 3],
    voxel_type: u8,
}

implement_vertex!(BasicVertexI, pos, color);
#[derive(Clone, Copy)]
pub struct BasicVertexI {
    pos: [VoxInd; 3],
    color: [VoxInd; 4],
}

implement_vertex!(BasicVertexF, pos, color);
#[derive(Clone, Copy)]
struct BasicVertexF {
    pos: [f32; 3],
    color: [f32; 4],
}

implement_vertex!(SkyboxVertex, pos);
#[derive(Clone, Copy)]
struct SkyboxVertex {
    pos: [f32; 3],
}

impl VoxelVertex {
    fn new(pos: [VoxInd; 3], voxel_type: VoxelType) -> Self {
        Self {
            pos,
            voxel_type: voxel_type as u8,
        }
    }
}
impl BasicVertexI {
    fn new(pos: [VoxInd; 3], color: [VoxInd; 4]) -> Self {
        Self { pos, color }
    }
}
impl BasicVertexF {
    fn new(pos: [f32; 3], color: [f32; 4]) -> Self {
        Self { pos, color }
    }
}
impl SkyboxVertex {
    fn new(x: f32, y: f32, z: f32) -> Self {
        Self { pos: [x, y, z] }
    }
}

const FOV: Deg<f32> = Deg(60.0);
const BLOCK_SEL_DIST: usize = 200;
const RAYCAST_STEP: f32 = 0.1;
const SKYBOX_SIZE: f32 = 1.0;
const CROSSHAIRS_SIZE: f32 = 15.0;
const PAUSE_SCREEN_DIM: f32 = 0.9; // The amount of screen dimming when paused
                                   // 1.0 is full black, 0.0 is no dimming

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
    let (forward, _, up) = physics::compute_dir_vectors(player.angle);
    let aspect_ratio = get_aspect_ratio(gfx);
    let proj = perspective(FOV, aspect_ratio, 0.1, 1000.0);
    let view = Matrix4::look_at_dir(player.pos, forward, up);
    proj * view
}

// Make a mesh of the voxel world
fn make_voxels_mesh(state: &GameState) -> Vec<VoxelVertex> {
    // TODO: Make this mesh a global
    // TODO: Change this to not be `BasicVertexI`
    let cube_vertices = [
        // From -x
        BasicVertexI::new([0, 0, 0], [0, 0, 1, 1]),
        BasicVertexI::new([0, 0, 1], [0, 1, 0, 1]),
        BasicVertexI::new([0, 1, 1], [1, 0, 0, 1]),
        // From -z
        BasicVertexI::new([1, 1, 0], [1, 0, 0, 1]),
        BasicVertexI::new([0, 0, 0], [0, 1, 0, 1]),
        BasicVertexI::new([0, 1, 0], [0, 0, 1, 1]),
        // From -y
        BasicVertexI::new([1, 0, 1], [0, 1, 0, 1]),
        BasicVertexI::new([0, 0, 0], [1, 0, 0, 1]),
        BasicVertexI::new([1, 0, 0], [0, 1, 0, 1]),
        // From -z
        BasicVertexI::new([1, 1, 0], [0, 0, 1, 1]),
        BasicVertexI::new([1, 0, 0], [0, 1, 0, 1]),
        // From -x
        BasicVertexI::new([0, 0, 0], [1, 0, 0, 1]),
        BasicVertexI::new([0, 0, 0], [0, 1, 0, 1]),
        BasicVertexI::new([0, 1, 1], [0, 0, 1, 1]),
        BasicVertexI::new([0, 1, 0], [0, 1, 0, 1]),
        BasicVertexI::new([1, 0, 1], [1, 0, 0, 1]),
        BasicVertexI::new([0, 0, 1], [0, 1, 0, 1]),
        BasicVertexI::new([0, 0, 0], [0, 0, 1, 1]),
        BasicVertexI::new([0, 1, 1], [0, 1, 0, 1]),
        BasicVertexI::new([0, 0, 1], [1, 0, 0, 1]),
        BasicVertexI::new([1, 0, 1], [0, 1, 0, 1]),
        BasicVertexI::new([1, 1, 1], [0, 0, 1, 1]),
        BasicVertexI::new([1, 0, 0], [0, 1, 0, 1]),
        BasicVertexI::new([1, 1, 0], [1, 0, 0, 1]),
        BasicVertexI::new([1, 0, 0], [0, 1, 0, 1]),
        BasicVertexI::new([1, 1, 1], [0, 0, 1, 1]),
        BasicVertexI::new([1, 0, 1], [0, 1, 0, 1]),
        BasicVertexI::new([1, 1, 1], [1, 0, 0, 1]),
        BasicVertexI::new([1, 1, 0], [0, 1, 0, 1]),
        BasicVertexI::new([0, 1, 0], [0, 0, 1, 1]),
        BasicVertexI::new([1, 1, 1], [0, 1, 0, 1]),
        BasicVertexI::new([0, 1, 0], [1, 0, 0, 1]),
        BasicVertexI::new([0, 1, 1], [0, 1, 0, 1]),
        BasicVertexI::new([1, 1, 1], [0, 0, 1, 1]),
        BasicVertexI::new([0, 1, 1], [0, 1, 0, 1]),
        BasicVertexI::new([1, 0, 1], [1, 0, 0, 1]),
    ];

    let mut mesh = Vec::new();
    // Iterate through all the voxels, creating a cube mesh for each
    for (x, y, z) in iter_3d(0..VOX_MAX_X, 0..VOX_MAX_Y, 0..VOX_MAX_Z) {
        let voxel_type = state.voxels[x][y][z];
        if voxel_type != VoxelType::Air {
            for v in cube_vertices.iter() {
                mesh.push(VoxelVertex::new(
                    [
                        v.pos[0] + x as VoxInd,
                        v.pos[1] + y as VoxInd,
                        v.pos[2] + z as VoxInd,
                    ],
                    voxel_type,
                ));
            }
        }
    }
    mesh
}

// Get the block in the player's line of sight. This is the box that a wireframe is drawn around
// and is modified by left/right clicks. This function returns `None` if no voxel is in the
// player's line of sight.
pub fn get_sight_block(state: &GameState) -> Option<SightBlock> {
    let forward = physics::compute_forward_vector(state.player.angle);
    let mut pos = state.player.pos;
    // Raycasting
    for _ in 0..BLOCK_SEL_DIST {
        let prev_pos = pos;
        pos += forward * RAYCAST_STEP;
        if physics::voxel_at(state, pos) {
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
fn make_wireframe_mesh(state: &GameState) -> Option<[BasicVertexI; 48]> {
    let Point3 { x, y, z } = state.sight_block?.pos;
    let color = [1, 1, 1, 1];
    // Array of lines (not triangles)
    Some([
        // From -x
        BasicVertexI::new([x, y, z], color),
        BasicVertexI::new([x, y + 1, z], color),
        BasicVertexI::new([x, y + 1, z], color),
        BasicVertexI::new([x, y + 1, z + 1], color),
        BasicVertexI::new([x, y + 1, z + 1], color),
        BasicVertexI::new([x, y, z + 1], color),
        BasicVertexI::new([x, y, z + 1], color),
        BasicVertexI::new([x, y, z], color),
        // From +x
        BasicVertexI::new([x + 1, y, z], color),
        BasicVertexI::new([x + 1, y + 1, z], color),
        BasicVertexI::new([x + 1, y + 1, z], color),
        BasicVertexI::new([x + 1, y + 1, z + 1], color),
        BasicVertexI::new([x + 1, y + 1, z + 1], color),
        BasicVertexI::new([x + 1, y, z + 1], color),
        BasicVertexI::new([x + 1, y, z + 1], color),
        BasicVertexI::new([x + 1, y, z], color),
        // From -y
        BasicVertexI::new([x, y, z], color),
        BasicVertexI::new([x + 1, y, z], color),
        BasicVertexI::new([x + 1, y, z], color),
        BasicVertexI::new([x + 1, y, z + 1], color),
        BasicVertexI::new([x + 1, y, z + 1], color),
        BasicVertexI::new([x, y, z + 1], color),
        BasicVertexI::new([x, y, z + 1], color),
        BasicVertexI::new([x, y, z], color),
        // From +y
        BasicVertexI::new([x, y + 1, z], color),
        BasicVertexI::new([x + 1, y + 1, z], color),
        BasicVertexI::new([x + 1, y + 1, z], color),
        BasicVertexI::new([x + 1, y + 1, z + 1], color),
        BasicVertexI::new([x + 1, y + 1, z + 1], color),
        BasicVertexI::new([x, y + 1, z + 1], color),
        BasicVertexI::new([x, y + 1, z + 1], color),
        BasicVertexI::new([x, y + 1, z], color),
        // From -z
        BasicVertexI::new([x, y, z], color),
        BasicVertexI::new([x + 1, y, z], color),
        BasicVertexI::new([x + 1, y, z], color),
        BasicVertexI::new([x + 1, y + 1, z], color),
        BasicVertexI::new([x + 1, y + 1, z], color),
        BasicVertexI::new([x, y + 1, z], color),
        BasicVertexI::new([x, y + 1, z], color),
        BasicVertexI::new([x, y, z], color),
        // From +z
        BasicVertexI::new([x, y, z + 1], color),
        BasicVertexI::new([x + 1, y, z + 1], color),
        BasicVertexI::new([x + 1, y, z + 1], color),
        BasicVertexI::new([x + 1, y + 1, z + 1], color),
        BasicVertexI::new([x + 1, y + 1, z + 1], color),
        BasicVertexI::new([x, y + 1, z + 1], color),
        BasicVertexI::new([x, y + 1, z + 1], color),
        BasicVertexI::new([x, y, z + 1], color),
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
            .draw(&vbuf, &ibuf, &gfx.basic_prog, &uniforms, &params)
            .unwrap();
    }
}

// Make a crosshairs mesh based on the window dimenions
fn make_crosshairs_mesh() -> [BasicVertexF; 4] {
    let sz = CROSSHAIRS_SIZE;
    let color = [1.0, 1.0, 1.0, 1.0];
    [
        BasicVertexF::new([-sz, 0.0, 0.0], color),
        BasicVertexF::new([sz, 0.0, 0.0], color),
        BasicVertexF::new([0.0, -sz, 0.0], color),
        BasicVertexF::new([0.0, sz, 0.0], color),
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
        .draw(&vbuf, &ibuf, &gfx.basic_prog, &uniforms, &params)
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
    let (forward, _, up) = physics::compute_dir_vectors(player.angle);
    let aspect_ratio = get_aspect_ratio(gfx);
    let proj = perspective(FOV, aspect_ratio, 0.1, 1000.0);
    let view = Matrix4::look_at_dir(Point3::new(0.0, 0.0, 0.0), forward, up);
    proj * view
}

fn make_skybox_cubemap_with_images(display: &Display, imgs: &[RgbaImage; 6]) -> SrgbCubemap {
    let (w, h) = imgs[0].dimensions();
    let cubemap = SrgbCubemap::empty(display, w).unwrap();
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
        .map(|layer| SimpleFrameBuffer::new(display, cubemap.main_level().image(*layer)).unwrap());
        let texture_positions = imgs.map(|img| Texture2d::new(display, img).unwrap());
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

pub fn make_skybox_cubemap(display: &Display) -> SrgbCubemap {
    make_skybox_cubemap_with_images(
        display,
        &[
            image_from_bytes(include_bytes!("../assets/isle_ft.jpg"), true),
            image_from_bytes(include_bytes!("../assets/isle_bk.jpg"), true),
            image_from_bytes(include_bytes!("../assets/isle_up.jpg"), false),
            image_from_bytes(include_bytes!("../assets/isle_dn.jpg"), false),
            image_from_bytes(include_bytes!("../assets/isle_lf.jpg"), true),
            image_from_bytes(include_bytes!("../assets/isle_rt.jpg"), true),
        ],
    )
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

    let uniforms = uniform! {
        matrix: array4x4(matrix),
        cubemap: gfx.cubemap.sampled().magnify_filter(MagnifySamplerFilter::Linear),
    };

    let params = DrawParameters {
        backface_culling: glium::draw_parameters::BackfaceCullingMode::CullClockwise,
        ..Default::default()
    };
    target
        .draw(&vbuf, &ibuf, &gfx.sky_prog, &uniforms, &params)
        .unwrap();
}

// Create a translucent black rectangle to dim the screen
fn make_screen_dimmer_mesh() -> [BasicVertexF; 6] {
    let color = [0.0, 0.0, 0.0, PAUSE_SCREEN_DIM];
    let sz = 1.0;
    [
        BasicVertexF::new([-sz, -sz, 0.0], color),
        BasicVertexF::new([-sz, sz, 0.0], color),
        BasicVertexF::new([sz, -sz, 0.0], color),
        BasicVertexF::new([sz, -sz, 0.0], color),
        BasicVertexF::new([-sz, sz, 0.0], color),
        BasicVertexF::new([sz, sz, 0.0], color),
    ]
}

// Dim the screen by rendering a translucent black rectangle
fn render_screen_dimmer(gfx: &Graphics, target: &mut Frame) {
    let mesh = make_screen_dimmer_mesh();
    let vbuf = VertexBuffer::new(&gfx.display, &mesh).unwrap();
    // Do not use an index buffer
    let ibuf = NoIndices(PrimitiveType::TrianglesList);
    let matrix: Matrix4<f32> = Matrix4::identity();
    let uniforms = uniform! {
        matrix: array4x4(matrix),
    };
    let params = DrawParameters {
        blend: Blend::alpha_blending(),
        ..Default::default()
    };
    target
        .draw(&vbuf, &ibuf, &gfx.basic_prog, &uniforms, &params)
        .unwrap();
}

// TODO: Handle screen resizing
fn render_pause_screen(gfx: &mut Graphics, target: &mut Frame) {
    render_screen_dimmer(gfx, target);
    // Generate the widget identifiers.
    widget_ids!(struct Ids { text });
    let ids = Ids::new(gfx.ui.ui.widget_id_generator());
    let ui = &mut gfx.ui.ui.set_widgets();
    conrod_core::widget::Text::new("Paused")
        .middle_of(ui.window)
        .color(conrod_core::color::WHITE)
        .font_size(32)
        .set(ids.text, ui);
    if let Some(primitives) = ui.draw_if_changed() {
        gfx.ui
            .renderer
            .fill(&gfx.display, primitives, &gfx.ui.image_map);
        gfx.ui
            .renderer
            .draw(&gfx.display, target, &gfx.ui.image_map)
            .unwrap();
    }
}

// Create meshes for the game objects and render them with OpenGL
pub fn render(gfx: &mut Graphics, state: &mut GameState) {
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
    if state.paused {
        render_pause_screen(gfx, &mut target);
    }

    // Swap buffers to finalize rendering
    target.finish().unwrap();
}
