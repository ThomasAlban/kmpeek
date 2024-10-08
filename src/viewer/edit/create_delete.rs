use super::select::{SelectSet, Selected};
use crate::{
    ui::viewport::ViewportInfo,
    util::{get_ray_from_cam, try_despawn, ui_viewport_to_ndc, RaycastFromCam},
    viewer::{
        camera::Gizmo2dCam,
        kcl_model::KCLModelSection,
        kmp::{
            checkpoints::{CheckpointHeight, GetSelectedCheckpoints},
            components::{
                AreaPoint, BattleFinishPoint, CannonPoint, Checkpoint, EnemyPathPoint, ItemPathPoint, KmpCamera,
                KmpSelectablePoint, MaxConnectedPath, Object, RespawnPoint, RoutePoint, Spawn, Spawner, StartPoint,
            },
            ordering::RefreshOrdering,
            path::{is_checkpoint, KmpPathNode, RecalcPaths},
            sections::KmpEditMode,
        },
    },
};
use bevy::{ecs::entity::EntityHashSet, prelude::*};
use bevy_mod_raycast::prelude::*;

#[derive(SystemSet, Debug, PartialEq, Eq, Hash, Clone)]
pub struct DeleteSet;

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
                    create_path::<RoutePoint>,
                    create_point::<AreaPoint>,
                    create_point::<KmpCamera>,
                    create_point::<CannonPoint>,
                    create_point::<BattleFinishPoint>,
                ),
            )
                .chain()
                .before(SelectSet),
        )
        .add_systems(Update, delete_point.in_set(DeleteSet).after(SelectSet));
}

#[derive(Event, Default)]
pub struct CreatePoint {
    pub position: Vec3,
}

#[derive(Event)]
pub struct JustCreatedPoint(pub Entity);

// responsible for consuming 'create point' events and creating the relevant point depending on what edit mode we are in
fn create_point<T: Component + Spawn + Default + Clone>(
    mut commands: Commands,
    mode: Res<KmpEditMode>,
    mut ev_create_point: EventReader<CreatePoint>,
    mut ev_just_created_point: EventWriter<JustCreatedPoint>,
) {
    if !mode.in_mode::<T>() {
        return;
    }
    let Some(create_pt) = ev_create_point.read().next() else {
        return;
    };
    let pos = create_pt.position;
    let entity = Spawner::<T>::builder().pos(pos).build().spawn_command(&mut commands);
    // we send this event which is recieved by the Select system, so it knows to add the Selected component
    // we can't add it now, because then in the select system it will just be deselected again
    // the select system has to run after this so that we know which previous points we have to link to this one
    // if it ran after, everything would already be deselected by the time we create the point
    ev_just_created_point.send(JustCreatedPoint(entity));
}

fn create_path<T: Component + Spawn + Default + Clone + MaxConnectedPath>(
    mut commands: Commands,
    mode: Res<KmpEditMode>,
    q_selected_pt: Query<Entity, (With<T>, With<Selected>)>,
    q_kmp_path_node: Query<&KmpPathNode>,
    mut q_cp: GetSelectedCheckpoints,
    mut ev_create_point: EventReader<CreatePoint>,
    mut ev_recalc_paths: EventWriter<RecalcPaths>,
    mut ev_just_created_point: EventWriter<JustCreatedPoint>,
) {
    if !mode.in_mode::<T>() {
        return;
    }
    let Some(create_pt) = ev_create_point.read().next() else {
        return;
    };
    let pos = create_pt.position;
    let prev_nodes: EntityHashSet = if is_checkpoint::<T>() {
        q_cp.get().into_iter().map(|x| x.0).collect()
    } else {
        q_selected_pt.iter().collect()
    };

    // if any prev points are at max linking capacity, then return
    if q_kmp_path_node.iter_many(&prev_nodes).any(|x| x.at_max_next()) {
        return;
    }

    ev_recalc_paths.send_default();
    let entity = Spawner::<T>::builder()
        .pos(pos)
        .prev_nodes(prev_nodes)
        .max(T::MAX_CONNECTED)
        .build()
        .spawn_command(&mut commands);
    // let entity = Spawner::<T>::default()
    //     .pos(pos)
    //     .prev_nodes(prev_nodes)
    //     .max_connected(T::MAX_CONNECTED)
    //     .spawn_command(&mut commands);
    ev_just_created_point.send(JustCreatedPoint(entity));
}

// this detects whether we have alt clicked, and if we have, sends an event to the above function to actually
// create the point in the mouse's 3d position
fn alt_click_create_point(
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mode: Res<KmpEditMode>,
    viewport_info: Res<ViewportInfo>,
    mut raycast: Raycast,
    cp_height: Res<CheckpointHeight>,
    q_camera: Query<(&Camera, &GlobalTransform), Without<Gizmo2dCam>>,
    q_window: Query<&Window>,
    q_kmp_pt: Query<(), With<KmpSelectablePoint>>,
    q_kcl: Query<(), With<KCLModelSection>>,
    mut ev_create_pt: EventWriter<CreatePoint>,
) {
    if *mode == KmpEditMode::TrackInfo {
        return;
    }
    if !viewport_info.mouse_in_viewport {
        return;
    }
    // only run the function if the alt key is held and the mouse has just been clicked
    if !keys.pressed(KeyCode::AltLeft) || !mouse_buttons.just_pressed(MouseButton::Left) {
        return;
    }

    let Some(mouse_pos) = q_window.get_single().ok().and_then(|x| x.cursor_position()) else {
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

    let mouse_3d_pos = if *mode == KmpEditMode::Checkpoints {
        let Some(ray) = get_ray_from_cam(cam, ndc_mouse_pos) else {
            return;
        };
        let Some(dist) = ray.intersect_plane(Vec3::Y * cp_height.0, InfinitePlane3d::default()) else {
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
    mut q_selected: Query<Entity, With<Selected>>,
    mut commands: Commands,
    viewport_info: Res<ViewportInfo>,
    mut ev_refresh_ordering: EventWriter<RefreshOrdering>,
) {
    if !viewport_info.mouse_in_viewport && !viewport_info.mouse_in_table {
        return;
    }
    if !keys.just_pressed(KeyCode::Backspace) && !keys.just_pressed(KeyCode::Delete) {
        return;
    }

    for e in q_selected.iter_mut() {
        try_despawn(&mut commands, e);
    }
    ev_refresh_ordering.send_default();
}
