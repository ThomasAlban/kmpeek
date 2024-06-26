use super::UiSubSection;
use crate::{
    ui::util::{combobox_enum, drag_vec3, euler_to_quat, quat_to_euler, DragSpeed},
    viewer::{
        edit::{create_delete::CreatePoint, select::Selected},
        kmp::{
            components::{
                AreaKind, AreaPoint, BattleFinishPoint, CannonPoint, Checkpoint, EnemyPathPoint, ItemPathPoint,
                KmpCamera, Object, RespawnPoint, StartPoint,
            },
            ordering::OrderID,
            sections::{KmpEditMode, KmpSections},
        },
    },
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{self, emath::Numeric, Checkbox, Direction, DragValue, Layout, Response, Ui};
use egui_extras::{Column, TableBuilder, TableRow};

type KmpTableItem<'a, 'w, 's, C> = (Mut<'a, C>, Mut<'a, Transform>, Entity, bool, &'a OrderID);

type KmpTableQuery<'w, 's, C> = Query<
    'w,
    's,
    (
        &'static mut C,
        &'static mut Transform,
        Entity,
        Has<Selected>,
        &'static OrderID,
    ),
>;

#[derive(SystemParam)]
pub struct ShowTableTab<'w, 's> {
    commands: Commands<'w, 's>,
    edit_mode: Res<'w, KmpEditMode>,
    ev_create_pt: EventWriter<'w, CreatePoint>,
    q: ParamSet<
        'w,
        's,
        (
            ParamSet<
                'w,
                's,
                (
                    KmpTableQuery<'w, 's, StartPoint>,
                    KmpTableQuery<'w, 's, EnemyPathPoint>,
                    KmpTableQuery<'w, 's, ItemPathPoint>,
                    KmpTableQuery<'w, 's, Checkpoint>,
                    KmpTableQuery<'w, 's, RespawnPoint>,
                ),
            >,
            ParamSet<
                'w,
                's,
                (
                    KmpTableQuery<'w, 's, Object>,
                    KmpTableQuery<'w, 's, AreaPoint>,
                    KmpTableQuery<'w, 's, KmpCamera>,
                    KmpTableQuery<'w, 's, CannonPoint>,
                    KmpTableQuery<'w, 's, BattleFinishPoint>,
                ),
            >,
        ),
    >,
}
impl UiSubSection for ShowTableTab<'_, '_> {
    fn show(&mut self, ui: &mut Ui) {
        use DragSpeed::*;
        use KmpSections::*;
        // top section
        if self.edit_mode.0 != TrackInfo {
            ui.horizontal(|ui| {
                ui.heading(self.edit_mode.0.to_string());
                ui.add_space(10.);
                if ui.button("+").clicked() {
                    self.ev_create_pt.send_default();
                }
            });
        }
        match self.edit_mode.0 {
            StartPoints => KmpTable::new(ui, &mut self.commands, self.q.p0().p0().iter_mut())
                .columns(["Player Index"])
                .show(|row, item| {
                    drag_value_column(row, Slow, &mut item.player_index);
                }),
            EnemyPaths => KmpTable::new(ui, &mut self.commands, self.q.p0().p1().iter_mut())
                .columns(["Leniency", "Setting 1", "Setting 2", "Setting 3", "Always Path Start"])
                .no_rotation()
                .show(|row, item| {
                    drag_value_column(row, Slow, &mut item.leniency);
                    combobox_column(row, &mut item.setting_1);
                    combobox_column(row, &mut item.setting_2);
                    drag_value_column(row, Slow, &mut item.setting_3);
                    //checkbox_column(row, &mut item.path_start);
                }),
            ItemPaths => KmpTable::new(ui, &mut self.commands, self.q.p0().p2().iter_mut())
                .columns([
                    "Bullet Control",
                    "Bullet Height",
                    "Bullet Can't Drop",
                    "Low Shell Priority",
                    "Always Path Start",
                ])
                .no_rotation()
                .show(|row, item| {
                    drag_value_column(row, Slow, &mut item.bullet_control);
                    combobox_column(row, &mut item.bullet_height);
                    checkbox_column(row, &mut item.bullet_cant_drop);
                    checkbox_column(row, &mut item.low_shell_priority);
                    //checkbox_column(row, &mut item.path_start);
                }),
            Checkpoints => KmpTable::new(ui, &mut self.commands, self.q.p0().p3().iter_mut())
                .columns(["Type", "Always Path Start"])
                .no_rotation()
                .no_y_translation()
                .show(|row, item| {
                    combobox_column(row, &mut item.kind);
                    //checkbox_column(row, &mut item.path_start_override);
                }),
            RespawnPoints => KmpTable::new(ui, &mut self.commands, self.q.p0().p4().iter_mut())
                .columns(["Sound Trigger"])
                .show(|row, item| {
                    drag_value_column(row, Slow, &mut item.sound_trigger);
                }),
            Objects => KmpTable::new(ui, &mut self.commands, self.q.p1().p0().iter_mut())
                .columns([
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
                ])
                .show(|row, item| {
                    drag_vec3_column(row, Slow, &mut item.scale);
                    drag_value_column(row, Slow, &mut item.object_id);
                    for setting in item.settings.iter_mut() {
                        drag_value_column(row, Slow, setting);
                    }
                    drag_value_column(row, Slow, &mut item.presence);
                }),
            Areas => KmpTable::new(ui, &mut self.commands, self.q.p1().p1().iter_mut())
                .columns(["Scale", "Shape", "Priority", "Type", "Setting"])
                .show(|row, item| {
                    drag_vec3_column(row, Slow, &mut item.scale);
                    combobox_column(row, &mut item.shape);
                    drag_value_column(row, Slow, &mut item.priority);
                    combobox_column(row, &mut item.kind);
                    match &mut item.kind {
                        AreaKind::Camera(cam_index) => {
                            labelled_drag_value_column(row, &mut cam_index.0, Slow, "Camera Index");
                        }
                        AreaKind::EnvEffect(env_effect_obj) => {
                            combobox_column(row, env_effect_obj);
                        }
                        AreaKind::FogEffect { bfg_entry, setting_2 } => {
                            two_labelled_drag_values_column(
                                row,
                                (bfg_entry, Slow, "BFG Entry"),
                                (setting_2, Slow, "BFG Entry"),
                            );
                        }
                        AreaKind::MovingRoad { route_id } => {
                            labelled_drag_value_column(row, route_id, Slow, "Route ID");
                        }
                        AreaKind::MinimapControl { setting_1, setting_2 } => {
                            two_labelled_drag_values_column(
                                row,
                                (setting_1, Slow, "Setting 1"),
                                (setting_2, Slow, "Setting 2"),
                            );
                        }
                        AreaKind::BloomEffect { bblm_file, fade_time } => {
                            two_labelled_drag_values_column(
                                row,
                                (bblm_file, Slow, "BBLM File"),
                                (fade_time, Slow, "Fade Time"),
                            );
                        }
                        // enable boos has no setting
                        AreaKind::ObjectGroup { group_id } | AreaKind::ObjectUnload { group_id } => {
                            labelled_drag_value_column(row, group_id, Slow, "Group ID");
                        }
                        _ => (),
                    };
                }),

            Cameras => KmpTable::new(ui, &mut self.commands, self.q.p1().p2().iter_mut())
                .columns([
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
                ])
                .show(|row, item| {
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
                }),
            CannonPoints => KmpTable::new(ui, &mut self.commands, self.q.p1().p3().iter_mut())
                .columns(["Shoot Effect"])
                .show(|row, item| {
                    combobox_column(row, &mut item.shoot_effect);
                }),
            BattleFinishPoints => KmpTable::new(ui, &mut self.commands, self.q.p1().p4().iter_mut())
                .columns([])
                .show(|_, _| {}),
            _ => (),
        }
    }
}

struct KmpTable<'a, 'w, 's, T: Component + PartialEq + Clone> {
    ui: &'a mut Ui,
    commands: &'a mut Commands<'w, 's>,
    columns: Vec<&'static str>,
    items: Vec<KmpTableItem<'a, 'w, 's, T>>,
    show_rotation: bool,
    show_y_translation: bool,
}
impl<'a, 'w, 's, T: Component + PartialEq + Clone> KmpTable<'a, 'w, 's, T> {
    fn new(
        ui: &'a mut Ui,
        commands: &'a mut Commands<'w, 's>,
        items: impl Iterator<Item = KmpTableItem<'a, 'w, 's, T>>,
    ) -> Self {
        let mut items: Vec<_> = items.collect();
        items.sort_by(|x, y| x.4.cmp(y.4));
        Self {
            ui,
            commands,
            columns: vec![],
            items,
            show_rotation: true,
            show_y_translation: true,
        }
    }
    fn columns(mut self, cols: impl IntoIterator<Item = &'static str>) -> Self {
        self.columns = cols.into_iter().collect();
        self
    }
    fn no_rotation(mut self) -> Self {
        self.show_rotation = false;
        self
    }
    fn no_y_translation(mut self) -> Self {
        self.show_y_translation = false;
        self
    }
    fn show(self, mut rows: impl FnMut(&mut TableRow, &mut T)) {
        let mut table_builder = TableBuilder::new(self.ui)
            .striped(true)
            .vscroll(false)
            // .sense(Sense::click())
            .cell_layout(Layout::centered_and_justified(egui::Direction::TopDown))
            .column(Column::exact(25.)) // id
            .column(Column::exact(50.)) // selected
            .column(Column::auto().resizable(true)); // translation
        if self.show_rotation {
            table_builder = table_builder.column(Column::auto().resizable(true));
        }
        for _ in self.columns.iter() {
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
            if self.show_rotation {
                header.col(|ui| {
                    ui.label("Rotation");
                });
            }
            for name in self.columns.iter() {
                header.col(|ui| {
                    ui.label(*name);
                });
            }
            // empty header (which fills remaining space on the right)
            header.col(|_| {});
        });
        table.body(|mut body| {
            for (mut t, mut transform, e, is_selected, order_id) in self.items {
                body.row(20., |mut row| {
                    row.set_selected(is_selected);

                    // show the 'select' ui (which is the same for every KMP table)
                    let mut select_checkbox = is_selected;
                    let mut select_checkbox_changed = false;
                    row.col(|ui| {
                        ui.label(order_id.to_string());
                    });
                    row.col(|ui| {
                        select_checkbox_changed = ui.checkbox(&mut select_checkbox, "").changed();
                    });

                    let mut t_cp = t.clone();
                    let mut transform_cp = *transform;

                    row.col(|ui| {
                        let value: &mut Vec3 = &mut transform_cp.translation;
                        let (num_cols, z_ix) = if self.show_y_translation { (3, 2) } else { (2, 1) };
                        ui.columns(num_cols, |ui| {
                            ui[0].centered_and_justified(|ui| {
                                ui.add(
                                    egui::DragValue::new(&mut value.x)
                                        .speed(DragSpeed::Fast)
                                        .prefix("X: ")
                                        .fixed_decimals(1),
                                )
                            });
                            if self.show_y_translation {
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
                    if self.show_rotation {
                        let mut rot = quat_to_euler(&transform_cp);
                        row.col(|ui| {
                            let res = drag_vec3(ui, &mut rot, DragSpeed::Slow);
                            euler_to_quat(rot, res, &mut transform_cp);
                        });
                    }

                    rows(&mut row, &mut t_cp);

                    t.set_if_neq(t_cp);
                    transform.set_if_neq(transform_cp);

                    // extra blank row that fills up whatever remaining space there is
                    row.col(|_| {});

                    if select_checkbox_changed {
                        if select_checkbox {
                            self.commands.entity(e).insert(Selected);
                        } else {
                            self.commands.entity(e).remove::<Selected>();
                        }
                    }
                });
            }
        });
    }
}

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
