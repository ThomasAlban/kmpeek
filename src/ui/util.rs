#![allow(dead_code)]

use std::{
    fmt::Display,
    ops::{AddAssign, RangeInclusive, SubAssign},
};

use bevy_egui::egui::{self, emath::Numeric, Button, ImageButton, ImageSource, Response, Ui, Vec2};

pub fn combobox_enum<T>(
    ui: &mut Ui,
    value: &mut T,
    label: &'static str,
    hover: Option<&'static str>,
    width: Option<f32>,
) where
    T: strum::IntoEnumIterator + Display + PartialEq + Clone,
{
    ui.horizontal(|ui| {
        if let Some(hover) = hover {
            ui.label(label).on_hover_text(hover);
        } else {
            ui.label(label);
        }
        let mut combobox = egui::ComboBox::from_id_source(label).selected_text(value.to_string());
        if let Some(width) = width {
            combobox = combobox.width(width);
        }
        combobox.show_ui(ui, |ui| {
            for variant in T::iter() {
                ui.selectable_value(value, variant.clone(), variant.to_string());
            }
        });
    });
}

pub fn num_edit<Num>(
    ui: &mut Ui,
    value: &mut Num,
    label: Option<&'static str>,
    range: Option<RangeInclusive<Num>>,
    increment: Option<Num>,
) -> Response
where
    Num: Numeric + AddAssign + SubAssign,
{
    ui.horizontal(|ui| {
        if let Some(label) = label {
            ui.label(label);
        }

        let mut drag_value = egui::DragValue::new(value).speed(0.05);
        if let Some(range) = range {
            drag_value = drag_value.clamp_range(range);
        }
        ui.add(drag_value);

        if let Some(increment) = increment {
            increment_buttons(ui, value, &increment);
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
