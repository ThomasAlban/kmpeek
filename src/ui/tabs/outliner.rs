use super::UiSubSection;
use crate::{
    ui::{
        ui_state::KmpVisibility,
        util::{view_icon_btn, Icons},
    },
    viewer::{
        edit::select::Selected,
        kmp::{
            components::{
                AreaPoint, BattleFinishPoint, CannonPoint, EnemyPathMarker, ItemPathMarker, KmpCamera, Object,
                RespawnPoint, StartPoint,
            },
            path::{EnemyPathGroups, ItemPathGroups, PathGroup},
            sections::{KmpEditMode, KmpSections},
        },
    },
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{self, collapsing_header::CollapsingState, Align, Color32, Layout, Ui};

/// collects an iterator into a vector in a concise way
macro_rules! to_vec {
    ($x:expr) => {
        $x.iter().collect::<Vec<_>>()
    };
}

#[derive(SystemParam)]
pub struct ShowOutlinerTab<'w, 's> {
    // keys: Res<'w, Input<KeyCode>>,
    edit_mode: ResMut<'w, KmpEditMode>,
    commands: Commands<'w, 's>,

    start_points: Query<'w, 's, Entity, With<StartPoint>>,
    enemy_paths: Query<'w, 's, Entity, With<EnemyPathMarker>>,
    item_paths: Query<'w, 's, Entity, With<ItemPathMarker>>,
    respawn_points: Query<'w, 's, Entity, With<RespawnPoint>>,
    objects: Query<'w, 's, Entity, With<Object>>,
    areas: Query<'w, 's, Entity, With<AreaPoint>>,
    cameras: Query<'w, 's, Entity, With<KmpCamera>>,
    cannon_points: Query<'w, 's, Entity, With<CannonPoint>>,
    battle_finish_points: Query<'w, 's, Entity, With<BattleFinishPoint>>,

    enemy_groups: Option<ResMut<'w, EnemyPathGroups>>,
    item_groups: Option<ResMut<'w, ItemPathGroups>>,

    q_visibility: Query<'w, 's, &'static mut Visibility>,
    q_selected: Query<'w, 's, Entity, With<Selected>>,
    link_visibilities: Local<'s, bool>,
    keys: Res<'w, ButtonInput<KeyCode>>,
    kmp_visibility: ResMut<'w, KmpVisibility>,
}
impl UiSubSection for ShowOutlinerTab<'_, '_> {
    fn show(&mut self, ui: &mut Ui) {
        *self.link_visibilities = true;

        use KmpSections::*;

        let checkpoints: Vec<Entity> = Vec::new();

        self.show_track_info_header(ui);

        self.show_point_outliner(ui, StartPoints, to_vec!(self.start_points));
        self.show_path_outliner(
            ui,
            EnemyPaths,
            to_vec!(self.enemy_paths),
            &self.enemy_groups.as_ref().map(|e| e.0.clone()),
        );
        self.show_path_outliner(
            ui,
            ItemPaths,
            to_vec!(self.item_paths),
            &self.item_groups.as_ref().map(|e| e.0.clone()),
        );
        self.show_path_outliner(ui, Checkpoints, checkpoints.iter().copied(), &None);
        self.show_point_outliner(ui, RespawnPoints, to_vec!(self.respawn_points));
        self.show_point_outliner(ui, Objects, to_vec!(self.objects));
        self.show_point_outliner(ui, Areas, to_vec!(self.areas));
        self.show_point_outliner(ui, Cameras, to_vec!(self.cameras));
        self.show_point_outliner(ui, CannonPoints, to_vec!(self.cannon_points));
        self.show_point_outliner(ui, BattleFinishPoints, to_vec!(self.battle_finish_points));
    }
}
impl ShowOutlinerTab<'_, '_> {
    const ICON_SIZE: f32 = 14.;
    fn show_point_outliner(&mut self, ui: &mut Ui, selected: KmpSections, entities: impl IntoIterator<Item = Entity>) {
        self.show_header(ui, selected, entities, false);
    }
    fn show_path_outliner(
        &mut self,
        ui: &mut Ui,
        selected: KmpSections,
        entities: impl IntoIterator<Item = Entity>,
        group_info: &Option<Vec<PathGroup>>,
    ) {
        CollapsingState::load_with_default_open(ui.ctx(), format!("{}_outliner", selected).into(), false)
            .show_header(ui, |ui| {
                self.show_header(ui, selected, entities, true);
            })
            .body(|ui| {
                if let Some(groups) = group_info {
                    for (i, pathgroup) in groups.iter().enumerate() {
                        self.show_path(ui, i, pathgroup, Icons::SECTION_COLORS[selected as usize]);
                    }
                }
            });
    }
    fn show_header(
        &mut self,
        ui: &mut Ui,
        selected: KmpSections,
        entities: impl IntoIterator<Item = Entity>,
        path: bool,
    ) {
        let entities: Vec<_> = entities.into_iter().collect();
        let mut current = self.edit_mode.clone();
        let mut visibilities = self.kmp_visibility.clone();
        ui.horizontal(|ui| {
            if !path {
                ui.add_space(18.);
            }
            ui.add_sized(
                [Self::ICON_SIZE, Self::ICON_SIZE],
                if path {
                    Icons::path_group(ui.ctx(), Self::ICON_SIZE)
                } else {
                    Icons::cube_group(ui.ctx(), Self::ICON_SIZE)
                }
                .tint(Icons::SECTION_COLORS[selected as usize]),
            );
            if ui
                .selectable_value(&mut current.0, selected, selected.to_string())
                .clicked()
                && *self.link_visibilities
            {
                visibilities.0 = [false; 10];
                visibilities.0[selected as usize] = true;
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let mut all_visible = if !entities.is_empty() {
                    entities
                        .iter()
                        .all(|e| self.q_visibility.get(*e) == Ok(&Visibility::Visible))
                } else {
                    false
                };
                if view_icon_btn(ui, &mut all_visible).changed() {
                    visibilities.0[selected as usize] = all_visible;
                }
            });
        });
        self.edit_mode.set_if_neq(current);
        self.kmp_visibility.set_if_neq(visibilities);
    }
    fn show_track_info_header(&mut self, ui: &mut Ui) {
        let mut current = self.edit_mode.clone();
        let mut visibilities = self.kmp_visibility.clone();
        ui.horizontal(|ui| {
            ui.add_space(18.);

            ui.add_sized(
                [Self::ICON_SIZE, Self::ICON_SIZE],
                Icons::track_info(ui.ctx(), Self::ICON_SIZE)
                    .tint(Icons::SECTION_COLORS[KmpSections::TrackInfo as usize]),
            );
            if ui
                .selectable_value(&mut current.0, KmpSections::TrackInfo, "Track Info")
                .clicked()
                && *self.link_visibilities
            {
                visibilities.0 = [false; 10];
            }
        });
        self.edit_mode.set_if_neq(current);
        self.kmp_visibility.set_if_neq(visibilities);
    }
    fn show_path(&mut self, ui: &mut Ui, i: usize, pathgroup: &PathGroup, color: Color32) {
        let mut all_visible = if !pathgroup.paths.is_empty() {
            pathgroup
                .paths
                .iter()
                .all(|e| self.q_visibility.get(*e) == Ok(&Visibility::Visible))
        } else {
            false
        };
        ui.horizontal(|ui| {
            ui.add_space(10.);
            ui.add_sized(
                [Self::ICON_SIZE, Self::ICON_SIZE],
                Icons::path(ui.ctx(), Self::ICON_SIZE).tint(color),
            );
            let label = ui.add(
                egui::Label::new(format!("Path {i}"))
                    .selectable(false)
                    .sense(egui::Sense::click()),
            );
            if label.clicked() {
                if !self.keys.pressed(KeyCode::ShiftLeft) && !self.keys.pressed(KeyCode::ShiftRight) {
                    // deselect everything
                    for e in self.q_selected.iter() {
                        self.commands.entity(e).remove::<Selected>();
                    }
                }
                for e in pathgroup.paths.iter() {
                    self.commands.entity(*e).insert(Selected);
                }
            }
            let view_btn_response = ui
                .with_layout(Layout::right_to_left(Align::Center), |ui| {
                    view_icon_btn(ui, &mut all_visible)
                })
                .inner;

            if view_btn_response.changed() {
                for e in pathgroup.paths.iter() {
                    let Ok(mut visibility) = self.q_visibility.get_mut(*e) else {
                        continue;
                    };
                    *visibility = if all_visible {
                        Visibility::Visible
                    } else {
                        Visibility::Hidden
                    };
                }
            }
        });
    }
}
