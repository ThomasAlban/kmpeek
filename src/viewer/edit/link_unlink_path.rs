use super::select::{SelectSet, Selected};
use crate::{
    ui::viewport::ViewportInfo,
    util::{ui_viewport_to_ndc, RaycastFromCam},
    viewer::{
        camera::EditorCamera,
        kmp::{
            checkpoints::{get_both_cp_nodes, CheckpointRight},
            components::{Checkpoint, CheckpointMarker, EnemyPathPoint, ItemPathPoint, KmpSelectablePoint, RoutePoint},
            path::{KmpPathNode, RecalcPaths},
        },
    },
};
use bevy::prelude::*;

pub fn link_unlink_plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            get_pt_to_link.pipe(link_points::<EnemyPathPoint>),
            get_pt_to_link.pipe(link_points::<ItemPathPoint>),
            get_pt_to_link.pipe(link_points::<CheckpointMarker>),
            get_pt_to_link.pipe(link_points::<RoutePoint>),
            unlink_points,
        )
            .after(SelectSet),
    );
}

pub fn get_pt_to_link(
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    q_selected: Query<Entity, With<Selected>>,
    q_transform: Query<&Transform, With<KmpSelectablePoint>>,
    q_camera: Query<(&Camera, &GlobalTransform), With<EditorCamera>>,
    q_window: Query<&Window>,
    mut raycast: MeshRayCast,
    viewport_info: Res<ViewportInfo>,
) -> Option<Entity> {
    if !mouse_buttons.just_pressed(MouseButton::Left) {
        return None;
    }
    if !keys.pressed(KeyCode::AltLeft) && !keys.pressed(KeyCode::AltRight) {
        return None;
    }
    if q_selected.is_empty() {
        return None;
    }
    let mouse_pos = q_window.single().ok()?.cursor_position()?;

    let cam = q_camera.iter().find(|cam| cam.0.is_active).unwrap();
    let ndc_mouse_pos = ui_viewport_to_ndc(mouse_pos, viewport_info.viewport_rect);
    let ray = RaycastFromCam::new(cam, ndc_mouse_pos, &mut raycast)
        .filter(&|e| q_transform.contains(e))
        .cast();

    let (alt_clicked_pt, _) = ray.first()?;

    Some(*alt_clicked_pt)
}

fn link_points<T: Component + LinkKmpPoint + Default>(
    alt_clicked_pt: In<Option<Entity>>,
    q_pts: Query<(), With<T>>,
    q_selected: Query<Entity, With<Selected>>,
    mut commands: Commands,
) {
    let Some(alt_clicked_pt) = *alt_clicked_pt else {
        return;
    };

    if q_pts.contains(alt_clicked_pt) {
        let links: Vec<_> = q_selected
            .iter()
            .filter(|selected| q_pts.contains(*selected))
            .map(|selected| (selected, alt_clicked_pt))
            .collect();

        if !links.is_empty() {
            commands.queue(move |world: &mut World| {
                let mut added = Vec::new();
                for (previous, next) in links {
                    if T::link(world, previous, next) {
                        added.push((previous, next));
                    } else {
                        // Treat one Option-click as a transaction: if any edge is
                        // invalid, roll back every edge added by this click.
                        for (added_previous, added_next) in added {
                            T::unlink(world, added_previous, added_next);
                        }
                        return;
                    }
                }
                world.write_message(T::recalc_paths());
            });
        }
    }
}

trait LinkKmpPoint {
    fn link(world: &mut World, prev_e: Entity, next_e: Entity) -> bool {
        KmpPathNode::link_nodes(prev_e, next_e, world)
    }

    fn unlink(world: &mut World, prev_e: Entity, next_e: Entity) -> bool {
        KmpPathNode::unlink_nodes(prev_e, next_e, world)
    }

    fn recalc_paths() -> RecalcPaths;
}
impl LinkKmpPoint for EnemyPathPoint {
    fn recalc_paths() -> RecalcPaths {
        RecalcPaths::enemy()
    }
}
impl LinkKmpPoint for ItemPathPoint {
    fn recalc_paths() -> RecalcPaths {
        RecalcPaths::item()
    }
}
impl LinkKmpPoint for RoutePoint {
    fn recalc_paths() -> RecalcPaths {
        RecalcPaths::route()
    }
}
impl LinkKmpPoint for CheckpointMarker {
    fn link(world: &mut World, prev_e: Entity, next_e: Entity) -> bool {
        let (prev_left, prev_right) = get_both_cp_nodes(world, prev_e);
        let (next_left, next_right) = get_both_cp_nodes(world, next_e);

        let left_changed = KmpPathNode::link_nodes(prev_left, next_left, world);
        let right_changed = KmpPathNode::link_nodes(prev_right, next_right, world);
        if left_changed != right_changed {
            if left_changed {
                KmpPathNode::unlink_nodes(prev_left, next_left, world);
            }
            if right_changed {
                KmpPathNode::unlink_nodes(prev_right, next_right, world);
            }
            return false;
        }
        left_changed
    }

    fn unlink(world: &mut World, prev_e: Entity, next_e: Entity) -> bool {
        let (prev_left, prev_right) = get_both_cp_nodes(world, prev_e);
        let (next_left, next_right) = get_both_cp_nodes(world, next_e);
        let left_changed = KmpPathNode::unlink_nodes(prev_left, next_left, world);
        let right_changed = KmpPathNode::unlink_nodes(prev_right, next_right, world);
        left_changed || right_changed
    }

    fn recalc_paths() -> RecalcPaths {
        RecalcPaths::cp()
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

    struct Unlink(Entity, Entity);
    impl Command for Unlink {
        fn apply(self, world: &mut World) {
            if world.get::<KmpPathNode>(self.0).is_none() || world.get::<KmpPathNode>(self.1).is_none() {
                return;
            }

            let changed = if world.get::<Checkpoint>(self.0).is_some()
                || world.get::<CheckpointRight>(self.1).is_some()
            {
                let (prev_left, prev_right) = get_both_cp_nodes(world, self.0);
                let (next_left, next_right) = get_both_cp_nodes(world, self.1);
                let left_changed = KmpPathNode::unlink_nodes(prev_left, next_left, world);
                let right_changed = KmpPathNode::unlink_nodes(prev_right, next_right, world);
                left_changed || right_changed
            } else {
                KmpPathNode::unlink_nodes(self.0, self.1, world)
            };

            if changed {
                world.write_message(RecalcPaths::all());
            }
        }
    }

    for selected in q_selected.iter() {
        let Ok(node) = q_kmp_path_node.get(selected) else {
            continue;
        };
        for prev_node_entity in node.prev_nodes.iter().copied() {
            commands.queue(Unlink(prev_node_entity, selected));
        }
        for next_node_entity in node.next_nodes.iter().copied() {
            commands.queue(Unlink(selected, next_node_entity));
        }
    }
}
