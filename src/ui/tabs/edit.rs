use std::{
    fmt::Display,
    ops::{AddAssign, Sub, SubAssign},
};

use super::UiSubSection;
use crate::{
    ui::util::{
        combobox_enum, framed_collapsing_header,
        multi_edit::{checkbox_multi_edit, combobox_enum_multi_edit, drag_value_multi_edit, map, rotation_multi_edit},
        DragSpeed,
    },
    viewer::{
        edit::select::Selected,
        kmp::{
            checkpoints::GetSelectedCheckpoints,
            components::{
                AreaKind, AreaPoint, BattleFinishPoint, CannonPoint, Checkpoint, EnemyPathPoint, ItemPathPoint,
                KmpCamera, Object, PathOverallStart, RespawnPoint, RoutePoint, StartPoint, TrackInfo,
                TransformEditOptions,
            },
            path::{PathType, RecalcPaths},
            routes::GetRouteStart,
            sections::KmpEditMode,
        },
    },
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{self, emath::Numeric, Align, Checkbox, DragValue, Layout, Response, Ui, WidgetText};

type KmpComponentQuery<'w, 's, C> = Query<'w, 's, (Entity, &'static mut C), With<Selected>>;

#[derive(SystemParam)]
pub struct ShowEditTab<'w, 's> {
    commands: Commands<'w, 's>,
    track_info_mode: Option<Res<'w, KmpEditMode<TrackInfo>>>,
    track_info: Option<ResMut<'w, TrackInfo>>,

    q_transform: Query<'w, 's, (Entity, &'static mut Transform), With<Selected>>,
    q_transform_opts: Query<'w, 's, &'static TransformEditOptions>,

    q_start_pt: KmpComponentQuery<'w, 's, StartPoint>,
    q_enemy_pt: KmpComponentQuery<'w, 's, EnemyPathPoint>,
    q_item_pt: KmpComponentQuery<'w, 's, ItemPathPoint>,
    q_cp: GetSelectedCheckpoints<'w, 's>,
    q_respawn_point: KmpComponentQuery<'w, 's, RespawnPoint>,
    q_object: KmpComponentQuery<'w, 's, Object>,
    q_route_pt: KmpComponentQuery<'w, 's, RoutePoint>,
    q_area: KmpComponentQuery<'w, 's, AreaPoint>,
    q_camera: KmpComponentQuery<'w, 's, KmpCamera>,
    q_cannon_point: KmpComponentQuery<'w, 's, CannonPoint>,
    q_battle_finish_point: KmpComponentQuery<'w, 's, BattleFinishPoint>,

    q_path_start: PathStartQuery<'w, 's>,
    ev_recalc_paths: EventWriter<'w, RecalcPaths>,
    get_route_start: GetRouteStart<'w, 's>,
}
impl UiSubSection for ShowEditTab<'_, '_> {
    fn show(&mut self, ui: &mut Ui) {
        if let Some(track_info) = &mut self.track_info {
            if self.track_info_mode.is_some() {
                framed_collapsing_header("Track Info", ui, |ui| {
                    edit_row(ui, "Track Type", false, |ui| {
                        combobox_enum(ui, &mut track_info.track_type, None);
                    });
                    edit_row(ui, "Lap Count", true, |ui| {
                        ui.add(DragValue::new(&mut track_info.lap_count).speed(DragSpeed::Slow))
                    });
                    edit_row(ui, "Speed Mod", true, |ui| {
                        ui.add(DragValue::new(&mut track_info.speed_mod).speed(DragSpeed::Slow))
                    });
                    edit_spacing(ui);
                    edit_row(ui, "Lens Flare Colour", false, |ui| {
                        ui.color_edit_button_srgba_unmultiplied(&mut track_info.lens_flare_color);
                    });
                    edit_row(ui, "Lens Flare Flashing", false, |ui| {
                        ui.add(Checkbox::without_text(&mut track_info.lens_flare_flashing));
                    });
                    edit_spacing(ui);
                    edit_row(ui, "First Player Pos", false, |ui| {
                        combobox_enum(ui, &mut track_info.first_player_pos, None);
                    });
                    edit_row(ui, "Narrow Player Spacing", false, |ui| {
                        ui.add(Checkbox::without_text(&mut track_info.narrow_player_spacing));
                    });
                });
                edit_spacing(ui);
            }
        }

        edit_component(ui, "Transform", self.q_transform.iter_mut(), |ui, items| {
            let transform_opts: Vec<_> = self.q_transform_opts.iter_many(items.iter().map(|x| x.0)).collect();
            let all_hide_rot = transform_opts.iter().all(|x| x.hide_rotation);
            let all_hide_y_tr = transform_opts.iter().all(|x| x.hide_y_translation);

            drag_value_edit_row(ui, "Translation X", DragSpeed::Fast, map!(items, translation.x));
            if !all_hide_y_tr {
                drag_value_edit_row(ui, "Y", DragSpeed::Fast, map!(items, translation.y));
            }
            drag_value_edit_row(ui, "Z", DragSpeed::Fast, map!(items, translation.z));

            if !all_hide_rot {
                edit_spacing(ui);
                rotation_multi_edit(ui, map!(items,), |ui, rots| {
                    let [x, y, z] = vec3_drag_value_edit_row(ui, "Rotation", DragSpeed::Slow, rots);
                    (x, y, z)
                });
            }
        });

        edit_component(ui, "Start Point", self.q_start_pt.iter_mut(), |ui, items| {
            drag_value_edit_row(ui, "Player Index", DragSpeed::Slow, map!(items, player_index));
        });

        edit_component(ui, "Enemy Point", self.q_enemy_pt.iter_mut(), |ui, items| {
            drag_value_edit_row(ui, "Leniency", DragSpeed::Slow, map!(items, leniency));
            combobox_edit_row(ui, "Setting 1", map!(items, setting_1));
            combobox_edit_row(ui, "Setting 2", map!(items, setting_2));
            drag_value_edit_row(ui, "Setting 3", DragSpeed::Slow, map!(items, setting_3));
            edit_spacing(ui);
            path_start_btn(
                ui,
                &mut self.commands,
                &mut self.q_path_start,
                &mut self.ev_recalc_paths,
                items,
                PathType::Enemy,
            );
        });

        edit_component(ui, "Item Point", self.q_item_pt.iter_mut(), |ui, items| {
            drag_value_edit_row(ui, "Bullet Control", DragSpeed::Slow, map!(items, bullet_control));
            edit_spacing(ui);
            combobox_edit_row(ui, "Bullet Height", map!(items, bullet_height));
            checkbox_edit_row(ui, "Bullet Can't Drop", map!(items, bullet_cant_drop));
            checkbox_edit_row(ui, "Low Shell Priority", map!(items, low_shell_priority));
            edit_spacing(ui);
            path_start_btn(
                ui,
                &mut self.commands,
                &mut self.q_path_start,
                &mut self.ev_recalc_paths,
                items,
                PathType::Item,
            );
        });

        edit_component(ui, "Checkpoint", self.q_cp.get(), |ui, items| {
            combobox_edit_row(ui, "Type", map!(items, kind));
            path_start_btn(
                ui,
                &mut self.commands,
                &mut self.q_path_start,
                &mut self.ev_recalc_paths,
                items,
                PathType::Checkpoint { right: false },
            );
        });

        edit_component(ui, "Respawn Point", self.q_respawn_point.iter_mut(), |ui, items| {
            drag_value_edit_row(ui, "Sound Trigger", DragSpeed::Slow, map!(items, sound_trigger));
        });

        edit_component(ui, "Object", self.q_object.iter_mut(), |ui, items| {
            vec3_drag_value_edit_row(ui, "Scale", DragSpeed::Fast, map!(items, scale));
            edit_spacing(ui);
            drag_value_edit_row(ui, "ID", DragSpeed::Slow, map!(items, object_id));
            edit_spacing(ui);
            for i in 0..8 {
                drag_value_edit_row(
                    ui,
                    format!("Setting {}", i + 1),
                    DragSpeed::Slow,
                    items.iter_mut().map(|x| &mut x.1.settings[i]),
                );
            }
        });

        edit_component(
            ui,
            "Route Settings",
            self.get_route_start
                .get_multiple_mut(self.q_route_pt.iter().map(|x| x.0)),
            |ui, items| {
                checkbox_edit_row(ui, "Smooth Motion", map!(items, smooth_motion));
                combobox_edit_row(ui, "Loop Style", map!(items, loop_style));
            },
        );

        edit_component(ui, "Route Point", self.q_route_pt.iter_mut(), |ui, items| {
            drag_value_edit_row(ui, "Settings", DragSpeed::Slow, map!(items, settings));
            drag_value_edit_row(
                ui,
                "Additional Settings",
                DragSpeed::Slow,
                map!(items, additional_settings),
            );
        });

        edit_component(ui, "Area", self.q_area.iter_mut(), |ui, items| {
            vec3_drag_value_edit_row(ui, "Scale", DragSpeed::Slow, map!(items, scale));
            edit_spacing(ui);
            combobox_edit_row(ui, "Shape", map!(items, shape));
            drag_value_edit_row(ui, "Priority", DragSpeed::Slow, map!(items, priority));
            combobox_edit_row(ui, "Type", map!(items, kind));

            // for now, area type UI settings will only work when 1 point is selected
            if let Some(item) = items.iter_mut().next() {
                match &mut item.1.kind {
                    AreaKind::Camera { cam_index } => {
                        edit_row(ui, "Camera Index", true, |ui| {
                            ui.add(DragValue::new(cam_index).speed(DragSpeed::Slow));
                        });
                    }
                    AreaKind::EnvEffect(env_effect_obj) => {
                        edit_row(ui, "Env Effect Object", true, |ui| {
                            combobox_enum(ui, env_effect_obj, None);
                        });
                    }
                    AreaKind::FogEffect { bfg_entry, setting_2 } => {
                        edit_row(ui, "BFG Entry", true, |ui| {
                            ui.add(DragValue::new(bfg_entry).speed(DragSpeed::Slow));
                        });
                        edit_row(ui, "Setting 2", true, |ui| {
                            ui.add(DragValue::new(setting_2).speed(DragSpeed::Slow));
                        });
                    }
                    // TODO - abstract route IDs away
                    AreaKind::MovingRoad { route_id } => {
                        edit_row(ui, "Route ID", true, |ui| {
                            ui.add(DragValue::new(route_id).speed(DragSpeed::Slow));
                        });
                    }
                    AreaKind::MinimapControl { setting_1, setting_2 } => {
                        edit_row(ui, "Setting 1", true, |ui| {
                            ui.add(DragValue::new(setting_1).speed(DragSpeed::Slow));
                        });
                        edit_row(ui, "Setting 2", true, |ui| {
                            ui.add(DragValue::new(setting_2).speed(DragSpeed::Slow));
                        });
                    }
                    AreaKind::BloomEffect { bblm_file, fade_time } => {
                        edit_row(ui, "BBLM File", true, |ui| {
                            ui.add(DragValue::new(bblm_file).speed(DragSpeed::Slow));
                        });
                        edit_row(ui, "Fade Time", true, |ui| {
                            ui.add(DragValue::new(fade_time).speed(DragSpeed::Slow));
                        });
                    }
                    AreaKind::ObjectGroup { group_id } | AreaKind::ObjectUnload { group_id } => {
                        edit_row(ui, "Group ID", true, |ui| {
                            ui.add(DragValue::new(group_id).speed(DragSpeed::Slow));
                        });
                    }
                    // other types of area don't have any settings
                    _ => {}
                }
            }
            edit_spacing(ui);
            checkbox_edit_row(ui, "Always Show Area", map!(items, show_area));
        });

        edit_component(ui, "Camera", self.q_camera.iter_mut(), |ui, items| {
            combobox_edit_row(ui, "Type", map!(items, kind));
            edit_spacing(ui);
            drag_value_edit_row(ui, "Next Index", DragSpeed::Slow, map!(items, next_index));
            drag_value_edit_row(ui, "Route Index", DragSpeed::Slow, map!(items, route));
            edit_spacing(ui);
            drag_value_edit_row(ui, "Time", DragSpeed::Slow, map!(items, time));
            edit_spacing(ui);
            drag_value_edit_row(ui, "Point Speed", DragSpeed::Slow, map!(items, point_velocity));
            drag_value_edit_row(ui, "Zoom Speed", DragSpeed::Slow, map!(items, zoom_velocity));
            drag_value_edit_row(ui, "View Speed", DragSpeed::Slow, map!(items, view_velocity));
            edit_spacing(ui);
            drag_value_edit_row(ui, "Zoom Start", DragSpeed::Slow, map!(items, zoom_start));
            drag_value_edit_row(ui, "Zoom End", DragSpeed::Slow, map!(items, zoom_end));
            edit_spacing(ui);
            vec3_drag_value_edit_row(ui, "View Start", DragSpeed::Slow, map!(items, view_start));
            edit_spacing(ui);
            vec3_drag_value_edit_row(ui, "View End", DragSpeed::Slow, map!(items, view_end));
            edit_spacing(ui);
            drag_value_edit_row(ui, "Shake (?)", DragSpeed::Slow, map!(items, shake));
            drag_value_edit_row(ui, "Start (?)", DragSpeed::Slow, map!(items, start));
            drag_value_edit_row(ui, "Movie (?)", DragSpeed::Slow, map!(items, movie));
        });

        edit_component(ui, "Cannon Point", self.q_cannon_point.iter_mut(), |ui, items| {
            combobox_edit_row(ui, "Shoot Effect", map!(items, shoot_effect));
        });

        edit_component(
            ui,
            "Battle Finish Point",
            self.q_battle_finish_point.iter_mut(),
            |_ui, _items| {},
        );
    }
}

type PathStartQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        Has<EnemyPathPoint>,
        Has<ItemPathPoint>,
        Has<Checkpoint>,
        Has<RoutePoint>,
    ),
    With<PathOverallStart>,
>;

fn path_start_btn<T>(
    ui: &mut Ui,
    commands: &mut Commands,
    q_path_start: &mut PathStartQuery,
    ev_recalc_paths: &mut EventWriter<RecalcPaths>,
    items: &[(Entity, T)],
    path_type: PathType,
) {
    ui.with_layout(Layout::top_down(Align::Center), |ui| {
        if items.len() != 1 {
            ui.disable();
        }
        if ui.button("Set As Path Start").clicked() && items.len() == 1 {
            for e in q_path_start
                .iter()
                .filter(|x| match path_type {
                    PathType::Enemy => x.1,
                    PathType::Item => x.2,
                    PathType::Checkpoint { .. } => x.3,
                    PathType::Route => x.4,
                })
                .map(|x| x.0)
            {
                commands.entity(e).remove::<PathOverallStart>();
            }
            commands.entity(items[0].0).insert(PathOverallStart);
            let ev = match path_type {
                PathType::Enemy => RecalcPaths::enemy(),
                PathType::Item => RecalcPaths::item(),
                PathType::Checkpoint { .. } => RecalcPaths::cp(),
                PathType::Route => RecalcPaths::route(),
            };
            ev_recalc_paths.send(ev);
        }
    });
}

fn edit_component_title(name: impl Into<String>, num: usize) -> String {
    let name = name.into();
    if num > 1 {
        format!("{} ({})", name, num)
    } else {
        name
    }
}

fn edit_component<'a, T: 'a + PartialEq + Clone, R>(
    ui: &mut Ui,
    title: &'static str,
    items: impl IntoIterator<Item = (Entity, Mut<'a, T>)>,
    add_body: impl FnOnce(&mut Ui, &mut [(Entity, T)]) -> R,
) {
    let mut len = 0;
    edit_many_mut(items, |items| {
        len = items.len();
        let title = edit_component_title(title, len);
        framed_collapsing_header(title, ui, |ui| add_body(ui, items));
    });
    if len > 0 {
        edit_spacing(ui);
    }
}

fn edit_many_mut<'a, T: 'a + PartialEq + Clone>(
    items: impl IntoIterator<Item = (Entity, Mut<'a, T>)>,
    contents: impl FnOnce(&mut [(Entity, T)]),
) {
    let mut items: Vec<(Entity, Mut<T>)> = items.into_iter().collect();
    if items.is_empty() {
        return;
    };

    let mut cloned: Vec<(Entity, T)> = items.iter().map(|x| (x.0, (*x.1).clone())).collect();
    contents(&mut cloned);
    for (item, other) in items.iter_mut().zip(cloned.iter()) {
        item.1.set_if_neq(other.1.clone());
    }
}

pub fn edit_spacing(ui: &mut Ui) {
    ui.vertical(|ui| ui.add_space(3.));
}

pub fn drag_value_edit_row<'a, T: 'a + Clone + PartialEq + Numeric + Sub<Output = T> + AddAssign<T> + SubAssign<T>>(
    ui: &mut Ui,
    label: impl Into<WidgetText>,
    speed: DragSpeed,
    items: impl IntoIterator<Item = &'a mut T>,
) -> Response {
    edit_row(ui, label, true, |ui| drag_value_multi_edit(ui, speed, items))
}

pub fn vec3_drag_value_edit_row<'a>(
    ui: &mut Ui,
    label: impl Into<String>,
    speed: DragSpeed,
    items: impl IntoIterator<Item = &'a mut Vec3>,
) -> [Response; 3] {
    let mut items: Vec<_> = items.into_iter().collect();
    let x_label = format!("{} X", label.into());
    [
        edit_row(ui, x_label, true, |ui| {
            drag_value_multi_edit(ui, speed, items.iter_mut().map(|x| &mut x.x))
        }),
        edit_row(ui, "Y", true, |ui| {
            drag_value_multi_edit(ui, speed, items.iter_mut().map(|x| &mut x.y))
        }),
        edit_row(ui, "Z", true, |ui| {
            drag_value_multi_edit(ui, speed, items.iter_mut().map(|x| &mut x.z))
        }),
    ]
}

pub fn combobox_edit_row<'a, T: 'a + strum::IntoEnumIterator + Display + PartialEq + Clone>(
    ui: &mut Ui,
    label: impl Into<WidgetText>,
    items: impl IntoIterator<Item = &'a mut T>,
) -> Response {
    edit_row(ui, label, true, |ui| combobox_enum_multi_edit(ui, None, items))
}

pub fn checkbox_edit_row<'a>(
    ui: &mut Ui,
    label: impl Into<WidgetText>,
    items: impl IntoIterator<Item = &'a mut bool>,
) -> Response {
    edit_row(ui, label, false, |ui| checkbox_multi_edit(ui, items))
}

pub fn edit_row<R>(
    ui: &mut Ui,
    label: impl Into<WidgetText>,
    justified: bool,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> R {
    ui.horizontal(|ui| {
        ui.columns(2, |ui| {
            ui[0].with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add(egui::Label::new(label).truncate());
            });
            if justified {
                ui[1].centered_and_justified(add_contents)
            } else {
                ui[1].with_layout(Layout::left_to_right(egui::Align::Center), add_contents)
            }
            .inner
        })
    })
    .inner
}
