use super::select::Selected;
use crate::{
    ui::ui_state::ViewportRect,
    util::{ui_viewport_to_ndc, RaycastFromCam},
    viewer::kmp::{
        components::{EnemyPathPoint, ItemPathPoint, KmpSelectablePoint},
        path::KmpPathNode,
    },
};
use bevy::prelude::*;
use bevy_mod_raycast::prelude::*;

pub fn link_points(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    q_selected: Query<Entity, With<Selected>>,
    q_visibility: Query<&Visibility>,
    q_enemy_point: Query<Entity, With<EnemyPathPoint>>,
    q_item_point: Query<Entity, With<ItemPathPoint>>,
    q_transform: Query<&Transform, With<KmpSelectablePoint>>,
    q_camera: Query<(&Camera, &GlobalTransform)>,
    q_window: Query<&Window>,
    mut raycast: Raycast,
    viewport_rect: Res<ViewportRect>,
    q_kmp_path_node: Query<&KmpPathNode>,
) {
    if !mouse_buttons.just_pressed(MouseButton::Left) {
        return;
    }
    if !keys.pressed(KeyCode::AltLeft) && !keys.pressed(KeyCode::AltRight) {
        return;
    }
    if q_selected.is_empty() {
        return;
    }
    let Some(mouse_pos) = q_window.single().cursor_position() else {
        return;
    };
    let cam = q_camera.iter().find(|cam| cam.0.is_active).unwrap();
    let ndc_mouse_pos = ui_viewport_to_ndc(mouse_pos, viewport_rect.0);
    let ray = RaycastFromCam::new(cam, ndc_mouse_pos, &mut raycast)
        .filter(&|e| q_transform.contains(e))
        .cast();

    let Some((alt_clicked_point, _)) = ray.first() else {
        return;
    };

    if q_enemy_point.contains(*alt_clicked_point) {
        let node = q_kmp_path_node.get(*alt_clicked_point).unwrap();
        // we have just alt clicked an enemy point, link any selected enemy points to this
        for selected in q_selected.iter().filter(|e| q_enemy_point.contains(*e)) {
            let alt_clicked_point = *alt_clicked_point;
            commands.add(move |world: &mut World| {
                KmpPathNode::link_nodes(selected, alt_clicked_point, world);
            });
        }
    }
}

pub fn unlink_points(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    q_kmp_path_node: Query<&KmpPathNode>,
    q_selected: Query<Entity, With<Selected>>,
) {
    if !keys.just_pressed(KeyCode::KeyU) {
        return;
    }

    for selected in q_selected.iter() {
        let Ok(node) = q_kmp_path_node.get(selected) else {
            continue;
        };
        for prev_node_entity in node.prev_nodes.iter() {
            let prev_node_entity = *prev_node_entity;
            commands.add(move |world: &mut World| {
                KmpPathNode::unlink_nodes(prev_node_entity, selected, world);
            });
        }
        for next_node_entity in node.next_nodes.iter() {
            let next_node_entity = *next_node_entity;
            commands.add(move |world: &mut World| {
                KmpPathNode::unlink_nodes(selected, next_node_entity, world);
            });
        }
    }
}
