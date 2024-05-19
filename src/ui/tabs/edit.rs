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
            components::{
                AreaKind, AreaPoint, BattleFinishPoint, CannonPoint, CheckpointLeft, CheckpointRight, EnemyPathPoint,
                HideRotation, ItemPathPoint, KmpCamera, Object, RespawnPoint, StartPoint, TrackInfo,
            },
            path::RecalculatePaths,
            sections::{KmpEditMode, KmpSections},
        },
    },
};
use bevy::{
    ecs::{
        entity::{EntityHashMap, EntityHashSet},
        system::SystemParam,
    },
    prelude::*,
};
use bevy_egui::egui::{self, emath::Numeric, Checkbox, DragValue, Layout, Response, Ui, WidgetText};

#[derive(SystemParam)]
pub struct ShowEditTab<'w, 's> {
    kmp_edit_mode: Res<'w, KmpEditMode>,
    track_info: Option<ResMut<'w, TrackInfo>>,

    q_transform: Query<'w, 's, (&'static mut Transform, Has<HideRotation>), With<Selected>>,

    q_start_point: Query<'w, 's, &'static mut StartPoint, With<Selected>>,
    q_enemy_point: Query<'w, 's, &'static mut EnemyPathPoint, With<Selected>>,
    q_item_point: Query<'w, 's, &'static mut ItemPathPoint, With<Selected>>,
    q_cp_left: Query<'w, 's, (&'static mut CheckpointLeft, Entity, Has<Selected>)>,
    q_cp_right: Query<'w, 's, &'static mut CheckpointRight, With<Selected>>,
    q_respawn_point: Query<'w, 's, &'static mut RespawnPoint, With<Selected>>,
    q_object: Query<'w, 's, &'static mut Object, With<Selected>>,
    q_area: Query<'w, 's, &'static mut AreaPoint, With<Selected>>,
    q_camera: Query<'w, 's, &'static mut KmpCamera, With<Selected>>,
    q_cannon_point: Query<'w, 's, &'static mut CannonPoint, With<Selected>>,
    q_battle_finish_point: Query<'w, 's, &'static mut BattleFinishPoint, With<Selected>>,

    ev_recalc_paths: EventWriter<'w, RecalculatePaths>,
}
impl UiSubSection for ShowEditTab<'_, '_> {
    fn show(&mut self, ui: &mut Ui) {
        if let Some(track_info) = &mut self.track_info {
            if self.kmp_edit_mode.0 == KmpSections::TrackInfo {
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

        if !self.q_transform.is_empty() {
            let mut tr: Vec<_> = self.q_transform.iter().map(|x| (*x.0, x.1)).collect();
            let title = if tr.len() > 1 {
                format!("Transform ({})", tr.len())
            } else {
                "Transform".to_owned()
            };
            framed_collapsing_header(title, ui, |ui| {
                vec3_drag_value_edit_row(
                    ui,
                    "Translation",
                    DragSpeed::Fast,
                    tr.iter_mut().map(|x| &mut x.0.translation),
                );
                if !tr.iter().all(|x| x.1) {
                    edit_spacing(ui);
                    rotation_multi_edit(ui, tr.iter_mut().map(|x| &mut x.0), |ui, rots| {
                        let [x, y, z] = vec3_drag_value_edit_row(ui, "Rotation", DragSpeed::Slow, rots);
                        (x, y, z)
                    });
                }
            });
            for (mut item, other) in self.q_transform.iter_mut().zip(tr.iter()) {
                if *item.0 != other.0 {
                    *item.0 = other.0;
                    if item.1 {
                        item.0.rotation = Quat::default();
                    }
                }
            }
            edit_spacing(ui);
        }

        edit_component(ui, "Start Point", self.q_start_point.iter_mut(), |ui, items| {
            drag_value_edit_row(ui, "Player Index", DragSpeed::Slow, map!(items, player_index));
        });

        edit_component(ui, "Enemy Point", self.q_enemy_point.iter_mut(), |ui, items| {
            drag_value_edit_row(ui, "Leniency", DragSpeed::Slow, map!(items, leniency));
            combobox_edit_row(ui, "Setting 1", map!(items, setting_1));
            combobox_edit_row(ui, "Setting 2", map!(items, setting_2));
            drag_value_edit_row(ui, "Setting 3", DragSpeed::Slow, map!(items, setting_3));
            edit_spacing(ui);
            let changed = checkbox_edit_row(ui, "Always Path Start", map!(items, path_start_override)).changed();
            if changed {
                self.ev_recalc_paths.send_default();
            }
        });

        edit_component(ui, "Item Point", self.q_item_point.iter_mut(), |ui, items| {
            drag_value_edit_row(ui, "Bullet Control", DragSpeed::Slow, map!(items, bullet_control));
            edit_spacing(ui);
            combobox_edit_row(ui, "Bullet Height", map!(items, bullet_height));
            checkbox_edit_row(ui, "Bullet Can't Drop", map!(items, bullet_cant_drop));
            checkbox_edit_row(ui, "Low Shell Priority", map!(items, low_shell_priority));
            edit_spacing(ui);
            let changed = checkbox_edit_row(ui, "Always Path Start", map!(items, path_start_override)).changed();
            if changed {
                self.ev_recalc_paths.send_default();
            }
        });

        let cp_left_of_right: EntityHashSet = self.q_cp_right.iter().map(|x| x.left).collect();
        let mut cps: EntityHashMap<Mut<CheckpointLeft>> = EntityHashMap::default();
        for (cp_l, e, selected) in self.q_cp_left.iter_mut() {
            if selected || cp_left_of_right.contains(&e) {
                cps.insert(e, cp_l);
            }
        }
        let cp_iter = cps.into_iter().map(|x| x.1);

        edit_component(ui, "Checkpoint", cp_iter, |ui, items| {
            combobox_edit_row(ui, "Type", map!(items, kind));
            let changed = checkbox_edit_row(ui, "Always Path Start", map!(items, path_start_override)).changed();
            if changed {
                self.ev_recalc_paths.send_default();
            }
        });

        edit_component(ui, "Respawn Point", self.q_respawn_point.iter_mut(), |ui, items| {
            drag_value_edit_row(ui, "ID", DragSpeed::Slow, map!(items, id));
            drag_value_edit_row(ui, "Sound Trigger", DragSpeed::Slow, map!(items, sound_trigger));
        });

        edit_component(ui, "Object", self.q_object.iter_mut(), |ui, items| {
            vec3_drag_value_edit_row(ui, "Scale", DragSpeed::Fast, map!(items, scale));
            edit_spacing(ui);
            drag_value_edit_row(ui, "ID", DragSpeed::Slow, map!(items, object_id));
            edit_spacing(ui);
            for i in 0..8 {
                let label = format!("Setting {}", i + 1);
                drag_value_edit_row(ui, label, DragSpeed::Slow, items.iter_mut().map(|x| &mut x.settings[i]));
            }
        });

        edit_component(ui, "Area", self.q_area.iter_mut(), |ui, items| {
            vec3_drag_value_edit_row(ui, "Scale", DragSpeed::Slow, map!(items, scale));
            edit_spacing(ui);
            combobox_edit_row(ui, "Shape", map!(items, shape));
            drag_value_edit_row(ui, "Priority", DragSpeed::Slow, map!(items, priority));
            combobox_edit_row(ui, "Type", map!(items, kind));

            // for now, area type UI settings will only work when 1 point is selected
            if items.len() == 1 {
                match &mut items[0].kind {
                    AreaKind::Camera(cam_index) => {
                        edit_row(ui, "Camera Index", true, |ui| {
                            ui.add(DragValue::new(&mut cam_index.0).speed(DragSpeed::Slow));
                        });
                    }
                    AreaKind::EnvEffect(env_effect_obj) => {
                        edit_row(ui, "Env Effect Object", true, |ui| {
                            combobox_enum(ui, env_effect_obj, None);
                        });
                    }
                    AreaKind::FogEffect(bfg_entry, setting_2) => {
                        edit_row(ui, "BFG Entry", true, |ui| {
                            ui.add(DragValue::new(&mut bfg_entry.0).speed(DragSpeed::Slow));
                        });
                        edit_row(ui, "Setting 2", true, |ui| {
                            ui.add(DragValue::new(&mut setting_2.0).speed(DragSpeed::Slow));
                        });
                    }
                    // TODO - abstract route IDs away
                    AreaKind::MovingRoad(area_route_id) => {
                        edit_row(ui, "Route ID", true, |ui| {
                            ui.add(DragValue::new(&mut area_route_id.0).speed(DragSpeed::Slow));
                        });
                    }
                    AreaKind::MinimapControl(setting_1, setting_2) => {
                        edit_row(ui, "Setting 1", true, |ui| {
                            ui.add(DragValue::new(&mut setting_1.0).speed(DragSpeed::Slow));
                        });
                        edit_row(ui, "Setting 2", true, |ui| {
                            ui.add(DragValue::new(&mut setting_2.0).speed(DragSpeed::Slow));
                        });
                    }
                    AreaKind::BloomEffect(bblm_file, fade_time) => {
                        edit_row(ui, "BBLM File", true, |ui| {
                            ui.add(DragValue::new(&mut bblm_file.0).speed(DragSpeed::Slow));
                        });
                        edit_row(ui, "Fade Time", true, |ui| {
                            ui.add(DragValue::new(&mut fade_time.0).speed(DragSpeed::Slow));
                        });
                    }
                    AreaKind::ObjectGroup(group_id) | AreaKind::ObjectUnload(group_id) => {
                        edit_row(ui, "Group ID", true, |ui| {
                            ui.add(DragValue::new(&mut group_id.0).speed(DragSpeed::Slow));
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
            drag_value_edit_row(ui, "ID", DragSpeed::Slow, map!(items, id));
            combobox_edit_row(ui, "Shoot Effect", map!(items, shoot_effect));
        });

        edit_component(
            ui,
            "Battle Finish Point",
            self.q_battle_finish_point.iter_mut(),
            |ui, items| {
                drag_value_edit_row(ui, "ID", DragSpeed::Slow, map!(items, id));
            },
        );
    }
}

fn edit_component<'a, T: 'a + PartialEq + Clone, R>(
    ui: &mut Ui,
    title: &'static str,
    items: impl IntoIterator<Item = Mut<'a, T>>,
    add_body: impl FnOnce(&mut Ui, &mut [T]) -> R,
) {
    let mut len = 0;
    edit_many_mut(items, |items| {
        len = items.len();
        let title = if items.len() > 1 {
            format!("{} ({})", title, items.len())
        } else {
            title.into()
        };
        framed_collapsing_header(title, ui, |ui| add_body(ui, items));
    });
    if len > 0 {
        edit_spacing(ui);
    }
}

fn edit_many_mut<'a, T: 'a + PartialEq + Clone>(
    items: impl IntoIterator<Item = Mut<'a, T>>,
    contents: impl FnOnce(&mut [T]),
) {
    let mut items: Vec<Mut<T>> = items.into_iter().collect();
    if items.is_empty() {
        return;
    };

    let mut cloned: Vec<T> = items.iter().map(|x| (*x).clone()).collect();
    contents(&mut cloned);
    for (item, other) in items.iter_mut().zip(cloned.iter()) {
        item.set_if_neq(other.clone());
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
                ui.add(egui::Label::new(label).truncate(true));
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
