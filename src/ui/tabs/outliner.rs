use super::UiSubSection;
use crate::{
    ui::util::view_icon_btn,
    viewer::{
        kmp::{path::EntityGroup, Kmp},
        transform::select::Selected,
    },
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{collapsing_header, Ui};

const VIEW_BTN_SIZE: f32 = 15.;

#[derive(SystemParam)]
pub struct ShowOutlinerTab<'w, 's> {
    commands: Commands<'w, 's>,
    kmp: ResMut<'w, Kmp>,
    q_visibility: Query<'w, 's, &'static mut Visibility>,
    q_is_selected: Query<'w, 's, Has<Selected>>,
    q_all_selected: Query<'w, 's, Entity, With<Selected>>,
    keys: Res<'w, Input<KeyCode>>,
}
impl UiSubSection for ShowOutlinerTab<'_, '_> {
    fn show(&mut self, ui: &mut bevy_egui::egui::Ui) {
        let kmp = self.kmp.clone();
        self.show_point_outliner(ui, "Start Points", &kmp.start_points);

        self.show_path_outliner(ui, "Enemy Paths", &kmp.enemy_paths);

        self.show_path_outliner(ui, "Item Paths", &kmp.item_paths);

        // todo: checkpoints

        self.show_point_outliner(ui, "Respawn Points", &kmp.respawn_points);

        self.show_point_outliner(ui, "Objects", &kmp.objects);

        self.show_point_outliner(ui, "Areas", &kmp.areas);

        self.show_point_outliner(ui, "Cameras", &kmp.cameras);

        // todo: cannon points

        // todo: battle finish points
    }
}

impl ShowOutlinerTab<'_, '_> {
    fn show_point_outliner(&mut self, ui: &mut Ui, name: impl Into<String>, entities: &[Entity]) {
        let name: String = name.into();
        let mut children_visible = Vec::new();
        let mut children_selected = Vec::new();

        for entity in entities.iter() {
            let visibility = self.q_visibility.get(*entity).unwrap();
            let is_selected = self.q_is_selected.get(*entity).unwrap();
            children_visible.push(visibility == Visibility::Visible);
            children_selected.push(is_selected);
        }
        collapsing_header::CollapsingState::load_with_default_open(
            ui.ctx(),
            name.clone().into(),
            false,
        )
        .show_header(ui, |ui| {
            let mut any_visible = children_visible.iter().any(|e| *e);
            let mut all_selected = children_selected.iter().all(|e| *e);
            let view_btn = view_icon_btn(ui, &mut any_visible, VIEW_BTN_SIZE);
            if view_btn.changed() {
                for entity in entities.iter() {
                    self.set_visibility(*entity, any_visible);
                }
            }
            let toggle_all_selected = ui.toggle_value(&mut all_selected, name);
            if toggle_all_selected.changed() {
                self.point_header_set_selected(all_selected, entities);
            }
        })
        .body(|ui| {
            self.show_points_list(ui, entities);
        });
    }
    fn show_path_outliner(
        &mut self,
        ui: &mut Ui,
        name: impl Into<String>,
        entity_groups: &[EntityGroup],
    ) {
        let name: String = name.into();
        let mut children_visible = Vec::new();
        let mut children_selected = Vec::new();

        for entity_group in entity_groups.iter() {
            let mut entity_group_visibilities = Vec::new();
            let mut entity_group_selected = Vec::new();
            for entity in entity_group.entities.iter() {
                let visibility = self.q_visibility.get(*entity).unwrap();
                let is_selected = self.q_is_selected.get(*entity).unwrap();

                entity_group_visibilities.push(visibility == Visibility::Visible);
                entity_group_selected.push(is_selected);
            }
            children_visible.push(entity_group_visibilities);
            children_selected.push(entity_group_selected);
        }

        collapsing_header::CollapsingState::load_with_default_open(
            ui.ctx(),
            name.clone().into(),
            false,
        )
        .show_header(ui, |ui| {
            let mut any_visible = children_visible.iter().any(|e| e.iter().any(|e| *e));
            let mut all_selected = children_selected.iter().all(|e| e.iter().all(|e| *e));
            let view_btn = view_icon_btn(ui, &mut any_visible, VIEW_BTN_SIZE);
            if view_btn.changed() {
                for entity_group in entity_groups.iter() {
                    for entity in entity_group.entities.iter() {
                        self.set_visibility(*entity, any_visible);
                    }
                }
            }
            let toggle_all_selected = ui.toggle_value(&mut all_selected, name);

            if toggle_all_selected.changed() {
                self.path_header_set_selected(all_selected, entity_groups);
            }
        })
        .body(|ui| {
            for (i, entity_group) in entity_groups.iter().enumerate() {
                self.show_point_outliner(ui, format!("Path {i}"), &entity_group.entities);
            }
        });
    }

    fn show_points_list(&mut self, ui: &mut Ui, entities: &[Entity]) {
        for (i, entity) in entities.iter().enumerate() {
            let mut visibility = self.q_visibility.get_mut(*entity).unwrap();
            let mut is_selected = self.q_is_selected.get(*entity).unwrap();
            let mut is_visible = *visibility == Visibility::Visible;

            let (view_btn, toggle_val) = ui
                .horizontal(|ui| {
                    let view_btn = view_icon_btn(ui, &mut is_visible, VIEW_BTN_SIZE);
                    let toggle_val = ui.toggle_value(&mut is_selected, format!("Point {i}"));
                    (view_btn, toggle_val)
                })
                .inner;
            if view_btn.changed() {
                if is_visible {
                    *visibility = Visibility::Visible;
                } else {
                    *visibility = Visibility::Hidden;
                }
            }
            if toggle_val.changed() {
                self.point_set_selected(is_selected, *entity);
            }
        }
    }

    fn set_visibility(&mut self, entity: Entity, visible: bool) {
        *self.q_visibility.get_mut(entity).unwrap() = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    fn path_header_set_selected(&mut self, all_selected: bool, entity_groups: &[EntityGroup]) {
        if !self.keys.pressed(KeyCode::ShiftLeft) {
            for selected in self.q_all_selected.iter() {
                self.commands.entity(selected).remove::<Selected>();
            }
        }

        for entity_group in entity_groups.iter() {
            for entity in entity_group.entities.iter() {
                if all_selected {
                    self.commands.entity(*entity).insert(Selected);
                } else {
                    self.commands.entity(*entity).remove::<Selected>();
                }
            }
        }
    }

    fn point_header_set_selected(&mut self, all_selected: bool, entities: &[Entity]) {
        if !self.keys.pressed(KeyCode::ShiftLeft) {
            for selected in self.q_all_selected.iter() {
                self.commands.entity(selected).remove::<Selected>();
            }
        }

        for entity in entities.iter() {
            if all_selected {
                self.commands.entity(*entity).insert(Selected);
            } else {
                self.commands.entity(*entity).remove::<Selected>();
            }
        }
    }
    fn point_set_selected(&mut self, selected: bool, entity: Entity) {
        if !self.keys.pressed(KeyCode::ShiftLeft) {
            for selected in self.q_all_selected.iter() {
                self.commands.entity(selected).remove::<Selected>();
            }
        }
        if selected {
            self.commands.entity(entity).insert(Selected);
        } else {
            self.commands.entity(entity).remove::<Selected>();
        }
    }
}
