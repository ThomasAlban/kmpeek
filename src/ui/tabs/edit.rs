#![allow(clippy::redundant_closure_call)]

use super::UiSubSection;
use crate::{
    ui::util::{
        framed_collapsing_header,
        multi_edit::{checkbox_multi_edit, combobox_enum_multi_edit, drag_value_multi_edit, rotation_multi_edit},
    },
    viewer::{
        edit::select::Selected,
        kmp::components::{EnemyPathPoint, ItemPathPoint, StartPoint},
    },
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{self, Layout, Ui, WidgetText};

#[derive(SystemParam)]
pub struct ShowEditTab<'w, 's> {
    q_transform: Query<'w, 's, &'static mut Transform, With<Selected>>,
    q_start_point: Query<'w, 's, &'static mut StartPoint, With<Selected>>,
    q_enemy_point: Query<'w, 's, &'static mut EnemyPathPoint, With<Selected>>,
    q_item_point: Query<'w, 's, &'static mut ItemPathPoint, With<Selected>>,
}
impl UiSubSection for ShowEditTab<'_, '_> {
    fn show(&mut self, ui: &mut Ui) {
        edit_component("Transform", self.q_transform.iter_mut(), ui, |ui, transforms| {
            edit_row("Translation X", true, ui, |ui| {
                drag_value_multi_edit(ui, transforms.iter_mut().map(|x| &mut x.translation.x));
            });
            edit_row("Y", true, ui, |ui| {
                drag_value_multi_edit(ui, transforms.iter_mut().map(|x| &mut x.translation.y));
            });
            edit_row("Z", true, ui, |ui| {
                drag_value_multi_edit(ui, transforms.iter_mut().map(|x| &mut x.translation.z));
            });
            edit_spacing(ui);
            rotation_multi_edit(ui, transforms, |ui, rots| {
                let x = edit_row("Rotation X", true, ui, |ui| {
                    drag_value_multi_edit(ui, rots.iter_mut().map(|r| &mut r.x))
                });
                let y = edit_row("Y", true, ui, |ui| {
                    drag_value_multi_edit(ui, rots.iter_mut().map(|r| &mut r.y))
                });
                let z = edit_row("Z", true, ui, |ui| {
                    drag_value_multi_edit(ui, rots.iter_mut().map(|r| &mut r.z))
                });
                (x, y, z)
            });
        });

        edit_component("Start Point", self.q_start_point.iter_mut(), ui, |ui, start_points| {
            edit_row("Player Index", true, ui, |ui| {
                drag_value_multi_edit(ui, start_points.iter_mut().map(|x| &mut x.player_index));
            });
        });

        edit_spacing(ui);

        edit_component("Enemy Point", self.q_enemy_point.iter_mut(), ui, |ui, enemy_points| {
            edit_row("Leniency", true, ui, |ui| {
                drag_value_multi_edit(ui, enemy_points.iter_mut().map(|x| &mut x.leniency));
            });
            edit_spacing(ui);
            edit_row("Setting 1", true, ui, |ui| {
                combobox_enum_multi_edit(ui, "enpt_s1", None, enemy_points.iter_mut().map(|x| &mut x.setting_1));
            });
            edit_row("Setting 2", true, ui, |ui| {
                combobox_enum_multi_edit(ui, "enpt_s2", None, enemy_points.iter_mut().map(|x| &mut x.setting_2));
            });
            edit_row("Setting 3", true, ui, |ui| {
                drag_value_multi_edit(ui, enemy_points.iter_mut().map(|x| &mut x.setting_3));
            });
        });

        edit_component("Item Point", self.q_item_point.iter_mut(), ui, |ui, item_points| {
            edit_row("Bullet Bill Control", true, ui, |ui| {
                drag_value_multi_edit(ui, item_points.iter_mut().map(|x| &mut x.bullet_control));
            });
            edit_spacing(ui);
            edit_row("Bullet Height", false, ui, |ui| {
                combobox_enum_multi_edit(
                    ui,
                    "itpt_s1",
                    None,
                    item_points.iter_mut().map(|x| &mut x.bullet_height),
                );
            });
            edit_row("Bullet Can't Drop", false, ui, |ui| {
                checkbox_multi_edit(ui, item_points.iter_mut().map(|x| &mut x.bullet_cant_drop));
            });
            edit_row("Low Shell Priority", false, ui, |ui| {
                checkbox_multi_edit(ui, item_points.iter_mut().map(|x| &mut x.low_shell_priority));
            });
        });
    }
}

fn edit_component<'a, T: 'a + PartialEq + Clone, R>(
    title: &'static str,
    items: impl IntoIterator<Item = Mut<'a, T>>,
    ui: &mut Ui,
    add_body: impl FnOnce(&mut Ui, &mut [T]) -> R + Copy,
) {
    edit_many_mut(items, |items| {
        let title = if items.len() > 1 {
            format!("{} ({})", title, items.len())
        } else {
            title.into()
        };
        framed_collapsing_header(title, ui, |ui| add_body(ui, items));
    });
    edit_spacing(ui);
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
    mut contents: impl FnMut(&mut [T]),
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
