use crate::{
    ui::{
        keybinds::ModifiersPressed,
        util::{combobox_enum, drag_vec3, euler_to_quat, quat_to_euler, DragSpeed},
        viewport::ViewportInfo,
    },
    viewer::{
        edit::{create_delete::CreatePoint, select::Selected},
        kmp::{
            components::{
                AreaKind, AreaPoint, BattleFinishPoint, CannonPoint, Checkpoint, EnemyPathPoint, ItemPathPoint,
                KmpCamera, Object, RespawnPoint, StartPoint, TrackInfo,
            },
            ordering::OrderId,
            sections::{get_kmp_section, KmpEditMode, ToKmpSection},
        },
    },
};
use bevy::{
    ecs::system::{SystemParam, SystemState},
    prelude::*,
};
use bevy_egui::egui::{self, emath::Numeric, Checkbox, Direction, DragValue, Layout, Response, Sense, Ui};
use egui_extras::{Column, TableBuilder, TableRow};

pub fn show_table_tab(ui: &mut Ui, world: &mut World) {
    // show the top bit if we are not in track info mode
    if !world.contains_resource::<KmpEditMode<TrackInfo>>() {
        ui.horizontal(|ui| {
            ui.heading(get_kmp_section(world).to_string());
            ui.add_space(10.);
            if ui.button("+").clicked() {
                world.send_event_default::<CreatePoint>();
            }
        });
    }

    show_kmp_table::<StartPoint>(ui, world);
    show_kmp_table::<EnemyPathPoint>(ui, world);
    show_kmp_table::<ItemPathPoint>(ui, world);
    show_kmp_table::<Checkpoint>(ui, world);
    show_kmp_table::<RespawnPoint>(ui, world);
    show_kmp_table::<Object>(ui, world);
    show_kmp_table::<AreaPoint>(ui, world);
    show_kmp_table::<KmpCamera>(ui, world);
    show_kmp_table::<CannonPoint>(ui, world);
    show_kmp_table::<BattleFinishPoint>(ui, world);
}

trait ShowKmpTableTrait {
    const ROTATION: bool = true;
    const Y_TRANSLATION: bool = true;
    const COLUMNS: &'static [&'static str];
    fn show_row(row: &mut TableRow, item: &mut Self);
}

use DragSpeed::*;

impl ShowKmpTableTrait for StartPoint {
    const COLUMNS: &'static [&'static str] = &["Player Index"];
    fn show_row(row: &mut TableRow, item: &mut Self) {
        drag_value_column(row, Slow, &mut item.player_index);
    }
}

impl ShowKmpTableTrait for EnemyPathPoint {
    const ROTATION: bool = false;
    const COLUMNS: &'static [&'static str] = &["Leniency", "Setting 1", "Setting 2", "Setting 3"];
    fn show_row(row: &mut TableRow, item: &mut Self) {
        drag_value_column(row, Slow, &mut item.leniency);
        combobox_column(row, &mut item.setting_1);
        combobox_column(row, &mut item.setting_2);
        drag_value_column(row, Slow, &mut item.setting_3);
    }
}

impl ShowKmpTableTrait for ItemPathPoint {
    const ROTATION: bool = false;
    const COLUMNS: &'static [&'static str] = &[
        "Bullet Control",
        "Bullet Height",
        "Bullet Can't Drop",
        "Low Shell Priority",
    ];
    fn show_row(row: &mut TableRow, item: &mut Self) {
        drag_value_column(row, Slow, &mut item.bullet_control);
        combobox_column(row, &mut item.bullet_height);
        checkbox_column(row, &mut item.bullet_cant_drop);
        checkbox_column(row, &mut item.low_shell_priority);
    }
}

impl ShowKmpTableTrait for Checkpoint {
    const ROTATION: bool = false;
    const Y_TRANSLATION: bool = false;
    const COLUMNS: &'static [&'static str] = &["Type"];
    fn show_row(row: &mut TableRow, item: &mut Self) {
        combobox_column(row, &mut item.kind);
    }
}

impl ShowKmpTableTrait for RespawnPoint {
    const COLUMNS: &'static [&'static str] = &["Sound Trigger"];
    fn show_row(row: &mut TableRow, item: &mut Self) {
        drag_value_column(row, Slow, &mut item.sound_trigger);
    }
}

impl ShowKmpTableTrait for Object {
    const COLUMNS: &'static [&'static str] = &[
        "Scale",
        "Object ID",
        "Setting 1",
        "Setting 2",
        "Setting 3",
        "Setting 4",
        "Setting 5",
        "Setting 6",
        "Setting 7",
        "Setting 8",
        "Presence",
    ];
    fn show_row(row: &mut TableRow, item: &mut Self) {
        drag_vec3_column(row, Slow, &mut item.scale);
        drag_value_column(row, Slow, &mut item.object_id);
        for setting in item.settings.iter_mut() {
            drag_value_column(row, Slow, setting);
        }
        drag_value_column(row, Slow, &mut item.presence);
    }
}

impl ShowKmpTableTrait for AreaPoint {
    const COLUMNS: &'static [&'static str] = &["Scale", "Shape", "Priority", "Type", "Setting"];
    fn show_row(row: &mut TableRow, item: &mut Self) {
        drag_vec3_column(row, Slow, &mut item.scale);
        combobox_column(row, &mut item.shape);
        drag_value_column(row, Slow, &mut item.priority);
        combobox_column(row, &mut item.kind);
        match &mut item.kind {
            AreaKind::Camera { cam_index } => {
                labelled_drag_value_column(row, cam_index, Slow, "Camera Index");
            }
            AreaKind::EnvEffect(env_effect_obj) => {
                combobox_column(row, env_effect_obj);
            }
            AreaKind::FogEffect { bfg_entry, setting_2 } => {
                two_labelled_drag_values_column(row, (bfg_entry, Slow, "BFG Entry"), (setting_2, Slow, "BFG Entry"));
            }
            AreaKind::MovingRoad { route_id } => {
                labelled_drag_value_column(row, route_id, Slow, "Route ID");
            }
            AreaKind::MinimapControl { setting_1, setting_2 } => {
                two_labelled_drag_values_column(row, (setting_1, Slow, "Setting 1"), (setting_2, Slow, "Setting 2"));
            }
            AreaKind::BloomEffect { bblm_file, fade_time } => {
                two_labelled_drag_values_column(row, (bblm_file, Slow, "BBLM File"), (fade_time, Slow, "Fade Time"));
            }
            // enable boos has no setting
            AreaKind::ObjectGroup { group_id } | AreaKind::ObjectUnload { group_id } => {
                labelled_drag_value_column(row, group_id, Slow, "Group ID");
            }
            _ => (),
        };
    }
}

impl ShowKmpTableTrait for KmpCamera {
    const COLUMNS: &'static [&'static str] = &[
        "Type",
        "Next Index",
        "Route Index",
        "Time",
        "Point Speed",
        "Zoom Speed",
        "View Speed",
        "Zoom Start",
        "Zoom End",
        "View Start",
        "View End",
        "Shake (?)",
        "Start (?)",
        "Movie (?)",
    ];
    fn show_row(row: &mut TableRow, item: &mut Self) {
        combobox_column(row, &mut item.kind);
        drag_value_column(row, Slow, &mut item.next_index);
        drag_value_column(row, Slow, &mut item.route);
        drag_value_column(row, Slow, &mut item.time);
        drag_value_column(row, Slow, &mut item.point_velocity);
        drag_value_column(row, Slow, &mut item.zoom_velocity);
        drag_value_column(row, Slow, &mut item.view_velocity);
        drag_value_column(row, Slow, &mut item.zoom_start);
        drag_value_column(row, Slow, &mut item.zoom_end);
        drag_vec3_column(row, Slow, &mut item.view_start);
        drag_vec3_column(row, Slow, &mut item.view_end);
        drag_value_column(row, Slow, &mut item.shake);
        drag_value_column(row, Slow, &mut item.start);
        drag_value_column(row, Slow, &mut item.movie);
    }
}

impl ShowKmpTableTrait for CannonPoint {
    const COLUMNS: &'static [&'static str] = &["Shoot Effect"];
    fn show_row(row: &mut TableRow, item: &mut Self) {
        combobox_column(row, &mut item.shoot_effect);
    }
}

impl ShowKmpTableTrait for BattleFinishPoint {
    const COLUMNS: &'static [&'static str] = &[];
    fn show_row(_: &mut TableRow, _: &mut Self) {}
}

fn show_kmp_table<T: Component + ToKmpSection + PartialEq + Clone + ShowKmpTableTrait>(ui: &mut Ui, world: &mut World) {
    if !world.contains_resource::<KmpEditMode<T>>() {
        return;
    }

    let mut ss = SystemState::<(
        Query<(&mut T, &mut Transform, Entity, Has<Selected>, &OrderId)>,
        Query<Entity, With<T>>,
        Commands,
        Res<ButtonInput<KeyCode>>,
    )>::new(world);
    let (mut q, q_entities, mut commands, keys) = ss.get_mut(world);

    let mut table_builder = TableBuilder::new(ui)
        .striped(true)
        .vscroll(false)
        .cell_layout(Layout::centered_and_justified(egui::Direction::TopDown))
        .sense(Sense::click())
        .column(Column::exact(25.)) // id
        .column(Column::exact(50.)) // selected
        .column(Column::auto().resizable(true)); // translation

    if T::ROTATION {
        table_builder = table_builder.column(Column::auto().resizable(true));
    }
    for _ in T::COLUMNS.iter() {
        table_builder = table_builder.column(Column::auto().resizable(true));
    }

    // empty row filling remaining space
    table_builder = table_builder.column(Column::remainder());

    let table = table_builder.header(20., |mut header| {
        header.col(|ui| {
            ui.label("ID");
        });
        header.col(|ui| {
            ui.label("Selected");
        });
        header.col(|ui| {
            ui.label("Translation");
        });
        if T::ROTATION {
            header.col(|ui| {
                ui.label("Rotation");
            });
        }
        for name in T::COLUMNS.iter() {
            header.col(|ui| {
                ui.label(*name);
            });
        }
        // empty header (which fills remaining space on the right)
        header.col(|_| {});
    });

    table.body(|mut body| {
        for (mut t, mut transform, e, is_selected, order_id) in q.iter_mut().sort::<&OrderId>() {
            body.row(20., |mut row| {
                row.set_selected(is_selected);

                // show the 'select' ui (which is the same for every KMP table)
                let mut select_checkbox = is_selected;
                let mut select_checkbox_changed = false;
                row.col(|ui| {
                    ui.add(egui::Label::new(order_id.to_string()).selectable(false));
                });
                row.col(|ui| {
                    select_checkbox_changed = ui.add(Checkbox::without_text(&mut select_checkbox)).changed();
                });

                let mut t_cp = t.clone();
                let mut transform_cp = *transform;

                row.col(|ui| {
                    let value: &mut Vec3 = &mut transform_cp.translation;
                    let (num_cols, z_ix) = if T::Y_TRANSLATION { (3, 2) } else { (2, 1) };
                    ui.columns(num_cols, |ui| {
                        ui[0].centered_and_justified(|ui| {
                            ui.add(
                                egui::DragValue::new(&mut value.x)
                                    .speed(DragSpeed::Fast)
                                    .prefix("X: ")
                                    .fixed_decimals(1),
                            )
                        });
                        if T::Y_TRANSLATION {
                            ui[1].centered_and_justified(|ui| {
                                ui.add(
                                    egui::DragValue::new(&mut value.y)
                                        .speed(DragSpeed::Fast)
                                        .prefix("Y: ")
                                        .fixed_decimals(1),
                                )
                            });
                        }
                        ui[z_ix].centered_and_justified(|ui| {
                            ui.add(
                                egui::DragValue::new(&mut value.z)
                                    .speed(DragSpeed::Fast)
                                    .prefix("Z: ")
                                    .fixed_decimals(1),
                            )
                        });
                    });
                });
                if T::ROTATION {
                    let mut rot = quat_to_euler(&transform_cp);
                    row.col(|ui| {
                        let res = drag_vec3(ui, &mut rot, DragSpeed::Slow);
                        euler_to_quat(rot, res, &mut transform_cp);
                    });
                }

                T::show_row(&mut row, &mut t_cp);

                t.set_if_neq(t_cp);
                transform.set_if_neq(transform_cp);

                // extra blank row that fills up whatever remaining space there is
                row.col(|_| {});

                if select_checkbox_changed {
                    if select_checkbox {
                        commands.entity(e).insert(Selected);
                    } else {
                        commands.entity(e).remove::<Selected>();
                    }
                }
                if row.response().clicked() {
                    if !keys.shift_pressed() {
                        for e in q_entities.iter() {
                            commands.entity(e).remove::<Selected>();
                        }
                    }
                    commands.entity(e).insert(Selected);
                }
            });
        }
    });

    ss.apply(world);
}

// #[derive(SystemParam)]
// struct ShowKmpTable<'w, 's, T: Component + ToKmpSection> {
//     mode: Option<Res<'w, KmpEditMode<T>>>,
//     q: KmpTableQuery<'w, 's, T>,
//     q_entities: Query<'w, 's, Entity, With<T>>,
//     commands: Commands<'w, 's>,
//     keys: Res<'w, ButtonInput<KeyCode>>,
// }
// impl<T: Component + ToKmpSection + PartialEq + Clone + ShowKmpTableTrait> ShowKmpTable<'_, '_, T> {
//     fn show(&mut self, ui: &mut Ui) {
//         if self.mode.is_none() {
//             return;
//         }
//         let mut table_builder = TableBuilder::new(ui)
//             .striped(true)
//             .vscroll(false)
//             .cell_layout(Layout::centered_and_justified(egui::Direction::TopDown))
//             .sense(Sense::click())
//             .column(Column::exact(25.)) // id
//             .column(Column::exact(50.)) // selected
//             .column(Column::auto().resizable(true)); // translation
//         if T::ROTATION {
//             table_builder = table_builder.column(Column::auto().resizable(true));
//         }
//         for _ in T::COLUMNS.iter() {
//             table_builder = table_builder.column(Column::auto().resizable(true));
//         }
//         // empty row filling remaining space
//         table_builder = table_builder.column(Column::remainder());

//         let table = table_builder.header(20., |mut header| {
//             header.col(|ui| {
//                 ui.label("ID");
//             });
//             header.col(|ui| {
//                 ui.label("Selected");
//             });
//             header.col(|ui| {
//                 ui.label("Translation");
//             });
//             if T::ROTATION {
//                 header.col(|ui| {
//                     ui.label("Rotation");
//                 });
//             }
//             for name in T::COLUMNS.iter() {
//                 header.col(|ui| {
//                     ui.label(*name);
//                 });
//             }
//             // empty header (which fills remaining space on the right)
//             header.col(|_| {});
//         });
//         table.body(|mut body| {
//             for (mut t, mut transform, e, is_selected, order_id) in self.q.iter_mut().sort::<&OrderId>() {
//                 body.row(20., |mut row| {
//                     row.set_selected(is_selected);

//                     // show the 'select' ui (which is the same for every KMP table)
//                     let mut select_checkbox = is_selected;
//                     let mut select_checkbox_changed = false;
//                     row.col(|ui| {
//                         ui.add(egui::Label::new(order_id.to_string()).selectable(false));
//                     });
//                     row.col(|ui| {
//                         select_checkbox_changed = ui.add(Checkbox::without_text(&mut select_checkbox)).changed();
//                     });

//                     let mut t_cp = t.clone();
//                     let mut transform_cp = *transform;

//                     row.col(|ui| {
//                         let value: &mut Vec3 = &mut transform_cp.translation;
//                         let (num_cols, z_ix) = if T::Y_TRANSLATION { (3, 2) } else { (2, 1) };
//                         ui.columns(num_cols, |ui| {
//                             ui[0].centered_and_justified(|ui| {
//                                 ui.add(
//                                     egui::DragValue::new(&mut value.x)
//                                         .speed(DragSpeed::Fast)
//                                         .prefix("X: ")
//                                         .fixed_decimals(1),
//                                 )
//                             });
//                             if T::Y_TRANSLATION {
//                                 ui[1].centered_and_justified(|ui| {
//                                     ui.add(
//                                         egui::DragValue::new(&mut value.y)
//                                             .speed(DragSpeed::Fast)
//                                             .prefix("Y: ")
//                                             .fixed_decimals(1),
//                                     )
//                                 });
//                             }
//                             ui[z_ix].centered_and_justified(|ui| {
//                                 ui.add(
//                                     egui::DragValue::new(&mut value.z)
//                                         .speed(DragSpeed::Fast)
//                                         .prefix("Z: ")
//                                         .fixed_decimals(1),
//                                 )
//                             });
//                         });
//                     });
//                     if T::ROTATION {
//                         let mut rot = quat_to_euler(&transform_cp);
//                         row.col(|ui| {
//                             let res = drag_vec3(ui, &mut rot, DragSpeed::Slow);
//                             euler_to_quat(rot, res, &mut transform_cp);
//                         });
//                     }

//                     T::show_row(&mut row, &mut t_cp);

//                     t.set_if_neq(t_cp);
//                     transform.set_if_neq(transform_cp);

//                     // extra blank row that fills up whatever remaining space there is
//                     row.col(|_| {});

//                     if select_checkbox_changed {
//                         if select_checkbox {
//                             self.commands.entity(e).insert(Selected);
//                         } else {
//                             self.commands.entity(e).remove::<Selected>();
//                         }
//                     }
//                     if row.response().clicked() {
//                         if !self.keys.shift_pressed() {
//                             for e in self.q_entities.iter() {
//                                 self.commands.entity(e).remove::<Selected>();
//                             }
//                         }
//                         self.commands.entity(e).insert(Selected);
//                     }
//                 });
//             }
//         });
//     }
// }

fn labelled_drag_value<T: Numeric>(ui: &mut Ui, item: &mut T, speed: impl Into<f64>, label: impl Into<String>) {
    ui.add(DragValue::new(item).prefix(format!("{}: ", label.into())).speed(speed));
}
fn drag_value_column<T: Numeric>(row: &mut TableRow, speed: impl Into<f64>, item: &mut T) -> Response {
    row.col(|ui| {
        ui.with_layout(Layout::centered_and_justified(Direction::TopDown), |ui| {
            ui.add(DragValue::new(item).speed(speed));
        });
    })
    .1
}
fn two_labelled_drag_values_column<T: Numeric>(
    row: &mut TableRow,
    first: (&mut T, impl Into<f64>, impl Into<String>),
    second: (&mut T, impl Into<f64>, impl Into<String>),
) {
    row.col(|ui| {
        ui.columns(2, |ui| {
            ui[0].with_layout(Layout::centered_and_justified(Direction::TopDown), |ui| {
                labelled_drag_value(ui, first.0, first.1, first.2);
            });
            ui[1].with_layout(Layout::centered_and_justified(Direction::TopDown), |ui| {
                labelled_drag_value(ui, second.0, second.1, second.2);
            });
        });
    });
}
fn labelled_drag_value_column<T: Numeric>(
    row: &mut TableRow,
    item: &mut T,
    speed: impl Into<f64>,
    label: impl Into<String>,
) -> Response {
    row.col(|ui| {
        ui.with_layout(Layout::centered_and_justified(Direction::TopDown), |ui| {
            labelled_drag_value(ui, item, speed, label);
        });
    })
    .1
}
fn drag_vec3_column(row: &mut TableRow, speed: impl Into<f64>, item: &mut Vec3) -> Response {
    let speed = speed.into();
    row.col(|ui| {
        ui.columns(3, |ui| {
            ui[0].centered_and_justified(|ui| {
                ui.add(
                    egui::DragValue::new(&mut item.x)
                        .speed(speed)
                        .prefix("X: ")
                        .fixed_decimals(1),
                )
            });
            ui[1].centered_and_justified(|ui| {
                ui.add(
                    egui::DragValue::new(&mut item.y)
                        .speed(speed)
                        .prefix("Y: ")
                        .fixed_decimals(1),
                )
            });
            ui[2].centered_and_justified(|ui| {
                ui.add(
                    egui::DragValue::new(&mut item.z)
                        .speed(speed)
                        .prefix("Z: ")
                        .fixed_decimals(1),
                )
            });
        })
    })
    .1
}
fn combobox_column<T: strum::IntoEnumIterator + std::fmt::Display + PartialEq + Clone>(
    row: &mut TableRow,
    item: &mut T,
) -> Response {
    row.col(|ui| {
        ui.with_layout(Layout::centered_and_justified(Direction::TopDown), |ui| {
            combobox_enum(ui, item, None);
        });
    })
    .1
}
fn checkbox_column(row: &mut TableRow, item: &mut bool) -> Response {
    row.col(|ui| {
        ui.with_layout(Layout::centered_and_justified(Direction::TopDown), |ui| {
            ui.add(Checkbox::without_text(item));
        });
    })
    .1
}
