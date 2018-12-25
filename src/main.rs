extern crate three;

use std::boxed::Box;

use three::camera::Camera;
use three::controls::axis;
use three::controls::FirstPerson;
use three::window::CursorState;
use three::Geometry;
use three::Key;
use three::Mesh;
use three::Object;
use three::Window;

const L: usize = 160;
const W: usize = 160;
const H: usize = 160;
const VOX_SIZE: f32 = 1.0 / 32.0;

struct Client {
    win: Window,
    cam: Camera,
    ctrls: FirstPerson,
    // true = sand, false = air
    voxels: Box<[[[bool; H]; W]; L]>,
    // Dirty flag to check if the voxels have changed
    dirty: bool,
}

impl Client {
    fn do_input(&mut self) {
        // Default controls
        self.ctrls.update(&self.win.input);

        // Custom handling
        let mut should_reset = false;
        for key in self.win.input.keys_hit() {
            match key {
                Key::Space => should_reset = true,
                _ => {}
            }
        }
        if should_reset {
            self.reset_world();
        }
    }

    fn reset_world(&mut self) {
        // TODO: Remove this test world
        for x in 0..L {
            for y in 0..W {
                for z in 0..H {
                    self.voxels[x][y][z] = x == y && x == z;
                }
            }
        }
        self.dirty = true;
    }

    fn update_state(&mut self) {
        // Make sand fall
        for x in 0..L {
            for y in 0..W {
                for z in 0..H {
                    // TODO: Swapping might not be the best way
                    let vox = self.voxels[x][y][z];
                    let low_vox = if y > 0 {
                        self.voxels[x][y - 1][z]
                    } else {
                        true
                    };
                    if vox && !low_vox {
                        // Swap sand blocks
                        self.voxels[x][y][z] = low_vox;
                        self.voxels[x][y - 1][z] = vox;
                        self.dirty = true;
                    }
                }
            }
        }
    }

    fn make_voxel_mesh(&mut self, x: usize, y: usize, z: usize) -> Mesh {
        let x = x as f32 * VOX_SIZE;
        let y = y as f32 * VOX_SIZE;
        let z = z as f32 * VOX_SIZE;
        let geo = Geometry::cuboid(VOX_SIZE, VOX_SIZE, VOX_SIZE);
        let material = three::material::Basic {
            color: 0xFFFF00,
            ..Default::default()
        };
        let mesh = self.win.factory.mesh(geo, material);
        mesh.set_position([x, y, z]);
        mesh
    }

    fn render(&mut self) {
        // TODO: Think of a cleaner way to do this
        if self.dirty {
            // Clear scene
            self.win.scene = self.win.factory.scene();

            // Add voxels
            for x in 0..L {
                for y in 0..W {
                    for z in 0..H {
                        if self.voxels[x][y][z] {
                            let mesh = self.make_voxel_mesh(x, y, z);
                            self.win.scene.add(mesh);
                        }
                    }
                }
            }
            self.dirty = false;
        }

        self.win.render(&self.cam);
    }
}

// TODO: Refactor into functions
fn main() {
    let mut win = Window::new("sandvox");

    let cam = win.factory.perspective_camera(60.0, 0.1..10.0);
    cam.set_position([10.0, 10.0, 10.0]);
    win.scene.add(&cam);
    let ctrls = FirstPerson::builder(&cam)
        .vertical_movement(false)
        .axis_forward(Some(axis::Key {
            neg: Key::Down,
            pos: Key::Up,
        }))
        .axis_strafing(Some(axis::Key {
            neg: Key::Left,
            pos: Key::Right,
        }))
        .build();
    let mut client = Client {
        win,
        cam,
        ctrls,
        // TODO: Maybe make this static
        voxels: Box::new([[[false; H]; W]; L]),
        dirty: false,
    };

    client.reset_world();

    // TODO: Move this somewhere
    // TODO: Ungrab on ESC
    // TODO: Hide cursor
    //client.win.set_cursor_state(CursorState::Grab);

    while client.win.update() {
        client.do_input();
        client.update_state();
        client.render();
    }
}
