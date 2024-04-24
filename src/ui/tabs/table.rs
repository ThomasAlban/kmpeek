#![allow(dead_code)]

use crate::{
    ui::util::{combobox_enum, drag_vec3},
    viewer::{
        edit::select::Selected,
        kmp::{
            components::{EnemyPathPoint, StartPoint},
            path::EntityGroup,
            sections::KmpEditMode,
        },
    },
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{self, DragValue, Layout, Ui};
use egui_extras::{Column, TableBody, TableBuilder, TableRow};

use super::UiSubSection;

#[derive(SystemParam)]
pub struct ShowTableTab<'w, 's> {
    kmp_edit_mode: Res<'w, KmpEditMode>,
    commands: Commands<'w, 's>,

    q_transform: Query<'w, 's, &'static mut Transform>,
    q_is_selected: Query<'w, 's, Has<Selected>>,

    q_start_point: Query<'w, 's, &'static mut StartPoint>,
    q_enemy_path_point: Query<'w, 's, &'static mut EnemyPathPoint>,
}
impl UiSubSection for ShowTableTab<'_, '_> {
    fn show(&mut self, ui: &mut Ui) {
        // match self.kmp_edit_mode.0 {
        //     KmpModelSections::StartPoints => StartPointTable::show(
        //         ui,
        //         &mut self.commands,
        //         &self.kmp.start_points,
        //         &mut self.q_transform,
        //         &mut self.q_is_selected,
        //         &mut self.q_start_point,
        //     ),
        //     KmpModelSections::EnemyPaths => EnemyPathTable::show(
        //         ui,
        //         &mut self.commands,
        //         &self.kmp.enemy_paths,
        //         &mut self.q_transform,
        //         &mut self.q_is_selected,
        //         &mut self.q_enemy_path_point,
        //     ),
        //     _ => (),
        // }
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
        });
    }
}

struct StartPointTable;
impl StartPointTable {
    fn show(
        ui: &mut egui::Ui,
        commands: &mut Commands,
        start_points: &[Entity],
        q_transform: &mut Query<&mut Transform>,
        q_is_selected: &mut Query<Has<Selected>>,
        q_start_point: &mut Query<&mut StartPoint>,
    ) {
        let headers = vec![
            ("Position".into(), 200.),
            ("Rotation".into(), 200.),
            ("Player Index".into(), 100.),
        ];
        KmpTable::show(ui, headers, |mut body| {
            for entity in start_points.iter() {
                let mut transform = q_transform.get_mut(*entity).unwrap();
                let is_selected = q_is_selected.get(*entity).unwrap();
                let mut start_point = q_start_point.get_mut(*entity).unwrap();

                KmpTable::make_row(&mut body, commands, is_selected, *entity, |row| {
                    let mut transform_cp = *transform;
                    let mut start_point_cp = *start_point;

                    row.col(|ui| {
                        drag_vec3(ui, &mut transform_cp.translation, 10.);
                    });
                    row.col(|ui| {
                        // rotation_edit(ui, &mut transform_cp, 1.);
                    });
                    row.col(|ui| {
                        ui.with_layout(Layout::centered_and_justified(egui::Direction::TopDown), |ui| {
                            ui.add(DragValue::new(&mut start_point_cp.player_index).speed(0.05));
                        });
                    });

                    transform.set_if_neq(transform_cp);
                    start_point.set_if_neq(start_point_cp);
                });
            }
        });
    }
}

struct EnemyPathTable;
impl EnemyPathTable {
    fn show(
        ui: &mut egui::Ui,
        commands: &mut Commands,
        enemy_paths: &[EntityGroup],
        q_transform: &mut Query<&mut Transform>,
        q_is_selected: &mut Query<Has<Selected>>,
        q_enemy_path_point: &mut Query<&mut EnemyPathPoint>,
    ) {
        let headers = vec![
            ("Position".into(), 250.),
            ("Leniency".into(), 75.),
            ("Setting 1".into(), 150.),
            ("Setting 2".into(), 150.),
            ("Setting 3".into(), 75.),
        ];
        KmpTable::show(ui, headers, |mut body| {
            for entity_group in enemy_paths.iter() {
                for entity in entity_group.entities.iter() {
                    let mut transform = q_transform.get_mut(*entity).unwrap();
                    let is_selected = q_is_selected.get(*entity).unwrap();
                    let mut enemy_path_point = q_enemy_path_point.get_mut(*entity).unwrap();

                    KmpTable::make_row(&mut body, commands, is_selected, *entity, |row| {
                        let mut transform_cp = *transform;
                        let mut enemy_path_point_cp = *enemy_path_point;

                        row.col(|ui| {
                            drag_vec3(ui, &mut transform_cp.translation, 10.);
                        });
                        row.col(|ui| {
                            ui.add(DragValue::new(&mut enemy_path_point_cp.leniency).speed(0.05));
                        });
                        row.col(|ui| {
                            let id = format!("enpt_setting_1:{:?}", entity);
                            combobox_enum(ui, &mut enemy_path_point_cp.setting_1, id, None);
                        });
                        row.col(|ui| {
                            let id = format!("enpt_setting_2:{:?}", entity);
                            combobox_enum(ui, &mut enemy_path_point_cp.setting_2, id, None);
                        });
                        row.col(|ui| {
                            ui.add(DragValue::new(&mut enemy_path_point_cp.setting_3).speed(0.05));
                        });

                        transform.set_if_neq(transform_cp);
                        enemy_path_point.set_if_neq(enemy_path_point_cp);
                    });
                }
            }
        });
    }
}
