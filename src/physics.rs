use cgmath::prelude::*;
use cgmath::{Euler, Point3, Quaternion, Rad, Vector2, Vector3};

use clamp::clamp;

use nd_iter::iter_3d;

use rand::prelude::*;

use client::{GameState, Player, PlayerState, VoxelType, VOX_MAX_X, VOX_MAX_Y, VOX_MAX_Z};
use render::VoxInd;

const EYE_HEIGHT: f32 = 1.62; // Height of the player's eyes
const FOREHEAD_SIZE: f32 = 0.2; // Vertical distance from the player's eyes to the top of the player
const PLAYER_RADIUS: f32 = 0.3; // Radius of the player hitbox (cylinder)
const ACCEL_GRAV: f32 = 9.8; // Acceleration due to gravity, in m/s^2

// In m/s
const FLY_SPEED: f32 = 30.0;
const WALK_SPEED: f32 = 4.3;
const RUN_SPEED: f32 = 5.6;

// Determine if the voxel at `pos` is a boundary (one voxel outside the voxel grid)
fn boundary_at_pos(pos: Point3<f32>) -> bool {
    pos.x as i32 == -1
        || pos.y as i32 == -1
        || pos.z as i32 == -1
        || pos.x as usize == VOX_MAX_X
        || pos.y as usize == VOX_MAX_Y
        || pos.z as usize == VOX_MAX_Z
}

// Get the type of the voxel at `pos`, returning `None` when the position isn't within the bounds
// of the voxel grid. Note that the boundary (one outside the voxel grid) is considered a voxel.
fn voxel_at_opt(state: &GameState, pos: Point3<f32>) -> Option<VoxelType> {
    if boundary_at_pos(pos) {
        Some(VoxelType::Boundary)
    } else {
        Some(
            *state
                .voxels
                .get(pos.x as usize)?
                .get(pos.y as usize)?
                .get(pos.z as usize)?,
        )
    }
}

// Determine if there is a non-air voxel at `pos`, returning `false` when the position is out of
// bounds
pub fn voxel_at(state: &GameState, pos: Point3<f32>) -> bool {
    voxel_at_opt(state, pos).unwrap_or(VoxelType::Air) != VoxelType::Air
}

// Set a voxel at a coordinate, returning `None` if out-of-bounds
pub fn put_voxel(state: &mut GameState, pos: Point3<VoxInd>, voxel_type: VoxelType) -> Option<()> {
    *state
        .voxels
        .get_mut(pos.x as usize)?
        .get_mut(pos.y as usize)?
        .get_mut(pos.z as usize)? = voxel_type;
    state.dirty = true;
    Some(())
}

pub fn player_in_freefall(state: &GameState) -> bool {
    !player_is_standing(state) && state.player.state != PlayerState::Flying
}

// Is the player standing on the bottom of the voxel grid or sand?
fn player_is_standing(state: &GameState) -> bool {
    let foot_pos = state.player.pos - Vector3::new(0.0, EYE_HEIGHT, 0.0);
    let surface_pos = foot_pos - Vector3::new(0.0, 1.0, 0.0);
    voxel_at(state, surface_pos)
}

// Clip the player inside the bounds of the voxel grid
fn bounds_correct_player(player: &mut Player) {
    player.pos.x = clamp(
        PLAYER_RADIUS,
        player.pos.x,
        VOX_MAX_X as f32 - PLAYER_RADIUS,
    );
    player.pos.y = clamp(EYE_HEIGHT, player.pos.y, VOX_MAX_Y as f32 - FOREHEAD_SIZE);
    player.pos.z = clamp(
        PLAYER_RADIUS,
        player.pos.z,
        VOX_MAX_Z as f32 - PLAYER_RADIUS,
    );
}

// Update player position and velocity
pub fn do_player_physics(player: &mut Player, dt: f32) {
    player.pos += player.velocity * dt;
    // TODO: Prevent player from clipping inside sand
    bounds_correct_player(player);
    player.velocity.y -= ACCEL_GRAV * dt;
}

// Get a random direction along a 2D plane
fn get_rand_dir<T: Rng>(rng: &mut T) -> (i32, i32) {
    // Add one because `gen_range()` is exclusive on the upper bound
    (rng.gen_range(-1, 1 + 1), rng.gen_range(-1, 1 + 1))
}

// Propagate the voxels downwards (gravity)
// TODO: Somehow use `dt` here
pub fn do_sandfall(state: &mut GameState) {
    if state.frame % 10 == 0 {
        for (x, y, z) in iter_3d(0..VOX_MAX_X, 1..VOX_MAX_Y, 0..VOX_MAX_Z) {
            // TODO: Make this less boilerplate
            let hi_ty = state.voxels[x][y][z];
            let lo_ty = state.voxels[x][y - 1][z];
            // Try direct up-down swap
            if hi_ty != VoxelType::Air && lo_ty == VoxelType::Air {
                state.voxels[x][y][z] = lo_ty;
                state.voxels[x][y - 1][z] = hi_ty;
                state.dirty = true;
            } else if x != 0 && state.voxels[x - 1][y - 1][z] == VoxelType::Air
                || z != 0 && state.voxels[x][y - 1][z - 1] == VoxelType::Air
                || x != 0 && z != 0 && state.voxels[x - 1][y - 1][z - 1] == VoxelType::Air
                || x != VOX_MAX_X - 1 && state.voxels[x + 1][y - 1][z] == VoxelType::Air
                || z != VOX_MAX_Z - 1 && state.voxels[x][y - 1][z + 1] == VoxelType::Air
                || x != VOX_MAX_X - 1
                    && z != VOX_MAX_Z - 1
                    && state.voxels[x + 1][y - 1][z + 1] == VoxelType::Air
            {
                // Try moving sideways-down
                let (dx, dz) = get_rand_dir(&mut state.rng);
                let x_alt = (x as i32 + dx) as usize;
                let z_alt = (z as i32 + dz) as usize;
                if x_alt < VOX_MAX_X
                    && z_alt < VOX_MAX_Z
                    && state.voxels[x_alt][y - 1][z_alt] == VoxelType::Air
                {
                    state.voxels[x][y][z] = VoxelType::Air;
                    state.voxels[x_alt][y - 1][z_alt] = hi_ty;
                    state.dirty = true;
                }
            }
        }
    }
}

// Calculate the forward vector based on the player angle
pub fn compute_forward_vector(angle: Vector2<f32>) -> Vector3<f32> {
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
pub fn compute_dir_vectors(angle: Vector2<f32>) -> (Vector3<f32>, Vector3<f32>, Vector3<f32>) {
    let forward = compute_forward_vector(angle);
    let right = forward.cross(Vector3::new(0.0, 1.0, 0.0));
    let up = right.cross(forward);
    (forward, right, up)
}

pub fn get_move_speed(player_state: PlayerState) -> f32 {
    match player_state {
        PlayerState::Normal => WALK_SPEED,
        PlayerState::Running => RUN_SPEED,
        PlayerState::Flying => FLY_SPEED,
    }
}
