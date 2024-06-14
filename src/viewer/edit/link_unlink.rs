use super::select::{SelectSet, Selected};
use crate::{
    ui::viewport::ViewportInfo,
    util::{ui_viewport_to_ndc, RaycastFromCam},
    viewer::{
        camera::Gizmo2dCam,
        kmp::{
            checkpoints::get_both_cp_nodes,
            components::{CheckpointLeft, CheckpointRight, EnemyPathPoint, ItemPathPoint, KmpSelectablePoint},
            path::KmpPathNode,
        },
    },
};
use bevy::prelude::*;
use bevy_mod_raycast::prelude::*;

pub struct LinkUnlinkPlugin;
impl Plugin for LinkUnlinkPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (link_points, unlink_points).after(SelectSet));
    }
}

pub fn link_points(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    q_selected: Query<Entity, With<Selected>>,
    q_transform: Query<&Transform, With<KmpSelectablePoint>>,
    q_camera: Query<(&Camera, &GlobalTransform), Without<Gizmo2dCam>>,
    q_window: Query<&Window>,
    mut raycast: Raycast,
    viewport_info: Res<ViewportInfo>,

    q_enemy_paths: Query<(), With<EnemyPathPoint>>,
    q_item_paths: Query<(), With<ItemPathPoint>>,
    q_cp_left: Query<(), With<CheckpointLeft>>,
    q_cp_right: Query<(), With<CheckpointRight>>,
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
    let ndc_mouse_pos = ui_viewport_to_ndc(mouse_pos, viewport_info.viewport_rect);
    let ray = RaycastFromCam::new(cam, ndc_mouse_pos, &mut raycast)
        .filter(&|e| q_transform.contains(e))
        .cast();

    let Some((alt_clicked_pt, _)) = ray.first() else {
        return;
    };
    let alt_clicked_pt = *alt_clicked_pt;

    if q_enemy_paths.contains(alt_clicked_pt) {
        for selected in q_selected.iter().filter(|e| q_enemy_paths.contains(*e)) {
            commands.add(move |world: &mut World| {
                KmpPathNode::link_nodes(selected, alt_clicked_pt, world);
            });
        }
    }
    if q_item_paths.contains(alt_clicked_pt) {
        for selected in q_selected.iter().filter(|e| q_item_paths.contains(*e)) {
            commands.add(move |world: &mut World| {
                KmpPathNode::link_nodes(selected, alt_clicked_pt, world);
            });
        }
    }

    if q_cp_left.contains(alt_clicked_pt) || q_cp_right.contains(alt_clicked_pt) {
        for selected in q_selected
            .iter()
            .filter(|e| q_cp_left.contains(*e) || q_cp_right.contains(*e))
        {
            commands.add(move |world: &mut World| {
                let (prev_left, prev_right) = get_both_cp_nodes(world, selected);
                let (next_left, next_right) = get_both_cp_nodes(world, alt_clicked_pt);

                KmpPathNode::link_nodes(prev_left, next_left, world);
                KmpPathNode::link_nodes(prev_right, next_right, world);
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
    // unlink points with the U key
    if !keys.just_pressed(KeyCode::KeyU) {
        return;
    }

    let unlink_command = |world: &mut World, prev: Entity, next: Entity| {
        // if it is a checkpoint
        if world.entity(prev).contains::<CheckpointLeft>() || world.entity(next).contains::<CheckpointRight>() {
            let (prev_left, prev_right) = get_both_cp_nodes(world, prev);
            let (next_left, next_right) = get_both_cp_nodes(world, next);
            KmpPathNode::unlink_nodes(prev_left, next_left, world);
            KmpPathNode::unlink_nodes(prev_right, next_right, world);
        } else {
            KmpPathNode::unlink_nodes(prev, next, world);
        }
    };

    for selected in q_selected.iter() {
        let Ok(node) = q_kmp_path_node.get(selected) else {
            continue;
        };
        for prev_node_entity in node.prev_nodes.iter().copied() {
            commands.add(move |world: &mut World| {
                unlink_command(world, prev_node_entity, selected);
            });
        }
        for next_node_entity in node.next_nodes.iter().copied() {
            commands.add(move |world: &mut World| {
                unlink_command(world, selected, next_node_entity);
            });
        }
    }
}
