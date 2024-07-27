use super::select::{SelectSet, Selected};
use crate::{
    ui::viewport::ViewportInfo,
    util::{ui_viewport_to_ndc, RaycastFromCam},
    viewer::{
        camera::Gizmo2dCam,
        kmp::{
            checkpoints::{get_both_cp_nodes, CheckpointRight},
            components::{Checkpoint, CheckpointMarker, EnemyPathPoint, ItemPathPoint, KmpSelectablePoint, RoutePoint},
            path::{is_route_pt, KmpPathNode, RecalcPaths},
            routes::GetRouteStart,
        },
    },
};
use bevy::{ecs::world::Command, prelude::*};
use bevy_mod_raycast::prelude::*;

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
    q_camera: Query<(&Camera, &GlobalTransform), Without<Gizmo2dCam>>,
    q_window: Query<&Window>,
    mut raycast: Raycast,
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
    let mouse_pos = q_window.single().cursor_position()?;

    let cam = q_camera.iter().find(|cam| cam.0.is_active).unwrap();
    let ndc_mouse_pos = ui_viewport_to_ndc(mouse_pos, viewport_info.viewport_rect);
    let ray = RaycastFromCam::new(cam, ndc_mouse_pos, &mut raycast)
        .filter(&|e| q_transform.contains(e))
        .cast();

    let (alt_clicked_pt, _) = ray.first()?;

    Some(*alt_clicked_pt)
}

fn link_points<T: Component + LinkKmpPoint>(
    alt_clicked_pt: In<Option<Entity>>,
    q_pts: Query<(), With<T>>,
    q_selected: Query<Entity, With<Selected>>,
    mut commands: Commands,
    mut ev_recalc_paths: EventWriter<RecalcPaths>,
    get_route_start: GetRouteStart,
) {
    let Some(alt_clicked_pt) = *alt_clicked_pt else {
        return;
    };

    if q_pts.contains(alt_clicked_pt) {
        for selected in q_selected.iter().filter(|e| q_pts.contains(*e)) {
            // we need to check we are not linking to our own route start to create a circle
            if is_route_pt::<T>() {
                let route_start_e = get_route_start.get_entity(selected);
                if route_start_e == alt_clicked_pt {
                    continue;
                }
            }
            commands.add(move |world: &mut World| {
                T::link(world, selected, alt_clicked_pt);
            });
        }
        ev_recalc_paths.send(RecalcPaths::all());
    }
}

trait LinkKmpPoint {
    fn link(world: &mut World, prev_e: Entity, next_e: Entity) {
        KmpPathNode::link_nodes(prev_e, next_e, world);
    }
}
impl LinkKmpPoint for EnemyPathPoint {}
impl LinkKmpPoint for ItemPathPoint {}
impl LinkKmpPoint for RoutePoint {}
impl LinkKmpPoint for CheckpointMarker {
    fn link(world: &mut World, prev_e: Entity, next_e: Entity) {
        let (prev_left, prev_right) = get_both_cp_nodes(world, prev_e);
        let (next_left, next_right) = get_both_cp_nodes(world, next_e);

        KmpPathNode::link_nodes(prev_left, next_left, world);
        KmpPathNode::link_nodes(prev_right, next_right, world);
    }
}

pub fn unlink_points(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    q_kmp_path_node: Query<&KmpPathNode>,
    q_selected: Query<Entity, With<Selected>>,
    mut ev_recalc_paths: EventWriter<RecalcPaths>,
) {
    // unlink points with the U key
    if !keys.just_pressed(KeyCode::KeyU) {
        return;
    }

    struct Unlink(Entity, Entity);
    impl Command for Unlink {
        fn apply(self, world: &mut World) {
            if world.entity(self.0).contains::<Checkpoint>() || world.entity(self.1).contains::<CheckpointRight>() {
                let (prev_left, prev_right) = get_both_cp_nodes(world, self.0);
                let (next_left, next_right) = get_both_cp_nodes(world, self.1);
                KmpPathNode::unlink_nodes(prev_left, next_left, world);
                KmpPathNode::unlink_nodes(prev_right, next_right, world);
            } else {
                KmpPathNode::unlink_nodes(self.0, self.1, world);
            }
        }
    }

    for selected in q_selected.iter() {
        let Ok(node) = q_kmp_path_node.get(selected) else {
            continue;
        };
        for prev_node_entity in node.prev_nodes.iter().copied() {
            commands.add(Unlink(prev_node_entity, selected));
        }
        for next_node_entity in node.next_nodes.iter().copied() {
            commands.add(Unlink(selected, next_node_entity));
        }
    }
    ev_recalc_paths.send(RecalcPaths::all());
}
