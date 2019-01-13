use cgmath::{Point3, Vector3};

use clamp::clamp;

use client::{GameState, Player, VOX_MAX_X, VOX_MAX_Y, VOX_MAX_Z};
use render::VoxInd;

const EYE_HEIGHT: f32 = 16.2; // Height of the player's eyes
const FOREHEAD_SIZE: f32 = 2.0; // Vertical distance from the player's eyes to the top of the player
const PLAYER_RADIUS: f32 = 3.0; // Radius of the player hitbox (cylinder)
const ACCEL_GRAV: f32 = 100.0; // Acceleration due to gravity, in dm/s^2

// Determine if the voxel at `pos` is a boundary (one voxel outside the voxel grid)
fn boundary_at_pos(pos: Point3<f32>) -> bool {
    pos.x as i32 == -1
        || pos.y as i32 == -1
        || pos.z as i32 == -1
        || pos.x as usize == VOX_MAX_X
        || pos.y as usize == VOX_MAX_Y
        || pos.z as usize == VOX_MAX_Z
}

// Determine if there is a voxel at `pos`, returning `None` when the position isn't within the
// bounds of the voxel grid. Note that the boundary (one outside the voxel grid) is considered a
// voxel.
fn voxel_at_opt(state: &GameState, pos: Point3<f32>) -> Option<bool> {
    if boundary_at_pos(pos) {
        Some(true)
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

// Determine if the is a voxel at `pos`, returning `false` when the position is out of bounds
pub fn voxel_at(state: &GameState, pos: Point3<f32>) -> bool {
    voxel_at_opt(state, pos).unwrap_or(false)
}

// Set a voxel at a coordinate, returning `None` if out-of-bounds
pub fn put_voxel(state: &mut GameState, pos: Point3<VoxInd>, val: bool) -> Option<()> {
    *state
        .voxels
        .get_mut(pos.x as usize)?
        .get_mut(pos.y as usize)?
        .get_mut(pos.z as usize)? = val;
    state.dirty = true;
    Some(())
}

pub fn player_in_freefall(state: &GameState) -> bool {
    !player_is_standing(state) && !state.player.flying
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

// Propagate the voxels downwards (gravity)
// TODO: Somehow use `dt` here
pub fn do_sandfall(state: &mut GameState) {
    if state.frame % 10 == 0 {
        // TODO: Find a better way to iterate over voxels
        for x in 0..VOX_MAX_X {
            for y in 0..VOX_MAX_Y {
                for z in 0..VOX_MAX_Z {
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
