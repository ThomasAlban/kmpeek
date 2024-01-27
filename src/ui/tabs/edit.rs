use super::UiSubSection;
use crate::{
    ui::util::{drag_vec3, rotation_edit},
    viewer::edit::select::Selected,
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui;

#[derive(SystemParam)]
pub struct ShowEditTab<'w, 's> {
    q_selected: Query<'w, 's, &'static mut Transform, With<Selected>>,
}
impl UiSubSection for ShowEditTab<'_, '_> {
    fn show(&mut self, ui: &mut bevy_egui::egui::Ui) {
        let Ok(mut transform) = self.q_selected.get_single_mut() else {
            return;
        };

        ui.visuals_mut().collapsing_header_frame = true;

        egui::CollapsingHeader::new("Transform")
            .default_open(true)
            .show_unindented(ui, |ui| {
                egui::Grid::new("transform_controls")
                    .num_columns(2)
                    .min_col_width(100.)
                    .show(ui, |ui| {
                        ui.label("Translate");

                        let mut transform_cp = *transform;
                        drag_vec3(ui, &mut transform_cp.translation, 10.);
                        ui.end_row();

                        ui.label("Rotate");
                        rotation_edit(ui, &mut transform_cp, 1.);

                        transform.set_if_neq(transform_cp);
                    });
            });

        // this is where ui for the currently selected point(s) will be
    }
}
