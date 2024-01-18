#![allow(dead_code)]

use std::{
    fmt::Display,
    ops::{AddAssign, SubAssign},
};

use bevy::{
    math::{vec3, EulerRot, Quat, Vec3},
    transform::components::Transform,
};
use bevy_egui::egui::{
    self, emath::Numeric, Button, ImageButton, ImageSource, Layout, Response, Ui, Vec2,
};

pub fn combobox_enum<T>(
    ui: &mut Ui,
    value: &mut T,
    id: impl std::hash::Hash,
    width: Option<f32>,
) -> Response
where
    T: strum::IntoEnumIterator + Display + PartialEq + Clone,
{
    let mut combobox = egui::ComboBox::from_id_source(id).selected_text(value.to_string());
    combobox = if let Some(width) = width {
        combobox.width(width)
    } else {
        combobox.width(ui.available_width())
    };
    combobox
        .show_ui(ui, |ui| {
            for variant in T::iter() {
                ui.selectable_value(value, variant.clone(), variant.to_string());
            }
        })
        .response
}

pub fn increment_buttons<Num>(ui: &mut Ui, value: &mut Num, increment: &Num)
where
    Num: Numeric + AddAssign + SubAssign,
{
    let width = 15.;
    if ui
        .add(Button::new("-").min_size(Vec2 { x: width, y: 0. }))
        .clicked()
    {
        *value -= *increment;
    }
    if ui
        .add(Button::new("+").min_size(Vec2 { x: width, y: 0. }))
        .clicked()
    {
        *value += *increment;
    }
}

pub fn image_selectable_value<'a, Value: PartialEq>(
    ui: &mut egui::Ui,
    size: f32,
    current: &mut Value,
    selected: Value,
    img: impl Into<ImageSource<'a>>,
) -> Response {
    let img = egui::Image::new(img);
    // scale up the svg image by the window scale factor so it doesn't look blurry on lower resolution screens
    img.load_for_size(
        ui.ctx(),
        egui::Vec2::splat(size) * ui.ctx().pixels_per_point(),
    )
    .unwrap();

    let res = ui.allocate_ui(egui::Vec2::splat(size), |ui| {
        let btn = ui.add(ImageButton::new(img).selected(*current == selected));
        if btn.clicked() {
            *current = selected;
        };
        btn
    });
    res.inner
}

pub fn drag_vec3(ui: &mut Ui, value: &mut Vec3, speed: f32) -> (Response, Response, Response) {
    ui.columns(3, |ui| {
        let x = ui[0]
            .centered_and_justified(|ui| {
                ui.add(
                    egui::DragValue::new(&mut value.x)
                        .speed(speed)
                        .fixed_decimals(1),
                )
            })
            .inner;
        let y = ui[1]
            .centered_and_justified(|ui| {
                ui.add(
                    egui::DragValue::new(&mut value.y)
                        .speed(speed)
                        .fixed_decimals(1),
                )
            })
            .inner;
        let z = ui[2]
            .centered_and_justified(|ui| {
                ui.add(
                    egui::DragValue::new(&mut value.z)
                        .speed(speed)
                        .fixed_decimals(1),
                )
            })
            .inner;
        (x, y, z)
    })
    // ui.columns(3, |ui| {
    //     let x = ui[0].add(
    //         egui::DragValue::new(&mut value.x)
    //             .speed(speed)
    //             .fixed_decimals(1),
    //     );
    //     let y = ui[1].add(
    //         egui::DragValue::new(&mut value.y)
    //             .speed(speed)
    //             .fixed_decimals(1),
    //     );
    //     let z = ui[2].add(
    //         egui::DragValue::new(&mut value.z)
    //             .speed(speed)
    //             .fixed_decimals(1),
    //     );
    //     (x, y, z)
    // })
}

pub fn rotation_edit(ui: &mut egui::Ui, transform: &mut Transform, speed: f32) -> bool {
    // this was a fucking fuck to code

    let euler = transform.rotation.to_euler(EulerRot::XYZ);

    let mut rot = vec3(
        f32::to_degrees(euler.0),
        f32::to_degrees(euler.1),
        f32::to_degrees(euler.2),
    );

    let clamp_0_360 = |angle: &mut f32| {
        *angle %= 360.;
        if *angle < 0. {
            *angle += 360.;
        }
        if *angle == 360. {
            *angle = 0.;
        }
    };

    clamp_0_360(&mut rot.x);
    clamp_0_360(&mut rot.y);
    clamp_0_360(&mut rot.z);

    let (x, y, z) = drag_vec3(ui, &mut rot, speed);

    let changed = x.changed() || y.changed() || z.changed();

    let mut update_rotation = |res: Response, axis: Vec3| {
        if res.changed() {
            // get the drag delta
            let delta = res.drag_delta().x / 100.;
            if res.dragged() {
                // rotate around the specific axis if we are dragging it
                transform.rotate_local_axis(axis, delta);
            } else {
                // if we are setting the value by typing it in, set the quaternion directly
                // we can't always set the quaternion directly because it leads to problems involving
                // the fact that euler -> quat -> euler does not always give the same result
                transform.rotation = Quat::from_euler(
                    EulerRot::XYZ,
                    f32::to_radians(rot.x),
                    f32::to_radians(rot.y),
                    f32::to_radians(rot.z),
                );
            }
        }
    };
    update_rotation(x, Vec3::X);
    update_rotation(y, Vec3::Y);
    update_rotation(z, Vec3::Z);

    transform.rotation = transform.rotation.normalize();

    changed
}