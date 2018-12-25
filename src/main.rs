extern crate three;

use std::boxed::Box;
use std::process;

use three::Key;
use three::Mesh;
use three::Window;
use three::camera:: Camera;
use three::Geometry;
use three::controls::FirstPerson;
use three::Object;
use three::controls::axis;
use three::window::CursorState;

const L: usize = 160;
const W: usize = 160;
const H: usize = 160;
const VOX_SIZE: f32 = 1.0 / 32.0;

struct Client {
    win: Window,
    cam: Camera,
    ctrls: FirstPerson,
    // true = sand, false = air
    voxels: Box<[[[bool; L]; W]; H]>,
    // Dirty flag to check if the voxels have changed
    dirty: bool,
}

// TODO: Refactor client parameters with impl

fn update_state(client: &mut Client) {
}

fn make_voxel_mesh(win: &mut Window, x: usize, y: usize, z: usize) -> Mesh {
    let x = x as f32 * VOX_SIZE;
    let y = y as f32 * VOX_SIZE;
    let z = z as f32 * VOX_SIZE;
    let geo = Geometry::cuboid(VOX_SIZE, VOX_SIZE, VOX_SIZE);
    let material = three::material::Basic {
        color: 0xFFFF00,
        .. Default::default()
    };
    let mesh = win.factory.mesh(geo, material);
    mesh.set_position([x, y, z]);
    mesh
}

fn render(client: &mut Client) {
    // TODO: Think of a cleaner way to do this
    if client.dirty {
        for (x, arr_2d) in client.voxels.iter().enumerate() {
            for (y, arr_1d) in arr_2d.iter().enumerate() {
                for (z, voxel) in arr_1d.iter().enumerate() {
                    if *voxel {
                        let mesh = make_voxel_mesh(&mut client.win, x, y, z);
                        client.win.scene.add(mesh);
                    }
                }
            }
        }
        client.dirty = false;
    }

    client.win.render(&client.cam);
}

// TODO: Refactor into functions
fn main() {
    let mut win = Window::new("sandvox");

    let cam = win.factory.perspective_camera(60.0, 0.1 .. 10.0);
    cam.set_position([0.0, 0.0, 10.0]);
    win.scene.add(&cam);
    let ctrls = FirstPerson::builder(&cam)
        .vertical_movement(false)
        .axis_forward(Some(axis::Key{
            neg: Key::Down,
            pos: Key::Up,
        }))
        .axis_strafing(Some(axis::Key{
            neg: Key::Left,
            pos: Key::Right,
        }))
        .build();
    let mut client = Client{
        win,
        cam,
        ctrls,
        // TODO: Maybe make this static
        voxels: Box::new([[[false; L]; W]; H]),
        dirty: false,
    };

    // TODO: Remove this test world
    for i in 0..L {
        client.voxels[i][i][i] = true;
    }
    client.dirty = true;

    // TODO: Move this somewhere
    client.win.set_cursor_state(CursorState::Grab);

    while client.win.update() {
        client.ctrls.update(&client.win.input);
        update_state(&mut client);
        render(&mut client);
    }
}
