use clamp::clamp;

use client::{GameState, Player, VOX_H, VOX_L, VOX_W};

const EYE_HEIGHT: f32 = 1.62; // Height of the player's eyes
const ACCEL_GRAV: f32 = 9.8; // Acceleration due to gravity, in m/s^2

pub fn in_freefall(player: &Player) -> bool {
    !is_standing(player) && !player.flying
}

// Is the player standing on the bottom of the voxel grid?
fn is_standing(player: &Player) -> bool {
    player.pos.y <= EYE_HEIGHT
}

// Clip the player inside the bounds of the voxel grid. The y-axis is unclamped in the positive
// direction, so the player can fly arbitrarily high.
fn bounds_correct_player(player: &mut Player) {
    player.pos.x = clamp(0.0, player.pos.x, VOX_L as f32);
    player.pos.y = player.pos.y.max(EYE_HEIGHT);
    player.pos.z = clamp(0.0, player.pos.z, VOX_H as f32);
}

// Update player position and velocity
pub fn do_player_physics(player: &mut Player, dt: f32) {
    player.pos += player.velocity * dt;
    bounds_correct_player(player);
    player.velocity.y -= ACCEL_GRAV * dt;
}

// Propagate the voxels downwards (gravity)
// TODO: Somehow use `dt` here
pub fn do_sandfall(state: &mut GameState) {
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
