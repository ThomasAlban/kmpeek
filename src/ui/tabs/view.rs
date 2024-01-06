use super::{super::app_state::AppSettings, UiTabSection};
use crate::viewer::kmp::{sections::KmpSections, KmpVisibilityUpdate};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui;
use strum::IntoEnumIterator;

#[derive(SystemParam)]
pub struct ShowViewTab<'w> {
    settings: ResMut<'w, AppSettings>,
    ev_kmp_visibility_updated: EventWriter<'w, KmpVisibilityUpdate>,
}
impl UiTabSection for ShowViewTab<'_> {
    fn show(&mut self, ui: &mut egui::Ui) {
        for (i, section_name) in KmpSections::iter().enumerate() {
            let visible = &mut self.settings.kmp_model.sections.visible[i];
            let changed = ui.checkbox(visible, section_name.to_string()).changed();
            if changed {
                self.ev_kmp_visibility_updated.send_default();
            }
        }
    }
}
