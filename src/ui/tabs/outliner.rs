use crate::{
    ui::{
        keybinds::ModifiersPressed,
        util::{view_icon_btn, Icons},
    },
    util::BoolToVisibility,
    viewer::{
        edit::select::Selected,
        kmp::{
            components::{
                AreaPoint, BattleFinishPoint, CannonPoint, Checkpoint, EnemyPathPoint, ItemPathPoint, KmpCamera,
                Object, RespawnPoint, RoutePoint, StartPoint, TrackInfo,
            },
            path::{PathGroup, PathGroups},
            sections::{change_kmp_edit_mode, KmpEditMode, KmpSection, ToKmpSection},
            SetSectionVisibility,
        },
    },
};
use bevy::{
    ecs::system::{SystemParam, SystemState},
    prelude::*,
};
use bevy_egui::egui::{self, collapsing_header::CollapsingState, Align, Color32, Layout, Ui};

pub fn show_outliner_tab(ui: &mut Ui, world: &mut World) {
    show_track_info_outliner(ui, world);
    show_point_outliner::<StartPoint>(ui, world);
    show_path_outliner::<EnemyPathPoint>(ui, world);
    show_path_outliner::<ItemPathPoint>(ui, world);
    show_path_outliner::<Checkpoint>(ui, world);
    show_point_outliner::<RespawnPoint>(ui, world);
    show_point_outliner::<Object>(ui, world);
    show_path_outliner::<RoutePoint>(ui, world);
    show_point_outliner::<AreaPoint>(ui, world);
    show_point_outliner::<KmpCamera>(ui, world);
    show_point_outliner::<CannonPoint>(ui, world);
    show_point_outliner::<BattleFinishPoint>(ui, world);
}

const ICON_SIZE: f32 = 14.;

fn show_track_info_outliner(ui: &mut Ui, world: &mut World) {
    ui.horizontal(|ui| {
        ui.add_space(18.);
        ui.add_sized(
            [ICON_SIZE, ICON_SIZE],
            Icons::track_info(ui.ctx(), ICON_SIZE).tint(Icons::SECTION_COLORS[KmpSection::TrackInfo as usize]),
        );
        if ui
            .selectable_label(world.contains_resource::<KmpEditMode<TrackInfo>>(), "Track Info")
            .clicked()
        {
            change_kmp_edit_mode::<TrackInfo>(world);
        }
    });
}

fn show_point_outliner<T: Component + ToKmpSection>(ui: &mut Ui, world: &mut World) {
    show_header::<T>(ui, world, false);
}

fn show_path_outliner<T: Component + ToKmpSection>(ui: &mut Ui, world: &mut World) {
    CollapsingState::load_with_default_open(ui.ctx(), ui.next_auto_id(), false)
        .show_header(ui, |ui| {
            show_header::<T>(ui, world, true);
        })
        .body(|ui| {
            let mut paths_to_show = Vec::new();
            if let Some(groups) = world.get_resource::<PathGroups<T>>() {
                for (i, pathgroup) in groups.groups.iter().enumerate() {
                    paths_to_show.push((i, pathgroup.clone()));
                }
            }
            for (i, pathgroup) in paths_to_show {
                show_path(
                    ui,
                    world,
                    i,
                    pathgroup.clone(),
                    Icons::SECTION_COLORS[T::to_kmp_section() as usize],
                );
            }
        });
}

fn show_path(ui: &mut Ui, world: &mut World, i: usize, pathgroup: PathGroup, color: Color32) {
    let mut all_visible = if !pathgroup.path.is_empty() {
        pathgroup
            .path
            .iter()
            .all(|e| world.query::<&Visibility>().get(world, *e) == Ok(&Visibility::Visible))
    } else {
        false
    };
    ui.horizontal(|ui| {
        ui.add_space(10.);
        ui.add_sized([ICON_SIZE, ICON_SIZE], Icons::path(ui.ctx(), ICON_SIZE).tint(color));
        let label = ui.add(
            egui::Label::new(format!("Path {i}"))
                .selectable(false)
                .sense(egui::Sense::click()),
        );
        if label.clicked() {
            let keys = world.resource::<ButtonInput<KeyCode>>();
            if !keys.shift_pressed() {
                // deselect everything
                let entities: Vec<_> = world.query_filtered::<Entity, With<Selected>>().iter(world).collect();
                for e in entities {
                    world.entity_mut(e).remove::<Selected>();
                }
            }
            for e in pathgroup.path.iter() {
                world.entity_mut(*e).insert(Selected);
            }
        }
        let view_btn_response = ui
            .with_layout(Layout::right_to_left(Align::Center), |ui| {
                view_icon_btn(ui, &mut all_visible)
            })
            .inner;

        if view_btn_response.changed() {
            for e in pathgroup.path.iter() {
                let Ok(mut visibility) = world.query::<&mut Visibility>().get_mut(world, *e) else {
                    continue;
                };
                *visibility = all_visible.to_visibility();
            }
        }
    });
}

fn show_header<T: Component + ToKmpSection>(ui: &mut Ui, world: &mut World, path: bool) {
    let entities: Vec<_> = world.query_filtered::<Entity, With<T>>().iter(world).collect();
    let cur_mode = world.contains_resource::<KmpEditMode<T>>();

    ui.horizontal(|ui| {
        if !path {
            ui.add_space(18.);
        }
        ui.add_sized(
            [ICON_SIZE, ICON_SIZE],
            if path {
                Icons::path_group(ui.ctx(), ICON_SIZE)
            } else {
                Icons::cube_group(ui.ctx(), ICON_SIZE)
            }
            .tint(Icons::SECTION_COLORS[T::to_kmp_section() as usize]),
        );
        if ui.selectable_label(cur_mode, T::to_kmp_section().to_string()).clicked() {
            change_kmp_edit_mode::<T>(world);
        }

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let mut all_visible = if !entities.is_empty() {
                entities
                    .iter()
                    .all(|e| world.query::<&Visibility>().get(world, *e) == Ok(&Visibility::Visible))
            } else {
                false
            };
            if view_icon_btn(ui, &mut all_visible).changed() {
                world.send_event(SetSectionVisibility::<T>::new(all_visible));
            }
        });
    });
}
