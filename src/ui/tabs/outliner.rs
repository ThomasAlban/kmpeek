use super::UiSubSection;
use crate::{
    ui::{
        settings::AppSettings,
        util::{view_icon_btn, Icons},
    },
    viewer::{
        edit::select::Selected,
        kmp::{
            components::{
                AreaPoint, CannonPoint, EnemyPathMarker, ItemPathMarker, KmpCamera, Object,
                RespawnPoint, StartPoint,
            },
            path::{EnemyPathGroups, ItemPathGroups},
            sections::{KmpEditMode, KmpModelSections},
            KmpVisibilityUpdate,
        },
    },
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{collapsing_header::CollapsingState, Align, Color32, Layout, Ui};

#[derive(SystemParam)]
pub struct ShowOutlinerTab<'w, 's> {
    keys: Res<'w, Input<KeyCode>>,
    edit_mode: ResMut<'w, KmpEditMode>,

    start_points: Query<'w, 's, Entity, With<StartPoint>>,
    enemy_paths: Query<'w, 's, Entity, With<EnemyPathMarker>>,
    item_paths: Query<'w, 's, Entity, With<ItemPathMarker>>,
    respawn_points: Query<'w, 's, Entity, With<RespawnPoint>>,
    objects: Query<'w, 's, Entity, With<Object>>,
    areas: Query<'w, 's, Entity, With<AreaPoint>>,
    cameras: Query<'w, 's, Entity, With<KmpCamera>>,
    cannon_points: Query<'w, 's, Entity, With<CannonPoint>>,

    enemy_groups: Option<Res<'w, EnemyPathGroups>>,
    item_groups: Option<Res<'w, ItemPathGroups>>,
    commands: Commands<'w, 's>,

    q_visibility: Query<'w, 's, &'static mut Visibility>,
    q_selected: Query<'w, 's, Entity, With<Selected>>,
    ev_kmp_visibility_update: EventWriter<'w, KmpVisibilityUpdate>,
    settings: ResMut<'w, AppSettings>,
    link_visibilities: Local<'s, bool>,
}
impl UiSubSection for ShowOutlinerTab<'_, '_> {
    fn show(&mut self, ui: &mut bevy_egui::egui::Ui) {
        *self.link_visibilities = true;

        let start_points = self.start_points.iter().collect::<Vec<Entity>>();

        let enemy_paths = self.enemy_paths.iter().collect::<Vec<Entity>>();
        let enemy_groups = self.enemy_groups.as_ref().map(|e| e.0.clone());
        let item_paths = self.item_paths.iter().collect::<Vec<Entity>>();
        let item_groups = self.item_groups.as_ref().map(|e| e.0.clone());

        // todo: checkpoints
        let checkpoints: Vec<Entity> = vec![];
        let checkpoint_paths: Option<Vec<Vec<Entity>>> = None;

        let respawn_points = self.respawn_points.iter().collect::<Vec<Entity>>();
        let objects = self.objects.iter().collect::<Vec<Entity>>();
        let areas = self.areas.iter().collect::<Vec<Entity>>();
        let cameras = self.cameras.iter().collect::<Vec<Entity>>();
        let cannon_points = self.cannon_points.iter().collect::<Vec<Entity>>();
        // todo: battle finish points
        let battle_finish_points: Vec<Entity> = vec![];

        use KmpModelSections::*;

        self.show_point_outliner(ui, StartPoints, &start_points);
        self.show_path_outliner(ui, EnemyPaths, &enemy_paths, &enemy_groups);
        self.show_path_outliner(ui, ItemPaths, &item_paths, &item_groups);
        self.show_path_outliner(ui, Checkpoints, &checkpoints, &checkpoint_paths);
        self.show_point_outliner(ui, RespawnPoints, &respawn_points);
        self.show_point_outliner(ui, Objects, &objects);
        self.show_point_outliner(ui, Areas, &areas);
        self.show_point_outliner(ui, Cameras, &cameras);
        self.show_point_outliner(ui, CannonPoints, &cannon_points);
        self.show_point_outliner(ui, BattleFinishPoints, &battle_finish_points);
    }
}
impl ShowOutlinerTab<'_, '_> {
    const ICON_SIZE: f32 = 14.;
    fn show_point_outliner(
        &mut self,
        ui: &mut Ui,
        selected: KmpModelSections,
        entities: &[Entity],
    ) {
        self.show_header(ui, selected, entities, false);
    }
    fn show_path_outliner(
        &mut self,
        ui: &mut Ui,
        selected: KmpModelSections,
        entities: &[Entity],
        group_info: &Option<Vec<Vec<Entity>>>,
    ) {
        CollapsingState::load_with_default_open(
            ui.ctx(),
            format!("{}_outliner", selected).into(),
            false,
        )
        .show_header(ui, |ui| {
            self.show_header(ui, selected, entities, true);
        })
        .body(|ui| {
            if let Some(groups) = group_info {
                for (i, entities) in groups.iter().enumerate() {
                    self.show_path(ui, i, entities, Icons::SECTION_COLORS[selected as usize]);
                }
            }
        });
    }
    fn show_header(
        &mut self,
        ui: &mut Ui,
        selected: KmpModelSections,
        entities: &[Entity],
        path: bool,
    ) {
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
            if ui
                .selectable_value(current, selected, selected.to_string())
                .clicked()
                && *self.link_visibilities
            {
                *visibilities = [false; 10];
                visibilities[selected as usize] = true;
                self.ev_kmp_visibility_update.send_default();
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let mut all_visible = entities
                    .iter()
                    .all(|e| self.q_visibility.get(*e) == Ok(&Visibility::Visible));
                if view_icon_btn(ui, &mut all_visible).changed() {
                    visibilities[selected as usize] = all_visible;
                    self.ev_kmp_visibility_update.send_default();
                }
            });
        });
    }
    fn show_path(&mut self, ui: &mut Ui, i: usize, entities: &[Entity], color: Color32) {
        ui.horizontal(|ui| {
            ui.add_space(10.);
            ui.add_sized(
                [Self::ICON_SIZE, Self::ICON_SIZE],
                Icons::path(ui.ctx(), Self::ICON_SIZE).tint(color),
            );
            ui.label(format!("Path {i}"));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                view_icon_btn(ui, &mut true);
            });
        });
    }
}
