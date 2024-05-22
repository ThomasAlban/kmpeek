use crate::{
    ui::util::{combobox_enum, drag_vec3, euler_to_quat, quat_to_euler, DragSpeed},
    viewer::{
        edit::select::Selected,
        kmp::{
            components::{
                AreaPoint, BattleFinishPoint, CannonPoint, CheckpointLeft, EnemyPathPoint, ItemPathPoint, KmpCamera,
                Object, RespawnPoint, StartPoint,
            },
            sections::{KmpEditMode, KmpSections},
        },
    },
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{self, emath::Numeric, Checkbox, Direction, DragValue, Layout, Response, Ui};
use egui_extras::{Column, TableBuilder, TableRow};

use super::UiSubSection;

type KmpTableQuery<'w, 's, C> = Query<'w, 's, (&'static mut C, &'static mut Transform, Entity, Has<Selected>)>;

#[derive(SystemParam)]
pub struct ShowTableTab<'w, 's> {
    commands: Commands<'w, 's>,
    edit_mode: Res<'w, KmpEditMode>,
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
                    KmpTableQuery<'w, 's, CheckpointLeft>,
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
                let _ = ui.button("+");
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
                    checkbox_column(row, &mut item.path_start_override);
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
                    checkbox_column(row, &mut item.path_start_override);
                }),
            Checkpoints => KmpTable::new(ui, &mut self.commands, self.q.p0().p3().iter_mut())
                .columns(["Type", "Always Path Start"])
                .no_rotation()
                .no_y_translation()
                .show(|row, item| {
                    combobox_column(row, &mut item.kind);
                    checkbox_column(row, &mut item.path_start_override);
                }),
            RespawnPoints => KmpTable::new(ui, &mut self.commands, self.q.p0().p4().iter_mut())
                .columns(["ID", "Sound Trigger"])
                .show(|row, item| {
                    drag_value_column(row, Slow, &mut item.id);
                    drag_value_column(row, Slow, &mut item.sound_trigger);
                }),
            _ => (),
        }
    }
}

struct KmpTable<'a, 'w, 's, T: Component + PartialEq + Clone> {
    ui: &'a mut Ui,
    commands: &'a mut Commands<'w, 's>,
    columns: Vec<&'static str>,
    items: Vec<(Mut<'a, T>, Mut<'a, Transform>, Entity, bool)>,
    show_rotation: bool,
    show_y_translation: bool,
}
impl<'a, 'w, 's, T: Component + PartialEq + Clone> KmpTable<'a, 'w, 's, T> {
    fn new(
        ui: &'a mut Ui,
        commands: &'a mut Commands<'w, 's>,
        items: impl Iterator<Item = (Mut<'a, T>, Mut<'a, Transform>, Entity, bool)>,
    ) -> Self {
        Self {
            ui,
            commands,
            columns: vec![],
            items: items.into_iter().collect(),
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
            .column(Column::exact(50.))
            // translation column
            .column(Column::auto().resizable(true));
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
            for (mut t, mut transform, e, is_selected) in self.items {
                body.row(20., |mut row| {
                    row.set_selected(is_selected);

                    // show the 'select' ui (which is the same for every KMP table)
                    let mut select_checkbox = is_selected;
                    let mut select_checkbox_changed = false;
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

fn drag_value_column<T: Numeric>(row: &mut TableRow, speed: impl Into<f64>, item: &mut T) -> Response {
    row.col(|ui| {
        ui.with_layout(Layout::centered_and_justified(Direction::TopDown), |ui| {
            ui.add(DragValue::new(item).speed(speed));
        });
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
