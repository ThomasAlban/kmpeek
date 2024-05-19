use crate::{
    ui::util::{drag_vec3, euler_to_quat, quat_to_euler, DragSpeed},
    viewer::{edit::select::Selected, kmp::components::StartPoint},
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{self, DragValue, Layout, Ui};
use egui_extras::{Column, TableBuilder, TableRow};

use super::UiSubSection;

#[derive(SystemParam)]
pub struct ShowTableTab<'w, 's> {
    commands: Commands<'w, 's>,
    q_start_point: Query<'w, 's, (&'static mut StartPoint, &'static mut Transform, Entity, Has<Selected>)>,
}
impl UiSubSection for ShowTableTab<'_, '_> {
    fn show(&mut self, ui: &mut Ui) {
        kmp_table(
            ui,
            &mut self.commands,
            vec![("Player Index", 100.)],
            true,
            false,
            &mut self.q_start_point.iter_mut(),
            |row, item| {
                row.col(|ui| {
                    ui.with_layout(Layout::centered_and_justified(egui::Direction::TopDown), |ui| {
                        ui.add(DragValue::new(&mut item.player_index).speed(0.05));
                    });
                });
            },
        );

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

fn kmp_table<'a, T: Component + PartialEq + Clone>(
    ui: &mut Ui,
    commands: &mut Commands,
    columns: Vec<(&'static str, f32)>,
    show_rotation: bool,
    show_y_translation: bool,
    items: impl Iterator<Item = (Mut<'a, T>, Mut<'a, Transform>, Entity, bool)>,
    mut contents: impl FnMut(&mut TableRow, &mut T),
) {
    let trans_width = if show_y_translation { 200. } else { 150. };
    let mut table_builder = TableBuilder::new(ui)
        .striped(true)
        .vscroll(false)
        // .sense(Sense::click())
        .cell_layout(Layout::centered_and_justified(egui::Direction::TopDown))
        // select column
        .column(Column::exact(50.))
        // translation column
        .column(Column::exact(trans_width))
        // rotation column
        .column(Column::exact(200.));

    for (_, width) in columns.iter() {
        table_builder = table_builder.column(Column::exact(*width));
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
        if show_rotation {
            header.col(|ui| {
                ui.label("Rotation");
            });
        }
        for (name, _) in columns.iter() {
            header.col(|ui| {
                ui.label(*name);
            });
        }
        // empty header (which fills remaining space on the right)
        header.col(|_| {});
    });
    table.body(|mut body| {
        for (mut t, mut transform, e, is_selected) in items {
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
                    let (num_cols, z_ix) = if show_y_translation { (3, 2) } else { (2, 1) };
                    ui.columns(num_cols, |ui| {
                        ui[0].centered_and_justified(|ui| {
                            ui.add(
                                egui::DragValue::new(&mut value.x)
                                    .speed(DragSpeed::Fast)
                                    .prefix("X: ")
                                    .fixed_decimals(1),
                            )
                        });
                        if show_y_translation {
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
                let mut rot = quat_to_euler(&transform_cp);
                row.col(|ui| {
                    let res = drag_vec3(ui, &mut rot, DragSpeed::Slow);
                    euler_to_quat(rot, res, &mut transform_cp);
                });

                contents(&mut row, &mut t_cp);

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
            });
        }
    });
}

// // boilerplate for constructing a kmp table
// trait KmpTable<'a> {
//     fn columns(&self) -> Vec<KmpTableColumn>;
//     fn transform_opts() -> TransformEditOptions;
//     fn iter_entities(&mut self) -> impl Iterator<Item = Entity>;
//     fn show(
//         &mut self,
//         ui: &mut egui::Ui,
//         mut get_selected: impl FnMut(Entity) -> bool,
//         mut set_selected: impl FnMut(Entity, bool),
//     ) {
//         let mut table_builder = TableBuilder::new(ui)
//             .striped(true)
//             .vscroll(false)
//             // .sense(Sense::click())
//             .cell_layout(Layout::centered_and_justified(egui::Direction::TopDown))
//             .column(Column::exact(50.));

//         for width in self.columns().iter().map(|x| x.width) {
//             table_builder = table_builder.column(Column::exact(width));
//         }
//         // empty row filling remaining space
//         table_builder = table_builder.column(Column::remainder());

//         let table = table_builder.header(20., |mut header| {
//             header.col(|ui| {
//                 ui.label("Selected");
//             });
//             for name in self.columns().iter().map(|x| x.name.clone()) {
//                 header.col(|ui| {
//                     ui.label(name);
//                 });
//             }
//             // empty header (which fills remaining space on the right)
//             header.col(|_| {});
//         });
//         table.body(|mut body| {
//             for e in self.iter_entities() {
//                 body.row(20., |mut row| {
//                     let is_selected = get_selected(e);
//                     row.set_selected(is_selected);

//                     // show the 'select' ui (which is the same for every KMP table)
//                     let mut select_checkbox = is_selected;
//                     let mut select_checkbox_changed = false;
//                     row.col(|ui| {
//                         select_checkbox_changed = ui.checkbox(&mut select_checkbox, "").changed();
//                     });

//                     row.col(|ui| {
//                         for col in self.columns().iter_mut() {
//                             (*col.contents)(ui);
//                         }
//                         //ttt
//                     });

//                     // extra blank row that fills up whatever remaining space there is
//                     row.col(|_| {});

//                     if select_checkbox_changed {
//                         set_selected(e, select_checkbox);
//                     }
//                 });
//             }
//         });
//     }
//     fn make_row(
//         body: &mut TableBody,
//         commands: &mut Commands,
//         is_selected: bool,
//         entity: Entity,
//         show_row: impl FnOnce(&mut TableRow),
//     ) {
//         body.row(20., |mut row| {
//             row.set_selected(is_selected);

//             // show the 'select' ui (which is the same for every KMP table)
//             let mut select_checkbox = is_selected;
//             let mut select_checkbox_changed = false;
//             row.col(|ui| {
//                 select_checkbox_changed = ui.checkbox(&mut select_checkbox, "").changed();
//             });

//             // call whatever show_row function we passed in
//             show_row(&mut row);

//             // extra blank row that fills up whatever remaining space there is
//             row.col(|_| {});

//             if select_checkbox_changed {
//                 if select_checkbox {
//                     commands.entity(entity).insert(Selected);
//                 } else {
//                     commands.entity(entity).remove::<Selected>();
//                 }
//             }
//         });
//     }
// }

// struct StartPointTable;
// impl StartPointTable {
//     fn show(
//         ui: &mut egui::Ui,
//         commands: &mut Commands,
//         start_points: &[Entity],
//         q_transform: &mut Query<&mut Transform>,
//         q_is_selected: &mut Query<Has<Selected>>,
//         q_start_point: &mut Query<&mut StartPoint>,
//     ) {
//         let headers = vec![
//             ("Position".into(), 200.),
//             ("Rotation".into(), 200.),
//             ("Player Index".into(), 100.),
//         ];
//         KmpTable::show(ui, headers, |mut body| {
//             for entity in start_points.iter() {
//                 let mut transform = q_transform.get_mut(*entity).unwrap();
//                 let is_selected = q_is_selected.get(*entity).unwrap();
//                 let mut start_point = q_start_point.get_mut(*entity).unwrap();

//                 KmpTable::make_row(&mut body, commands, is_selected, *entity, |row| {
//                     let mut transform_cp = *transform;
//                     let mut start_point_cp = *start_point;

//                     row.col(|ui| {
//                         drag_vec3(ui, &mut transform_cp.translation, 10.);
//                     });
//                     row.col(|_ui| {
//                         // rotation_edit(ui, &mut transform_cp, 1.);
//                     });
//                     row.col(|ui| {
//                         ui.with_layout(Layout::centered_and_justified(egui::Direction::TopDown), |ui| {
//                             ui.add(DragValue::new(&mut start_point_cp.player_index).speed(0.05));
//                         });
//                     });

//                     transform.set_if_neq(transform_cp);
//                     start_point.set_if_neq(start_point_cp);
//                 });
//             }
//         });
//     }
// }

// struct EnemyPathTable;
// impl EnemyPathTable {
//     fn show(
//         ui: &mut egui::Ui,
//         commands: &mut Commands,
//         enemy_paths: &[EntityGroup],
//         q_transform: &mut Query<&mut Transform>,
//         q_is_selected: &mut Query<Has<Selected>>,
//         q_enemy_path_point: &mut Query<&mut EnemyPathPoint>,
//     ) {
//         let headers = vec![
//             ("Position".into(), 250.),
//             ("Leniency".into(), 75.),
//             ("Setting 1".into(), 150.),
//             ("Setting 2".into(), 150.),
//             ("Setting 3".into(), 75.),
//         ];
//         KmpTable::show(ui, headers, |mut body| {
//             for entity_group in enemy_paths.iter() {
//                 for entity in entity_group.entities.iter() {
//                     let mut transform = q_transform.get_mut(*entity).unwrap();
//                     let is_selected = q_is_selected.get(*entity).unwrap();
//                     let mut enemy_path_point = q_enemy_path_point.get_mut(*entity).unwrap();

//                     KmpTable::make_row(&mut body, commands, is_selected, *entity, |row| {
//                         let mut transform_cp = *transform;
//                         let mut enemy_path_point_cp = *enemy_path_point;

//                         row.col(|ui| {
//                             drag_vec3(ui, &mut transform_cp.translation, 10.);
//                         });
//                         row.col(|ui| {
//                             ui.add(DragValue::new(&mut enemy_path_point_cp.leniency).speed(0.05));
//                         });
//                         row.col(|ui| {
//                             combobox_enum(ui, &mut enemy_path_point_cp.setting_1, None);
//                         });
//                         row.col(|ui| {
//                             combobox_enum(ui, &mut enemy_path_point_cp.setting_2, None);
//                         });
//                         row.col(|ui| {
//                             ui.add(DragValue::new(&mut enemy_path_point_cp.setting_3).speed(0.05));
//                         });

//                         transform.set_if_neq(transform_cp);
//                         enemy_path_point.set_if_neq(enemy_path_point_cp);
//                     });
//                 }
//             }
//         });
//     }
// }
