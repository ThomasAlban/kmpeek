use super::UiSubSection;
use crate::{
    ui::{
        settings::AppSettings,
        util::{view_icon_btn, Icons},
    },
    viewer::kmp::{
        components::{
            AreaPoint, BattleFinishPoint, CannonPoint, EnemyPathMarker, ItemPathMarker, KmpCamera, Object,
            RespawnPoint, StartPoint,
        },
        path::{EnemyPathGroups, ItemPathGroups, PathGroup},
        sections::{KmpEditMode, KmpModelSections},
        KmpVisibilityUpdate,
    },
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{collapsing_header::CollapsingState, Align, Color32, Layout, Ui};

#[derive(SystemParam)]
pub struct ShowOutlinerTab<'w, 's> {
    // keys: Res<'w, Input<KeyCode>>,
    edit_mode: ResMut<'w, KmpEditMode>,

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
    // commands: Commands<'w, 's>,
    q_visibility: Query<'w, 's, &'static mut Visibility>,
    // q_selected: Query<'w, 's, Entity, With<Selected>>,
    ev_kmp_visibility_update: EventWriter<'w, KmpVisibilityUpdate>,
    settings: ResMut<'w, AppSettings>,
    link_visibilities: Local<'s, bool>,
}
impl UiSubSection for ShowOutlinerTab<'_, '_> {
    fn show(&mut self, ui: &mut bevy_egui::egui::Ui) {
        *self.link_visibilities = true;

        // let enemy_paths = self.enemy_paths.iter().collect::<Vec<Entity>>();
        // let enemy_groups = self.enemy_groups.as_ref().map(|e| e.0.clone());
        // let item_paths = self.item_paths.iter().collect::<Vec<Entity>>();
        // let item_groups = self.item_groups.as_ref().map(|e| e.0.clone());

        use KmpModelSections::*;

        let checkpoints: Vec<Entity> = Vec::new();

        self.show_point_outliner(ui, StartPoints, self.start_points.iter().collect::<Vec<_>>());
        self.show_path_outliner(
            ui,
            EnemyPaths,
            self.enemy_paths.iter().collect::<Vec<_>>(),
            &self.enemy_groups.as_ref().map(|e| e.0.clone()),
        );
        self.show_path_outliner(
            ui,
            ItemPaths,
            self.item_paths.iter().collect::<Vec<_>>(),
            &self.item_groups.as_ref().map(|e| e.0.clone()),
        );
        self.show_path_outliner(ui, Checkpoints, checkpoints.iter().copied(), &None);
        self.show_point_outliner(ui, RespawnPoints, self.respawn_points.iter().collect::<Vec<_>>());
        self.show_point_outliner(ui, Objects, self.objects.iter().collect::<Vec<_>>());
        self.show_point_outliner(ui, Areas, self.areas.iter().collect::<Vec<_>>());
        self.show_point_outliner(ui, Cameras, self.cameras.iter().collect::<Vec<_>>());
        self.show_point_outliner(ui, CannonPoints, self.cannon_points.iter().collect::<Vec<_>>());
        self.show_point_outliner(
            ui,
            BattleFinishPoints,
            self.battle_finish_points.iter().collect::<Vec<_>>(),
        );
    }
}
impl ShowOutlinerTab<'_, '_> {
    const ICON_SIZE: f32 = 14.;
    fn show_point_outliner(
        &mut self,
        ui: &mut Ui,
        selected: KmpModelSections,
        entities: impl IntoIterator<Item = Entity>,
    ) {
        self.show_header(ui, selected, entities, false);
    }
    fn show_path_outliner(
        &mut self,
        ui: &mut Ui,
        selected: KmpModelSections,
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
        selected: KmpModelSections,
        entities: impl IntoIterator<Item = Entity>,
        path: bool,
    ) {
        let entities: Vec<_> = entities.into_iter().collect();
        let current = &mut self.edit_mode.0;
        let visibilities = &mut self.settings.kmp_model.sections.visible;
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
            if ui.selectable_value(current, selected, selected.to_string()).clicked() && *self.link_visibilities {
                *visibilities = [false; 10];
                visibilities[selected as usize] = true;
                self.ev_kmp_visibility_update.send_default();
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
                    visibilities[selected as usize] = all_visible;
                    self.ev_kmp_visibility_update.send_default();
                }
            });
        });
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
            ui.label(format!("Path {i}"));
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
