mod kcl;
mod kmp;

use crate::kcl::*;

use std::fs::File;
use three_d::*;

pub fn main() {
    let window = Window::new(WindowSettings {
        title: "test".to_string(),
        ..Default::default()
    })
    .unwrap();

    let context = window.gl();

    let f = File::open("dry_dry_ruins.kcl").unwrap();
    let kcl = KCL::read(f).unwrap();
    let gm = kcl.build_model(&context);

    let mut avg_point = Vec3 {
        x: 0.,
        y: 0.,
        z: 0.,
    };
    let mut count = 0.;
    for tri in kcl.tris {
        for vertex in tri.vertices {
            avg_point += vertex;
            count += 1.;
        }
    }
    avg_point = avg_point / count;

    let mut camera = Camera::new_perspective(
        window.viewport(),
        vec3(0.0, 0.0, 2.0),
        vec3(avg_point.x, avg_point.y, avg_point.z),
        vec3(0.0, 1.0, 0.0),
        degrees(45.0),
        1000.,
        10000000.0,
    );

    let u8_colors = KCL_COLORS.map(|i| i.map(|j| (j * 255.) as u8));
    println!("{:#?}", u8_colors);

    let mut control = OrbitControl::new(*camera.target(), 1.0, 10000000.0);

    let dir_light_1 = DirectionalLight::new(&context, 0.9, Color::WHITE, &vec3(0., -1., -1.));
    let dir_light_2 = DirectionalLight::new(&context, 0.9, Color::WHITE, &vec3(0., -1., 1.));

    let ambient_light = renderer::light::AmbientLight::new(&context, 0.8, Color::WHITE);

    // Start the main render loop
    window.render_loop(
        move |mut frame_input| // Begin a new frame with an updated frame input
    {
        // Ensure the viewport matches the current window viewport which changes if the window is resized
        camera.set_viewport(frame_input.viewport);

        control.handle_events(&mut camera, &mut frame_input.events);

        frame_input.screen()
            .clear(ClearState::color_and_depth(0.8, 0.8, 0.8, 1.0, 1.0))
            .render(
                &camera, &gm, &[&ambient_light, &dir_light_1, &dir_light_2]
            );

        // Returns default frame output to end the frame
        FrameOutput::default()
    },
    );
}
