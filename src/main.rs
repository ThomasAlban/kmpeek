mod kcl;
mod kmp;

use crate::kcl::*;

use std::fs::File;
use three_d::*;

pub fn main() {
    let window = Window::new(WindowSettings {
        title: "test".to_string(),
        max_size: Some((1280, 720)),
        ..Default::default()
    })
    .unwrap();

    let context = window.gl();

    let f = File::open("course1.kcl").unwrap();
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
        1.,
        10000000.0,
    );

    let mut control = OrbitControl::new(*camera.target(), 1.0, 10000000.0);

    // Start the main render loop
    window.render_loop(
        move |mut frame_input| // Begin a new frame with an updated frame input
    {
        // Ensure the viewport matches the current window viewport which changes if the window is resized
        camera.set_viewport(frame_input.viewport);

        control.handle_events(&mut camera, &mut frame_input.events);

        // Get the screen render target to be able to render something on the screen
        frame_input.screen()
            // Clear the color and depth of the screen render target
            .clear(ClearState::color_and_depth(0.8, 0.8, 0.8, 1.0, 1.0))
            // Render the triangle with the color material which uses the per vertex colors defined at construction
            .render(
                &camera, &gm, &[]
            );

        // Returns default frame output to end the frame
        FrameOutput::default()
    },
    );
}
