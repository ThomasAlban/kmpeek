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
                AreaPoint, BattleFinishPoint, CannonPoint, Checkpoint, EnemyPathPoint, ItemPathPoint, KmpCamera,
                Object, RespawnPoint, StartPoint, TrackInfo,
            },
            path::{PathGroup, PathGroups},
            sections::{KmpEditMode, KmpEditModeOptions, KmpSection, ToKmpSection},
        },
    },
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{self, collapsing_header::CollapsingState, Align, Color32, Layout, Ui};

#[derive(SystemParam)]
pub struct ShowOutlinerTab<'w, 's> {
    p: ParamSet<
        'w,
        's,
        (
            ParamSet<
                'w,
                's,
                (
                    KmpOutliner<'w, 's, TrackInfo>,
                    KmpOutliner<'w, 's, StartPoint>,
                    KmpOutliner<'w, 's, EnemyPathPoint>,
                    KmpOutliner<'w, 's, ItemPathPoint>,
                    KmpOutliner<'w, 's, Checkpoint>,
                ),
            >,
            ParamSet<
                'w,
                's,
                (
                    KmpOutliner<'w, 's, RespawnPoint>,
                    KmpOutliner<'w, 's, Object>,
                    KmpOutliner<'w, 's, AreaPoint>,
                    KmpOutliner<'w, 's, KmpCamera>,
                    KmpOutliner<'w, 's, CannonPoint>,
                    KmpOutliner<'w, 's, BattleFinishPoint>,
                ),
            >,
        ),
    >,
}
impl UiSubSection for ShowOutlinerTab<'_, '_> {
    fn show(&mut self, ui: &mut Ui) {
        self.p.p0().p0().show_track_info_header(ui);
        self.p.p0().p1().show_point_outliner(ui);
        self.p.p0().p2().show_path_outliner(ui);
        self.p.p0().p3().show_path_outliner(ui);
        self.p.p0().p4().show_path_outliner(ui);
        self.p.p1().p0().show_point_outliner(ui);
        self.p.p1().p1().show_point_outliner(ui);
        self.p.p1().p2().show_point_outliner(ui);
        self.p.p1().p3().show_point_outliner(ui);
        self.p.p1().p4().show_point_outliner(ui);
        self.p.p1().p5().show_point_outliner(ui);
    }
}

#[derive(SystemParam)]
pub struct KmpOutliner<'w, 's, T: Component + ToKmpSection> {
    q: Query<'w, 's, Entity, With<T>>,
    path_groups: Option<Res<'w, PathGroups<T>>>,
    mode: Option<Res<'w, KmpEditMode<T>>>,
    kmp_visibility: ResMut<'w, KmpVisibility>,
    mode_opts: KmpEditModeOptions<'w, 's>,
    q_visibility: Query<'w, 's, &'static mut Visibility>,
    keys: Res<'w, ButtonInput<KeyCode>>,
    q_selected: Query<'w, 's, Entity, With<Selected>>,
    commands: Commands<'w, 's>,
}
impl<T: Component + ToKmpSection> KmpOutliner<'_, '_, T> {
    const ICON_SIZE: f32 = 14.;

    fn show_point_outliner(&mut self, ui: &mut Ui) {
        self.show_header(ui, false);
    }
    fn show_path_outliner(&mut self, ui: &mut Ui) {
        CollapsingState::load_with_default_open(ui.ctx(), ui.next_auto_id(), false)
            .show_header(ui, |ui| {
                self.show_header(ui, true);
            })
            .body(|ui| {
                let mut paths_to_show = Vec::new();
                if let Some(groups) = &self.path_groups {
                    for (i, pathgroup) in groups.groups.iter().enumerate() {
                        paths_to_show.push((i, pathgroup.clone()));
                    }
                }
                for (i, pathgroup) in paths_to_show {
                    self.show_path(
                        ui,
                        i,
                        pathgroup.clone(),
                        Icons::SECTION_COLORS[T::to_kmp_section() as usize],
                    );
                }
            });
    }
    fn show_header(&mut self, ui: &mut Ui, path: bool) {
        let entities: Vec<_> = self.q.iter().collect();
        let cur_mode = self.mode.is_some();
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
                .tint(Icons::SECTION_COLORS[T::to_kmp_section() as usize]),
            );
            if ui.selectable_label(cur_mode, T::to_kmp_section().to_string()).clicked() {
                visibilities.0 = [false; 10];
                visibilities.0[T::to_kmp_section() as usize] = true;
                self.mode_opts.change_mode::<T>();
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
                    visibilities.0[T::to_kmp_section() as usize] = all_visible;
                }
            });
        });
        self.kmp_visibility.set_if_neq(visibilities);
    }
    fn show_path(&mut self, ui: &mut Ui, i: usize, pathgroup: PathGroup, color: Color32) {
        let mut all_visible = if !pathgroup.path.is_empty() {
            pathgroup
                .path
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
                for e in pathgroup.path.iter() {
                    self.commands.entity(*e).insert(Selected);
                }
            }
            let view_btn_response = ui
                .with_layout(Layout::right_to_left(Align::Center), |ui| {
                    view_icon_btn(ui, &mut all_visible)
                })
                .inner;

            if view_btn_response.changed() {
                for e in pathgroup.path.iter() {
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
    fn show_track_info_header(&mut self, ui: &mut Ui) {
        let mut visibilities = self.kmp_visibility.clone();
        ui.horizontal(|ui| {
            ui.add_space(18.);
            ui.add_sized(
                [Self::ICON_SIZE, Self::ICON_SIZE],
                Icons::track_info(ui.ctx(), Self::ICON_SIZE)
                    .tint(Icons::SECTION_COLORS[KmpSection::TrackInfo as usize]),
            );
            if ui.selectable_label(self.mode.is_some(), "Track Info").clicked() {
                visibilities.0 = [false; 10];
                self.mode_opts.change_mode::<TrackInfo>();
            }
        });
        self.kmp_visibility.set_if_neq(visibilities);
    }
}
