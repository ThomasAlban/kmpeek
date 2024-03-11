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
                AreaKind, AreaPoint, BattleFinishPoint, CannonPoint, EnemyPathPoint, ItemPathPoint, KmpCamera, Object,
                RespawnPoint, StartPoint, TrackInfo,
            },
            path::RecalculatePaths,
            sections::{KmpEditMode, KmpSections},
        },
    },
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{self, Checkbox, DragValue, Layout, Ui, WidgetText};

#[derive(SystemParam)]
pub struct ShowEditTab<'w, 's> {
    kmp_edit_mode: Res<'w, KmpEditMode>,
    track_info: Option<ResMut<'w, TrackInfo>>,

    q_transform: Query<'w, 's, &'static mut Transform, With<Selected>>,
    q_start_point: Query<'w, 's, &'static mut StartPoint, With<Selected>>,
    q_enemy_point: Query<'w, 's, &'static mut EnemyPathPoint, With<Selected>>,
    q_item_point: Query<'w, 's, &'static mut ItemPathPoint, With<Selected>>,
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
                    edit_row("Track Type", false, ui, |ui| {
                        combobox_enum(ui, &mut track_info.track_type, "track_type", None);
                    });
                    edit_row("Lap Count", true, ui, |ui| {
                        ui.add(DragValue::new(&mut track_info.lap_count).speed(DragSpeed::Slow))
                    });
                    edit_row("Speed Mod", true, ui, |ui| {
                        ui.add(DragValue::new(&mut track_info.speed_mod).speed(DragSpeed::Slow))
                    });
                    edit_spacing(ui);
                    edit_row("Lens Flare Colour", false, ui, |ui| {
                        ui.color_edit_button_srgba_unmultiplied(&mut track_info.lens_flare_color);
                    });
                    edit_row("Lens Flare Flashing", false, ui, |ui| {
                        ui.add(Checkbox::without_text(&mut track_info.lens_flare_flashing));
                    });
                    edit_spacing(ui);
                    edit_row("First Player Pos", false, ui, |ui| {
                        combobox_enum(ui, &mut track_info.first_player_pos, "first_player_pos", None);
                    });
                    edit_row("Narrow Player Spacing", false, ui, |ui| {
                        ui.add(Checkbox::without_text(&mut track_info.narrow_player_spacing));
                    });
                });
                edit_spacing(ui);
            }
        }

        edit_component("Transform", self.q_transform.iter_mut(), ui, |ui, transforms| {
            edit_row("Translation X", true, ui, |ui| {
                drag_value_multi_edit(DragSpeed::Fast, ui, map!(transforms, translation.x));
            });
            edit_row("Y", true, ui, |ui| {
                drag_value_multi_edit(DragSpeed::Fast, ui, map!(transforms, translation.y));
            });
            edit_row("Z", true, ui, |ui| {
                drag_value_multi_edit(DragSpeed::Fast, ui, map!(transforms, translation.z));
            });
            edit_spacing(ui);
            rotation_multi_edit(ui, transforms, |ui, rots| {
                let x = edit_row("Rotation X", true, ui, |ui| {
                    drag_value_multi_edit(DragSpeed::Slow, ui, map!(rots, x))
                });
                let y = edit_row("Y", true, ui, |ui| {
                    drag_value_multi_edit(DragSpeed::Slow, ui, map!(rots, y))
                });
                let z = edit_row("Z", true, ui, |ui| {
                    drag_value_multi_edit(DragSpeed::Slow, ui, map!(rots, z))
                });
                (x, y, z)
            });
        });

        edit_component("Start Point", self.q_start_point.iter_mut(), ui, |ui, start_points| {
            edit_row("Player Index", true, ui, |ui| {
                drag_value_multi_edit(DragSpeed::Slow, ui, map!(start_points, player_index));
            });
        });

        edit_component("Enemy Point", self.q_enemy_point.iter_mut(), ui, |ui, enemy_points| {
            edit_row("Leniency", true, ui, |ui| {
                drag_value_multi_edit(DragSpeed::Slow, ui, map!(enemy_points, leniency));
            });
            edit_spacing(ui);
            edit_row("Setting 1", true, ui, |ui| {
                combobox_enum_multi_edit(ui, "enpt_s1", None, map!(enemy_points, setting_1));
            });
            edit_row("Setting 2", true, ui, |ui| {
                combobox_enum_multi_edit(ui, "enpt_s2", None, map!(enemy_points, setting_2));
            });
            edit_row("Setting 3", true, ui, |ui| {
                drag_value_multi_edit(DragSpeed::Slow, ui, map!(enemy_points, setting_3));
            });
            edit_spacing(ui);
            edit_row("Always Path Start", false, ui, |ui| {
                let res = checkbox_multi_edit(ui, map!(enemy_points, path_start_override));
                if res.changed() {
                    self.ev_recalc_paths.send_default();
                }
            });
        });

        edit_component("Item Point", self.q_item_point.iter_mut(), ui, |ui, item_points| {
            edit_row("Bullet Bill Control", true, ui, |ui| {
                drag_value_multi_edit(DragSpeed::Slow, ui, map!(item_points, bullet_control));
            });
            edit_spacing(ui);
            edit_row("Bullet Height", false, ui, |ui| {
                combobox_enum_multi_edit(ui, "itpt_s1", None, map!(item_points, bullet_height));
            });
            edit_row("Bullet Can't Drop", false, ui, |ui| {
                checkbox_multi_edit(ui, map!(item_points, bullet_cant_drop));
            });
            edit_row("Low Shell Priority", false, ui, |ui| {
                checkbox_multi_edit(ui, map!(item_points, low_shell_priority));
            });
            edit_spacing(ui);
            edit_row("Always Path Start", false, ui, |ui| {
                let res = checkbox_multi_edit(ui, map!(item_points, path_start_override));
                if res.changed() {
                    self.ev_recalc_paths.send_default();
                }
            });
        });

        // todo: checkpoints

        edit_component(
            "Respawn Point",
            self.q_respawn_point.iter_mut(),
            ui,
            |ui, respawn_points| {
                edit_row("ID", true, ui, |ui| {
                    drag_value_multi_edit(DragSpeed::Slow, ui, map!(respawn_points, id))
                });
                edit_row("Sound Trigger", true, ui, |ui| {
                    drag_value_multi_edit(DragSpeed::Slow, ui, map!(respawn_points, sound_trigger))
                });
            },
        );

        edit_component("Object", self.q_object.iter_mut(), ui, |ui, objects| {
            edit_row("Scale X", true, ui, |ui| {
                drag_value_multi_edit(DragSpeed::Fast, ui, map!(objects, scale.x));
            });
            edit_row("Y", true, ui, |ui| {
                drag_value_multi_edit(DragSpeed::Fast, ui, map!(objects, scale.y));
            });
            edit_row("Z", true, ui, |ui| {
                drag_value_multi_edit(DragSpeed::Fast, ui, map!(objects, scale.z));
            });
            edit_spacing(ui);
            edit_row("ID", true, ui, |ui| {
                drag_value_multi_edit(DragSpeed::Slow, ui, map!(objects, object_id));
            });
            edit_spacing(ui);
            for i in 0..8 {
                edit_row(format!("Setting {}", i + 1), true, ui, |ui| {
                    drag_value_multi_edit(DragSpeed::Slow, ui, objects.iter_mut().map(|x| &mut x.settings[i]));
                });
            }
        });

        edit_component("Area", self.q_area.iter_mut(), ui, |ui, areas| {
            edit_row("Scale X", true, ui, |ui| {
                drag_value_multi_edit(DragSpeed::Fast, ui, map!(areas, scale.x));
            });
            edit_row("Y", true, ui, |ui| {
                drag_value_multi_edit(DragSpeed::Fast, ui, map!(areas, scale.y));
            });
            edit_row("Z", true, ui, |ui| {
                drag_value_multi_edit(DragSpeed::Fast, ui, map!(areas, scale.z));
            });
            edit_spacing(ui);
            edit_row("Shape", false, ui, |ui| {
                combobox_enum_multi_edit(ui, "area_shape", None, map!(areas, shape));
            });
            edit_row("Priority", true, ui, |ui| {
                drag_value_multi_edit(DragSpeed::Slow, ui, map!(areas, priority));
            });
            edit_spacing(ui);
            edit_row("Type", false, ui, |ui| {
                combobox_enum_multi_edit(ui, "area_type", None, map!(areas, kind));
            });

            // for now, area type UI settings will only work when 1 point is selected
            if areas.len() == 1 {
                match &mut areas[0].kind {
                    AreaKind::Camera(cam_index) => {
                        edit_row("Camera Index", true, ui, |ui| {
                            ui.add(DragValue::new(&mut cam_index.0).speed(DragSpeed::Slow));
                        });
                    }
                    AreaKind::EnvEffect(env_effect_obj) => {
                        edit_row("Env Effect Object", true, ui, |ui| {
                            combobox_enum(ui, env_effect_obj, "area_env_effect_obj", None);
                        });
                    }
                    AreaKind::FogEffect(bfg_entry, setting_2) => {
                        edit_row("BFG Entry", true, ui, |ui| {
                            ui.add(DragValue::new(&mut bfg_entry.0).speed(DragSpeed::Slow));
                        });
                        edit_row("Setting 2", true, ui, |ui| {
                            ui.add(DragValue::new(&mut setting_2.0).speed(DragSpeed::Slow));
                        });
                    }
                    // TODO - abstract route IDs away
                    AreaKind::MovingRoad(area_route_id) => {
                        edit_row("Route ID", true, ui, |ui| {
                            ui.add(DragValue::new(&mut area_route_id.0).speed(DragSpeed::Slow));
                        });
                    }
                    AreaKind::MinimapControl(setting_1, setting_2) => {
                        edit_row("Setting 1", true, ui, |ui| {
                            ui.add(DragValue::new(&mut setting_1.0).speed(DragSpeed::Slow));
                        });
                        edit_row("Setting 2", true, ui, |ui| {
                            ui.add(DragValue::new(&mut setting_2.0).speed(DragSpeed::Slow));
                        });
                    }
                    AreaKind::BloomEffect(bblm_file, fade_time) => {
                        edit_row("BBLM File", true, ui, |ui| {
                            ui.add(DragValue::new(&mut bblm_file.0).speed(DragSpeed::Slow));
                        });
                        edit_row("Fade Time", true, ui, |ui| {
                            ui.add(DragValue::new(&mut fade_time.0).speed(DragSpeed::Slow));
                        });
                    }
                    AreaKind::ObjectGroup(group_id) | AreaKind::ObjectUnload(group_id) => {
                        edit_row("Group ID", true, ui, |ui| {
                            ui.add(DragValue::new(&mut group_id.0).speed(DragSpeed::Slow));
                        });
                    }
                    // other types of area don't have any settings
                    _ => {}
                }
            }
            edit_spacing(ui);
            edit_row("Always show area", false, ui, |ui| {
                checkbox_multi_edit(ui, map!(areas, show_area));
            });
        });

        edit_component("Camera", self.q_camera.iter_mut(), ui, |ui, cameras| {
            edit_row("Type", false, ui, |ui| {
                combobox_enum_multi_edit(ui, "cam_type", None, map!(cameras, kind));
            });
            edit_spacing(ui);
            edit_row("Next Index", true, ui, |ui| {
                drag_value_multi_edit(DragSpeed::Slow, ui, map!(cameras, next_index));
            });
            edit_row("Route Index", true, ui, |ui| {
                drag_value_multi_edit(DragSpeed::Slow, ui, map!(cameras, route));
            });
            edit_spacing(ui);
            edit_row("Time", true, ui, |ui| {
                drag_value_multi_edit(DragSpeed::Slow, ui, map!(cameras, time));
            });
            edit_spacing(ui);
            edit_row("Point Speed", true, ui, |ui| {
                drag_value_multi_edit(DragSpeed::Slow, ui, map!(cameras, point_velocity));
            });
            edit_row("Zoom Speed", true, ui, |ui| {
                drag_value_multi_edit(DragSpeed::Slow, ui, map!(cameras, zoom_velocity));
            });
            edit_row("View Speed", true, ui, |ui| {
                drag_value_multi_edit(DragSpeed::Slow, ui, map!(cameras, view_velocity));
            });
            edit_spacing(ui);
            edit_row("Zoom Start", true, ui, |ui| {
                drag_value_multi_edit(DragSpeed::Slow, ui, map!(cameras, zoom_start));
            });
            edit_row("Zoom End", true, ui, |ui| {
                drag_value_multi_edit(DragSpeed::Slow, ui, map!(cameras, zoom_end));
            });
            edit_spacing(ui);
            edit_row("View Start X", true, ui, |ui| {
                drag_value_multi_edit(DragSpeed::Slow, ui, map!(cameras, view_start.x));
            });
            edit_row("Y", true, ui, |ui| {
                drag_value_multi_edit(DragSpeed::Slow, ui, map!(cameras, view_start.y));
            });
            edit_row("Z", true, ui, |ui| {
                drag_value_multi_edit(DragSpeed::Slow, ui, map!(cameras, view_start.z));
            });
            edit_spacing(ui);
            edit_row("View End X", true, ui, |ui| {
                drag_value_multi_edit(DragSpeed::Slow, ui, map!(cameras, view_end.x));
            });
            edit_row("Y", true, ui, |ui| {
                drag_value_multi_edit(DragSpeed::Slow, ui, map!(cameras, view_end.y));
            });
            edit_row("Z", true, ui, |ui| {
                drag_value_multi_edit(DragSpeed::Slow, ui, map!(cameras, view_end.z));
            });
            edit_spacing(ui);
            edit_row("Shake (?)", true, ui, |ui| {
                drag_value_multi_edit(DragSpeed::Slow, ui, map!(cameras, shake));
            });
            edit_row("Start (?)", true, ui, |ui| {
                drag_value_multi_edit(DragSpeed::Slow, ui, map!(cameras, start));
            });
            edit_row("Movie (?)", true, ui, |ui| {
                drag_value_multi_edit(DragSpeed::Slow, ui, map!(cameras, movie));
            });
        });

        edit_component(
            "Cannon Point",
            self.q_cannon_point.iter_mut(),
            ui,
            |ui, cannon_points| {
                edit_row("ID", true, ui, |ui| {
                    drag_value_multi_edit(DragSpeed::Slow, ui, map!(cannon_points, id))
                });
                edit_row("Shoot Effect", false, ui, |ui| {
                    combobox_enum_multi_edit(ui, "cnpt_shoot_effect", None, map!(cannon_points, shoot_effect))
                });
            },
        );

        edit_component(
            "Battle Finish Point",
            self.q_battle_finish_point.iter_mut(),
            ui,
            |ui, battle_finish_points| {
                edit_row("ID", true, ui, |ui| {
                    drag_value_multi_edit(DragSpeed::Slow, ui, map!(battle_finish_points, id))
                });
            },
        );
    }
}

fn edit_component<'a, T: 'a + PartialEq + Clone, R>(
    title: &'static str,
    items: impl IntoIterator<Item = Mut<'a, T>>,
    ui: &mut Ui,
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

fn edit_spacing(ui: &mut Ui) {
    ui.vertical(|ui| ui.add_space(3.));
}

fn edit_row<R>(
    label: impl Into<WidgetText>,
    justified: bool,
    ui: &mut Ui,
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
