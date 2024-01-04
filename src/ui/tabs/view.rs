use super::super::app_state::AppSettings;
use crate::viewer::kmp::{sections::KmpSections, KmpVisibilityUpdated};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui;
use strum::IntoEnumIterator;

#[derive(SystemParam)]
pub struct ViewParams<'w> {
    settings: ResMut<'w, AppSettings>,
    ev_kmp_visibility_updated: EventWriter<'w, KmpVisibilityUpdated>,
}

pub fn show_view_tab(ui: &mut egui::Ui, p: &mut ViewParams) {
    for (i, section_name) in KmpSections::iter().enumerate() {
        let Some(section) = p.settings.kmp_model.sections.field_at_mut(i) else {
            continue;
        };
        let Ok(visible) = section.path_mut::<bool>("visible") else {
            continue;
        };
        let changed = ui.checkbox(visible, section_name.to_string()).changed();
        if changed {
            p.ev_kmp_visibility_updated.send(KmpVisibilityUpdated);
        }
    }
}
