extern crate three;

fn main() {
    let mut window = three::Window::new("sandvox");

    let vertices = vec![
        [-0.5, -0.5, -0.5].into(),
        [ 0.5, -0.5, -0.5].into(),
        [ 0.0,  0.5, -0.5].into(),
    ];
    let geometry = three::Geometry::with_vertices(vertices);
    let material = three::material::Basic {
        color: 0xFFFF00,
        .. Default::default()
    };
    let mesh = window.factory.mesh(geometry, material);
    window.scene.add(&mesh);

    let center = [0.0, 0.0];
    let yextent = 1.0;
    let zrange = -1.0 .. 1.0;
    let camera = window.factory.orthographic_camera(center, yextent, zrange);

    while window.update() {
        window.render(&camera);
    }
}
