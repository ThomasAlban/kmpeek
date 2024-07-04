use super::select::{SelectSet, Selected};
use crate::{
    ui::viewport::ViewportInfo,
    util::{get_ray_from_cam, ui_viewport_to_ndc, RaycastFromCam},
    viewer::{
        camera::Gizmo2dCam,
        kcl_model::KCLModelSection,
        kmp::{
            checkpoints::{CheckpointHeight, GetSelectedCheckpoints},
            components::{
                AreaPoint, BattleFinishPoint, CannonPoint, Checkpoint, CheckpointLeft, CheckpointRight, EnemyPathPoint,
                ItemPathPoint, KmpCamera, KmpSelectablePoint, Object, RespawnPoint, Spawn, Spawner, StartPoint,
                TrackInfo,
            },
            ordering::RefreshOrdering,
            path::{is_checkpoint, KmpPathNode, RecalcPaths},
            sections::{KmpEditMode, ToKmpSection},
        },
    },
};
use bevy::{prelude::*, utils::HashSet};
use bevy_mod_raycast::prelude::*;

pub fn create_delete_plugin(app: &mut App) {
    app.add_event::<CreatePoint>()
        .add_event::<JustCreatedPoint>()
        .add_systems(
            Update,
            (
                alt_click_create_point,
                (
                    create_point::<StartPoint>,
                    create_path::<EnemyPathPoint>,
                    create_path::<ItemPathPoint>,
                    create_path::<Checkpoint>,
                    create_point::<RespawnPoint>,
                    create_point::<Object>,
                    create_point::<AreaPoint>,
                    create_point::<KmpCamera>,
                    create_point::<CannonPoint>,
                    create_point::<BattleFinishPoint>,
                ),
            )
                .chain()
                .before(SelectSet),
        )
        .add_systems(Update, delete_point.after(SelectSet));
}

#[derive(Event, Default)]
pub struct CreatePoint {
    pub position: Vec3,
}

#[derive(Event)]
pub struct JustCreatedPoint(pub Entity);

// responsible for consuming 'create point' events and creating the relevant point depending on what edit mode we are in
fn create_point<T: Component + ToKmpSection + Spawn + Default + Clone>(
    mut commands: Commands,
    mode: Option<Res<KmpEditMode<T>>>,
    mut ev_create_point: EventReader<CreatePoint>,
    mut ev_just_created_point: EventWriter<JustCreatedPoint>,
) {
    if mode.is_none() {
        return;
    }
    let Some(create_pt) = ev_create_point.read().next() else {
        return;
    };
    let pos = create_pt.position;
    let entity = Spawner::<T>::default().pos(pos).spawn_command(&mut commands);
    // we send this event which is recieved by the Select system, so it knows to add the Selected component
    // we can't add it now, because then in the select system it will just be deselected again
    // the select system has to run after this so that we know which previous points we have to link to this one
    // if it ran after, everything would already be deselected by the time we create the point
    ev_just_created_point.send(JustCreatedPoint(entity));
}

fn create_path<T: Component + ToKmpSection + Spawn + Default + Clone>(
    mut commands: Commands,
    mode: Option<Res<KmpEditMode<T>>>,
    q_selected_pt: Query<Entity, (With<T>, With<Selected>)>,
    mut q_cp: GetSelectedCheckpoints,
    mut ev_create_point: EventReader<CreatePoint>,
    mut ev_recalc_paths: EventWriter<RecalcPaths>,
    mut ev_just_created_point: EventWriter<JustCreatedPoint>,
) {
    if mode.is_none() {
        return;
    }
    let Some(create_pt) = ev_create_point.read().next() else {
        return;
    };
    let pos = create_pt.position;
    let prev_nodes: HashSet<_> = if is_checkpoint::<T>() {
        q_cp.get().map(|x| x.0).collect()
    } else {
        q_selected_pt.iter().collect()
    };
    ev_recalc_paths.send(RecalcPaths::all());
    let entity = Spawner::<T>::default()
        .pos(pos)
        .prev_nodes(prev_nodes)
        .spawn_command(&mut commands);
    ev_just_created_point.send(JustCreatedPoint(entity));
}

// this detects whether we have alt clicked, and if we have, sends an event to the above function to actually
// create the point in the mouse's 3d position
fn alt_click_create_point(
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    track_info_mode: Option<Res<KmpEditMode<TrackInfo>>>,
    checkpoint_mode: Option<Res<KmpEditMode<Checkpoint>>>,
    viewport_info: Res<ViewportInfo>,
    mut raycast: Raycast,
    cp_height: Res<CheckpointHeight>,
    q_camera: Query<(&Camera, &GlobalTransform), Without<Gizmo2dCam>>,
    q_window: Query<&Window>,
    q_kmp_pt: Query<(), With<KmpSelectablePoint>>,
    q_kcl: Query<(), With<KCLModelSection>>,
    mut ev_create_pt: EventWriter<CreatePoint>,
) {
    if track_info_mode.is_some() {
        return;
    }
    // only run the function if the alt key is held and the mouse has just been clicked
    if !keys.pressed(KeyCode::AltLeft) || !mouse_buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(mouse_pos) = q_window.single().cursor_position() else {
        return;
    };

    // get the active camera
    let cam = q_camera.iter().find(|cam| cam.0.is_active).unwrap();

    let ndc_mouse_pos = ui_viewport_to_ndc(mouse_pos, viewport_info.viewport_rect);
    let intersections = RaycastFromCam::new(cam, ndc_mouse_pos, &mut raycast).cast();

    // if we are clicking on a kmp point then return and don't create another point
    if intersections.iter().any(|e| q_kmp_pt.contains(e.0)) {
        return;
    };

    let mouse_3d_pos = if checkpoint_mode.is_some() {
        let Some(ray) = get_ray_from_cam(cam, ndc_mouse_pos) else {
            return;
        };
        let Some(dist) = ray.intersect_plane(Vec3::Y * cp_height.0, Plane3d::default()) else {
            return;
        };
        ray.get_point(dist)
    } else {
        let Some(kcl_intersection) = intersections.iter().find(|e| q_kcl.contains(e.0)) else {
            return;
        };
        kcl_intersection.1.position()
    };

    ev_create_pt.send(CreatePoint { position: mouse_3d_pos });
}

fn delete_point(
    keys: Res<ButtonInput<KeyCode>>,
    mut q_selected: Query<(Entity, Option<&CheckpointLeft>, Option<&CheckpointRight>), With<Selected>>,
    mut q_kmp_path_node: Query<&mut KmpPathNode>,
    mut commands: Commands,
    viewport_info: Res<ViewportInfo>,
    mut ev_recalc_paths: EventWriter<RecalcPaths>,
    mut ev_refresh_ordering: EventWriter<RefreshOrdering>,
) {
    if !viewport_info.mouse_in_viewport && !viewport_info.mouse_in_table {
        return;
    }
    if !keys.just_pressed(KeyCode::Backspace) && !keys.just_pressed(KeyCode::Delete) {
        return;
    }
    // keep track of which entities we have despawned so we don't despawn any twice
    let mut despawned_entities = HashSet::new();
    let mut deleted_a_path = false;
    for (entity, cp_left, cp_right) in q_selected.iter_mut() {
        // unlink ourselves if we are a kmp path node so we don't have any stale references before we delete
        if let Ok(kmp_path_node) = q_kmp_path_node.get(entity) {
            kmp_path_node.clone().delete(entity, q_kmp_path_node.as_query_lens());
        }
        // if we are a checkpoint, get the other checkpoint entity
        if let Some(other_cp) = cp_left.map(|x| x.right).or_else(|| cp_right.map(|x| x.left)) {
            // unlink that checkpoint first
            if let Ok(kmp_path_node) = q_kmp_path_node.get(other_cp) {
                kmp_path_node.clone().delete(other_cp, q_kmp_path_node.as_query_lens());
            }
            // then delete it
            if !despawned_entities.contains(&other_cp) {
                commands.entity(other_cp).despawn_recursive();
                despawned_entities.insert(other_cp);
            }
        }
        if !despawned_entities.contains(&entity) {
            commands.entity(entity).despawn_recursive();
            despawned_entities.insert(entity);
        }
        if q_kmp_path_node.contains(entity) {
            deleted_a_path = true;
        };
    }
    if deleted_a_path {
        // do all because we don't know what type of path it is
        ev_recalc_paths.send(RecalcPaths::all());
    }
    ev_refresh_ordering.send_default();
}
