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
        for (i, section) in KmpModelSections::iter().enumerate() {
            ui.horizontal(|ui| {
                let mut visible_changed = false;
                if ui
                    .selectable_value(&mut self.kmp_edit_mode.0, section, section.to_string())
                    .clicked()
                {
                    self.settings.kmp_model.sections.visible = [false; 11];
                    self.settings.kmp_model.sections.visible[i] = true;
                    visible_changed = true;
                };
                if visible_changed {
                    self.ev_kmp_visibility_updated.send_default();
                }
            });
        }
    }
}
