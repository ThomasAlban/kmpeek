use super::UiSubSection;
use crate::{
    ui::settings::AppSettings,
    viewer::kmp::{
        sections::{KmpEditMode, KmpModelSections},
        KmpVisibilityUpdate,
    },
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui;
use strum::IntoEnumIterator;

#[derive(Resource, SystemParam)]
pub struct ShowViewTab<'w> {
    settings: ResMut<'w, AppSettings>,
    ev_kmp_visibility_updated: EventWriter<'w, KmpVisibilityUpdate>,
    kmp_edit_mode: ResMut<'w, KmpEditMode>,
}
impl UiSubSection for ShowViewTab<'_> {
    fn show(&mut self, ui: &mut egui::Ui) {
        if ui
            .checkbox(&mut self.settings.view_multiple, "View Multiple")
            .changed()
            && !self.settings.view_multiple
        {
            let i = KmpModelSections::iter()
                .position(|e| e == self.kmp_edit_mode.0)
                .unwrap();
            self.settings.kmp_model.sections.visible = [false; 11];
            self.settings.kmp_model.sections.visible[i] = true;
            self.ev_kmp_visibility_updated.send_default();
        }

        for (i, section) in KmpModelSections::iter().enumerate() {
            egui::collapsing_header::CollapsingState::load_with_default_open(
                ui.ctx(),
                section.to_string().into(),
                false,
            )
            .show_header(ui, |ui| {
                let mut visible_changed = false;
                if self.settings.view_multiple {
                    visible_changed = ui
                        .checkbox(&mut self.settings.kmp_model.sections.visible[i], "")
                        .changed();
                }
                if ui
                    .selectable_value(&mut self.kmp_edit_mode.0, section, section.to_string())
                    .clicked()
                    && !self.settings.view_multiple
                {
                    self.settings.kmp_model.sections.visible = [false; 11];
                    self.settings.kmp_model.sections.visible[i] = true;
                    visible_changed = true;
                };
                if visible_changed {
                    self.ev_kmp_visibility_updated.send_default();
                }
            })
            .body(|ui| ui.label("WIP: Show each item in the section"));
        }
    }
}
