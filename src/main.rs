mod flycontrol;
mod kcl;
mod kmp;

use crate::kcl::*;
use crate::kmp::*;

use std::fs::File;
use three_d::*;

pub fn main() {
    let window = Window::new(WindowSettings {
        title: "test".to_string(),
        max_size: None,
        ..Default::default()
    })
    .unwrap();

    let context = window.gl();

    let name = "coconut_mall";

    let kcl_file = File::open(format!("{name}.kcl")).unwrap();
    let kmp_file = File::open(format!("{name}.kmp")).unwrap();

    let kcl = KCL::read(kcl_file).unwrap();
    let kmp = KMP::read(kmp_file).unwrap();

    let kcl_model = kcl.build_model(&context);
    let kmp_model = kmp.build_model(&context);

    let mut avg_point = Vec3 {
        x: 0.,
        y: 0.,
        z: 0.,
    };
    let mut count = 0.;
    for tri_group in kcl.tri_groups {
        for tri in tri_group {
            for vertex in tri.vertices {
                avg_point += vertex;
                count += 1.;
            }
        }
    }
    avg_point = avg_point / count;

    let mut camera = Camera::new_perspective(
        window.viewport(),
        vec3(0.0, 0.0, 2.0),
        vec3(avg_point.x, avg_point.y, avg_point.z),
        vec3(0.0, 1.0, 0.0),
        degrees(45.0),
        200.,
        10000000.0,
    );

    let mut control = flycontrol::FlyControl::new(25.);

    let dir_light_1 = DirectionalLight::new(&context, 0.9, Color::WHITE, &vec3(0., -1., -1.));
    let dir_light_2 = DirectionalLight::new(&context, 0.9, Color::WHITE, &vec3(0., -1., 1.));

    let ambient_light = renderer::light::AmbientLight::new(&context, 0.8, Color::WHITE);

    let mut model = kcl_model;
    model.extend(kmp_model);

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
                &camera, &model, &[&ambient_light, &dir_light_1, &dir_light_2]
            );

        // Returns default frame output to end the frame
        FrameOutput::default()
    },
    );
}
