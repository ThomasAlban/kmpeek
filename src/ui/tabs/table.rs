use crate::{
    ui::util::{drag_vec3, num_edit, rotation_edit},
    viewer::{
        kmp::{
            components::{EnemyPathPoint, StartPoint},
            sections::{KmpEditMode, KmpModelSections},
        },
        transform::select::Selected,
    },
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{self, Align, Layout, Sense};
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

struct KmpTable;
impl KmpTable {
    fn show(ui: &mut egui::Ui, columns: Vec<(String, f32)>, show_body: impl FnOnce(TableBody)) {
        let mut table_builder = TableBuilder::new(ui)
            .striped(true)
            .sense(Sense::click())
            .cell_layout(Layout::left_to_right(Align::Center))
            .column(Column::exact(50.));

        for (_, width) in columns.iter() {
            table_builder = table_builder.column(Column::exact(*width));
        }

        let table = table_builder.header(20., |mut header| {
            header.col(|ui| {
                ui.label("Selected");
            });
            for (header_name, _) in columns {
                header.col(|ui| {
                    ui.label(header_name);
                });
            }
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

            let mut select_checkbox = is_selected;

            let mut select_checkbox_changed = false;
            row.col(|ui| {
                ui.with_layout(
                    Layout::centered_and_justified(egui::Direction::TopDown),
                    |ui| {
                        select_checkbox_changed = ui.checkbox(&mut select_checkbox, "").changed();
                    },
                );
            });

            show_row(&mut row);

            if select_checkbox_changed {
                if select_checkbox {
                    commands.entity(entity).insert(Selected);
                } else {
                    commands.entity(entity).remove::<Selected>();
                }
            } else if row.response().clicked() {
                if is_selected {
                    commands.entity(entity).remove::<Selected>();
                } else {
                    commands.entity(entity).insert(Selected);
                }
            }
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
            ("Position".into(), 250.),
            ("Rotation".into(), 200.),
            ("Player Index".into(), 200.),
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
                        num_edit(ui, &mut start_point.player_index, 0.05, None, None, None);
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
        let headers = vec![("Position".into(), 250.), ("Leniency".into(), 100.)];
        KmpTable::show(ui, headers, |mut body| {
            for (mut transform, mut enemy_point, is_selected, entity) in q.iter_mut() {
                KmpTable::make_row(&mut body, commands, is_selected, entity, |row| {
                    row.col(|ui| {
                        drag_vec3(ui, &mut transform.translation, 10.);
                    });
                    row.col(|ui| {
                        num_edit(ui, &mut enemy_point.leniency, 0.05, None, None, None);
                    });
                });
            }
        });
    }
}
