use client::{GameState, Player, VOX_H, VOX_L, VOX_W};

const ACCEL_GRAV: f32 = 9.8; // Acceleration due to gravity, in m/s^2

// Update player position and velocity
pub fn do_player_physics(player: &mut Player, dt: f32) {
    player.pos += player.velocity * dt;
    // Prevent clipping through the floor when falling quickly
    if player.pos.y < 0.0 {
        player.pos.y = 0.0;
    }
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
