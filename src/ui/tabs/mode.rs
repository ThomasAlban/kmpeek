use super::UiTabSection;
use crate::{
    ui::{
        app_state::{AppMode, AppSettings, AppState},
        util::{combobox_enum, num_edit},
    },
    viewer::kmp::{components::TrackInfo, sections::KmpSections, KmpVisibilityUpdate},
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui;
use std::ops::RangeInclusive;

#[derive(SystemParam)]
pub struct ShowModeTab<'w, 's> {
    app_state: ResMut<'w, AppState>,
    p: ParamSet<
        'w,
        's,
        (
            ShowTrackInfoMode<'w, 's>,
            ShowStartFinishPointsMode<'w, 's>,
            ShowPathsMode<'w>,
            ShowCheckpointsRespawnsMode,
            ShowObjectsMode,
            ShowCamerasMode,
            ShowRoutesAreasMode,
            ShowFreeEditMode,
        ),
    >,
}
impl UiTabSection for ShowModeTab<'_, '_> {
    fn show(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.app_state.mode.to_string());
        ui.separator();
        match self.app_state.mode {
            AppMode::TrackInfo => self.p.p0().show(ui),
            AppMode::StartFinishPoints => self.p.p1().show(ui),
            AppMode::Paths => self.p.p2().show(ui),
            AppMode::CheckpointsRespawns => self.p.p3().show(ui),
            AppMode::Objects => self.p.p4().show(ui),
            AppMode::Cameras => self.p.p5().show(ui),
            AppMode::RoutesAreas => self.p.p6().show(ui),
            AppMode::FreeEdit => self.p.p7().show(ui),
        }
    }
}

#[derive(SystemParam)]
pub struct ShowTrackInfoMode<'w, 's> {
    query: Query<'w, 's, &'static mut TrackInfo>,
    settings: Res<'w, AppSettings>,
}
impl UiTabSection for ShowTrackInfoMode<'_, '_> {
    fn show(&mut self, ui: &mut egui::Ui) {
        let Ok(mut track_info) = self.query.get_single_mut() else {
            return;
        };

        combobox_enum(
            ui,
            &mut track_info.track_type,
            "Track Type:",
            None,
            Some(60.),
        );

        num_edit(
            ui,
            &mut track_info.lap_count,
            Some("Lap Count:"),
            Some("This only works when the Lap & Speed Modifier cheat code is enabled. In Nintendo tracks it is always set to 3, but the base game ignores this value"),
            Some(RangeInclusive::new(1, 10)),
            Some(self.settings.increment as u8)
        );

        num_edit(
            ui,
            &mut track_info.speed_mod,
            Some("Speed Mod:"),
            Some("This only works when the Lap & Speed Modifier cheat code is enabled. If set to 0, a value of 1 will be used when the code is enabled (for backwards compatibility)"),
            Some(RangeInclusive::new(0., 3.)),
            Some(self.settings.increment as f32)
        );

        ui.collapsing("Lens Flare", |ui| {
            ui.horizontal(|ui| {
                ui.label("Flare Colour:").on_hover_text(
                    "The lighting colour that covers the screen when the lensFX object is used",
                );
                ui.color_edit_button_srgba_unmultiplied(&mut track_info.lens_flare_color);
            });
            ui.checkbox(&mut track_info.lens_flare_flashing, "Flashing")
                .on_hover_text("Whether or not the lens flare should flash/pulsate");
        });
    }
}

#[derive(SystemParam)]
pub struct ShowStartFinishPointsMode<'w, 's> {
    track_info: Query<'w, 's, &'static mut TrackInfo>,
}
impl UiTabSection for ShowStartFinishPointsMode<'_, '_> {
    fn show(&mut self, ui: &mut egui::Ui) {
        let Ok(mut track_info) = self.track_info.get_single_mut() else {
            return;
        };
        combobox_enum(
            ui,
            &mut track_info.first_player_pos,
            "First Player Position:",
            Some("Whether the player in first place should be positioned to the left or right at the start line"),
            Some(40.)
        );
        ui.checkbox(
            &mut track_info.narrow_player_spacing,
            "Narrow Player Spacing",
        )
        .on_hover_text(
            "Whether players at the start line should be positioned in a more narrow arrangement",
        );
    }
}

#[derive(SystemParam)]
pub struct ShowPathsMode<'w> {
    settings: ResMut<'w, AppSettings>,
    ev_kmp_visibility_update: EventWriter<'w, KmpVisibilityUpdate>,
}
impl UiTabSection for ShowPathsMode<'_> {
    fn show(&mut self, ui: &mut egui::Ui) {
        let enemy_paths_visible = ui.checkbox(
            &mut self.settings.kmp_model.sections.visible[usize::from(KmpSections::EnemyPaths)],
            "Show Enemy Paths",
        );
        let item_paths_visible = ui.checkbox(
            &mut self.settings.kmp_model.sections.visible[usize::from(KmpSections::ItemPaths)],
            "Show Item Paths",
        );
        if enemy_paths_visible.changed() || item_paths_visible.changed() {
            self.ev_kmp_visibility_update.send_default();
        }
        ui.button("Copy Enemy to Item").clicked();
        ui.button("Copy Item to Enemy").clicked();
    }
}

#[derive(SystemParam)]
pub struct ShowCheckpointsRespawnsMode;
impl UiTabSection for ShowCheckpointsRespawnsMode {
    fn show(&mut self, ui: &mut egui::Ui) {
        ui.label("Checkpoints & Respawns");
    }
}

#[derive(SystemParam)]
pub struct ShowObjectsMode;
impl UiTabSection for ShowObjectsMode {
    fn show(&mut self, ui: &mut egui::Ui) {
        ui.label("Objects");
    }
}

#[derive(SystemParam)]
pub struct ShowCamerasMode;
impl UiTabSection for ShowCamerasMode {
    fn show(&mut self, ui: &mut egui::Ui) {
        ui.label("Cameras");
    }
}

#[derive(SystemParam)]
pub struct ShowRoutesAreasMode;
impl UiTabSection for ShowRoutesAreasMode {
    fn show(&mut self, ui: &mut egui::Ui) {
        ui.label("Routes & Areas");
    }
}

#[derive(SystemParam)]
pub struct ShowFreeEditMode;
impl UiTabSection for ShowFreeEditMode {
    fn show(&mut self, ui: &mut egui::Ui) {
        ui.label("Free Edit");
    }
}
