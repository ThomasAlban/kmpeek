use std::{
    fmt::Display,
    ops::{AddAssign, RangeInclusive, SubAssign},
};

use bevy_egui::egui::{self, emath::Numeric, Button, Ui, Vec2};

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
    hover: Option<&'static str>,
    range: Option<RangeInclusive<Num>>,
    increment: Option<Num>,
) where
    Num: Numeric + AddAssign + SubAssign,
{
    ui.horizontal(|ui| {
        if let Some(label) = label {
            if let Some(hover) = hover {
                ui.label(label).on_hover_text(hover);
            } else {
                ui.label(label);
            }
        }

        let mut drag_value = egui::DragValue::new(value).speed(0.05);
        if let Some(range) = range {
            drag_value = drag_value.clamp_range(range);
        }
        ui.add(drag_value);

        if let Some(increment) = increment {
            increment_buttons(ui, value, &increment);
        }
    });
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
