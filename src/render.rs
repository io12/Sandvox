use glium::framebuffer::SimpleFrameBuffer;
use glium::index::{NoIndices, PrimitiveType};
use glium::texture::srgb_cubemap::SrgbCubemap;
use glium::texture::{CubeLayer, RawImage2d};
use glium::uniforms::MagnifySamplerFilter;
use glium::{Depth, Display, DrawParameters, Frame, Surface, Texture2d, VertexBuffer};

use glium::glutin::dpi::LogicalSize;

use cgmath::conv::array4x4;
use cgmath::prelude::*;
use cgmath::{ortho, perspective, Deg, Euler, Matrix4, Point3, Quaternion, Rad, Vector2, Vector3};

use image::RgbaImage;

use client::{GameState, Graphics, Player, SightBlock, VOX_H, VOX_L, VOX_W};

pub type VoxInd = i8;

implement_vertex!(VoxelVertex, pos, color);
#[derive(Clone, Copy)]
pub struct VoxelVertex {
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

const FOV: Deg<f32> = Deg(60.0);
const BLOCK_SEL_DIST: usize = 200;
const RAYCAST_STEP: f32 = 0.1;
const SKYBOX_SIZE: f32 = 1.0;
const CROSSHAIRS_SIZE: f32 = 15.0;
const EYE_HEIGHT: f32 = 1.62;

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

// Get the position of the player's eyes
fn get_eye_pos(player: &Player) -> Point3<f32> {
    player.pos + Vector3::new(0.0, EYE_HEIGHT, 0.0)
}

// Compute the transformation matrix. Each vertex is multiplied by the matrix so it renders in the
// correct position relative to the player.
fn compute_voxel_matrix(player: &Player, gfx: &Graphics) -> Matrix4<f32> {
    let (forward, _, up) = compute_dir_vectors(&player.angle);
    let aspect_ratio = get_aspect_ratio(gfx);
    let proj = perspective(FOV, aspect_ratio, 0.1, 1000.0);
    let eye = get_eye_pos(player);
    let view = Matrix4::look_at_dir(eye, forward, up);
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
    pos.x as i32 == -1
        || pos.y as i32 == -1
        || pos.z as i32 == -1
        || pos.x as usize == VOX_L
        || pos.y as usize == VOX_W
        || pos.z as usize == VOX_H
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
// TODO: Move this to `client.rs`
pub fn put_voxel(state: &mut GameState, pos: Point3<VoxInd>, val: bool) -> Option<()> {
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
pub fn get_sight_block(state: &GameState) -> Option<SightBlock> {
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

    // Swap buffers to finalize rendering
    target.finish().unwrap();
}
