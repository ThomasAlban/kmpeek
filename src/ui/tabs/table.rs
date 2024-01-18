use crate::{
    ui::util::{combobox_enum, drag_vec3, rotation_edit},
    viewer::{
        kmp::{
            components::{EnemyPathPoint, StartPoint},
            sections::{KmpEditMode, KmpModelSections},
        },
        transform::select::Selected,
    },
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{self, DragValue, Layout, Sense};
use egui_extras::{Column, TableBody, TableBuilder, TableRow};

use super::UiSubSection;

#[derive(SystemParam)]
pub struct ShowTableTab<'w, 's> {
    kmp_edit_mode: Res<'w, KmpEditMode>,
    commands: Commands<'w, 's>,
    q: ParamSet<
        'w,
        's,
        (
            Query<
                'w,
                's,
                (
                    &'static mut Transform,
                    &'static mut StartPoint,
                    Has<Selected>,
                    Entity,
                ),
            >,
            Query<
                'w,
                's,
                (
                    &'static mut Transform,
                    &'static mut EnemyPathPoint,
                    Has<Selected>,
                    Entity,
                ),
            >,
        ),
    >,
}
impl UiSubSection for ShowTableTab<'_, '_> {
    fn show(&mut self, ui: &mut bevy_egui::egui::Ui) {
        match self.kmp_edit_mode.0 {
            KmpModelSections::StartPoints => {
                StartPointTable::show(ui, &mut self.q.p0(), &mut self.commands)
            }
            KmpModelSections::EnemyPaths => {
                EnemyPathTable::show(ui, &mut self.q.p1(), &mut self.commands);
            }
            _ => (),
        }
    }
}

// boilerplate for constructing a kmp table
struct KmpTable;
impl KmpTable {
    fn show(ui: &mut egui::Ui, columns: Vec<(String, f32)>, show_body: impl FnOnce(TableBody)) {
        let mut table_builder = TableBuilder::new(ui)
            .striped(true)
            .vscroll(false)
            // .sense(Sense::click())
            .cell_layout(Layout::centered_and_justified(egui::Direction::TopDown))
            .column(Column::exact(50.));

        for (_, width) in columns.iter() {
            table_builder = table_builder.column(Column::exact(*width));
        }
        // empty row filling remaining space
        table_builder = table_builder.column(Column::remainder());

        let table = table_builder.header(20., |mut header| {
            header.col(|ui| {
                ui.label("Selected");
            });
            for (header_name, _) in columns {
                header.col(|ui| {
                    ui.label(header_name);
                });
            }
            // empty header (which fills remaining space on the right)
            header.col(|_| {});
        });
        table.body(show_body);
    }
    fn make_row(
        body: &mut TableBody,
        commands: &mut Commands,
        is_selected: bool,
        entity: Entity,
        show_row: impl FnOnce(&mut TableRow),
    ) {
        body.row(20., |mut row| {
            row.set_selected(is_selected);

            // show the 'select' ui (which is the same for every KMP table)
            let mut select_checkbox = is_selected;
            let mut select_checkbox_changed = false;
            row.col(|ui| {
                select_checkbox_changed = ui.checkbox(&mut select_checkbox, "").changed();
            });

            // call whatever show_row function we passed in
            show_row(&mut row);

            // extra blank row that fills up whatever remaining space there is
            row.col(|_| {});

            if select_checkbox_changed {
                if select_checkbox {
                    commands.entity(entity).insert(Selected);
                } else {
                    commands.entity(entity).remove::<Selected>();
                }
            }
            // else if row.response().clicked() {
            //     if is_selected {
            //         commands.entity(entity).remove::<Selected>();
            //     } else {
            //         commands.entity(entity).insert(Selected);
            //     }
            // }
        });
    }
}

struct StartPointTable;
impl StartPointTable {
    fn show(
        ui: &mut egui::Ui,
        q: &mut Query<(&mut Transform, &mut StartPoint, Has<Selected>, Entity), ()>,
        commands: &mut Commands,
    ) {
        let headers = vec![
            ("Position".into(), 200.),
            ("Rotation".into(), 200.),
            ("Player Index".into(), 100.),
        ];
        KmpTable::show(ui, headers, |mut body| {
            for (mut transform, mut start_point, is_selected, entity) in q.iter_mut() {
                KmpTable::make_row(&mut body, commands, is_selected, entity, |row| {
                    row.col(|ui| {
                        drag_vec3(ui, &mut transform.translation, 10.);
                    });
                    row.col(|ui| {
                        rotation_edit(ui, &mut transform, 1.);
                    });
                    row.col(|ui| {
                        ui.with_layout(
                            Layout::centered_and_justified(egui::Direction::TopDown),
                            |ui| {
                                ui.add(DragValue::new(&mut start_point.player_index).speed(0.05));
                            },
                        );
                    });
                });
            }
        });
    }
}

struct EnemyPathTable;
impl EnemyPathTable {
    fn show(
        ui: &mut egui::Ui,
        q: &mut Query<(&mut Transform, &mut EnemyPathPoint, Has<Selected>, Entity), ()>,
        commands: &mut Commands,
    ) {
        let headers = vec![
            ("Position".into(), 250.),
            ("Leniency".into(), 75.),
            ("Setting 1".into(), 150.),
            ("Setting 2".into(), 150.),
            ("Setting 3".into(), 75.),
        ];
        KmpTable::show(ui, headers, |mut body| {
            for (mut transform, mut enemy_point, is_selected, entity) in q.iter_mut() {
                KmpTable::make_row(&mut body, commands, is_selected, entity, |row| {
                    row.col(|ui| {
                        drag_vec3(ui, &mut transform.translation, 10.);
                    });
                    row.col(|ui| {
                        ui.add(DragValue::new(&mut enemy_point.leniency).speed(0.05));
                    });
                    row.col(|ui| {
                        let id = format!("enpt_setting_1:{:?}", entity);
                        combobox_enum(ui, &mut enemy_point.setting_1, id, None);
                    });
                    row.col(|ui| {
                        let id = format!("enpt_setting_2:{:?}", entity);
                        combobox_enum(ui, &mut enemy_point.setting_2, id, None);
                    });
                    row.col(|ui| {
                        ui.add(DragValue::new(&mut enemy_point.setting_3).speed(0.05));
                    });
                });
            }
        });
    }
}
