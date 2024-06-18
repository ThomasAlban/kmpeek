use std::collections::HashSet;

use super::select::{SelectSet, Selected};
use crate::{
    ui::viewport::ViewportInfo,
    util::{get_ray_from_cam, ui_viewport_to_ndc, RaycastFromCam},
    viewer::{
        camera::Gizmo2dCam,
        kcl_model::KCLModelSection,
        kmp::{
            checkpoints::{get_selected_cp_lefts, CheckpointHeight},
            components::{
                AreaPoint, BattleFinishPoint, CannonPoint, CheckpointLeft, CheckpointRight, EnemyPathPoint,
                ItemPathPoint, KmpCamera, KmpSelectablePoint, Object, RespawnPoint, SpawnNewPath, SpawnNewPoint,
                SpawnNewWithId, StartPoint,
            },
            path::{KmpPathNode, RecalcPaths},
            sections::{KmpEditMode, KmpSections},
        },
    },
};
use bevy::prelude::*;
use bevy_mod_raycast::prelude::*;

pub struct CreateDeletePlugin;
impl Plugin for CreateDeletePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<CreatePoint>()
            .add_event::<JustCreatedPoint>()
            .add_systems(Update, (alt_click_create_point, create_point).chain().before(SelectSet))
            .add_systems(Update, delete_point.after(SelectSet));
    }
}

#[derive(Event, Default)]
pub struct CreatePoint {
    pub position: Vec3,
}

#[derive(Event)]
pub struct JustCreatedPoint(pub Entity);

// responsible for consuming 'create point' events and creating the relevant point depending on what edit mode we are in
fn create_point(
    mut commands: Commands,
    kmp_edit_mode: Res<KmpEditMode>,
    mut ev_create_point: EventReader<CreatePoint>,
    mut ev_recalc_paths: EventWriter<RecalcPaths>,
    mut ev_just_created_point: EventWriter<JustCreatedPoint>,

    q_selected_item_pt: Query<Entity, (With<ItemPathPoint>, With<Selected>)>,
    q_selected_enemy_pt: Query<Entity, (With<EnemyPathPoint>, With<Selected>)>,
    mut q_cp_left: Query<(&mut CheckpointLeft, Entity, Has<Selected>)>,
    mut q_cp_right: Query<&mut CheckpointRight, With<Selected>>,
    q_cannon_pt: Query<(), With<CannonPoint>>,
    q_respawn_pt: Query<(), With<RespawnPoint>>,
    q_battle_finish_pt: Query<(), With<BattleFinishPoint>>,
) {
    for create_pt in ev_create_point.read() {
        let pos = create_pt.position;
        use KmpSections::*;
        let created_entity = match kmp_edit_mode.0 {
            StartPoints => StartPoint::spawn(&mut commands, pos),
            EnemyPaths => {
                let prev_nodes: HashSet<_> = q_selected_enemy_pt.iter().collect();
                ev_recalc_paths.send(RecalcPaths::enemy());
                EnemyPathPoint::spawn(&mut commands, pos, prev_nodes)
            }
            ItemPaths => {
                let prev_nodes: HashSet<_> = q_selected_item_pt.iter().collect();
                ev_recalc_paths.send(RecalcPaths::item());
                ItemPathPoint::spawn(&mut commands, pos, prev_nodes)
            }
            Checkpoints => {
                let prev_nodes = get_selected_cp_lefts(&mut q_cp_left, &mut q_cp_right).map(|x| x.0);
                let (_, right) = CheckpointLeft::spawn(&mut commands, pos, prev_nodes.collect());
                ev_recalc_paths.send(RecalcPaths::cp());
                // return the right entity, so that that's the one that is selected and interacted with
                right
            }
            RespawnPoints => RespawnPoint::spawn(&mut commands, pos, q_respawn_pt.iter().count()),
            Objects => Object::spawn(&mut commands, pos),
            Areas => AreaPoint::spawn(&mut commands, pos),
            Cameras => KmpCamera::spawn(&mut commands, pos),
            CannonPoints => CannonPoint::spawn(&mut commands, pos, q_cannon_pt.iter().count()),
            BattleFinishPoints => BattleFinishPoint::spawn(&mut commands, pos, q_battle_finish_pt.iter().count()),
            TrackInfo => return,
        };
        // we send this event which is recieved by the Select system, so it knows to add the Selected component
        // we can't add it now, because then in the select system it will just be deselected again
        // the select system has to run after this so that we know which previous points we have to link to this one
        // if it ran after, everything would already be deselected by the time we create the point
        ev_just_created_point.send(JustCreatedPoint(created_entity));
    }
}

// this detects whether we have alt clicked, and if we have, sends an event to the above function to actually
// create the point in the mouse's 3d position
fn alt_click_create_point(
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    kmp_edit_mode: Res<KmpEditMode>,
    viewport_info: Res<ViewportInfo>,
    mut raycast: Raycast,
    cp_height: Res<CheckpointHeight>,
    q_camera: Query<(&Camera, &GlobalTransform), Without<Gizmo2dCam>>,
    q_window: Query<&Window>,
    q_kmp_pt: Query<(), With<KmpSelectablePoint>>,
    q_kcl: Query<(), With<KCLModelSection>>,
    mut ev_create_pt: EventWriter<CreatePoint>,
) {
    if kmp_edit_mode.0 == KmpSections::TrackInfo {
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

    use KmpSections::*;
    let mouse_3d_pos = match kmp_edit_mode.0 {
        Checkpoints => {
            let Some(ray) = get_ray_from_cam(cam, ndc_mouse_pos) else {
                return;
            };
            let Some(dist) = ray.intersect_plane(Vec3::Y * cp_height.0, Plane3d::default()) else {
                return;
            };
            ray.get_point(dist)
        }
        _ => {
            let Some(kcl_intersection) = intersections.iter().find(|e| q_kcl.contains(e.0)) else {
                return;
            };
            kcl_intersection.1.position()
        }
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
) {
    if !viewport_info.mouse_in_viewport
        || (!keys.just_pressed(KeyCode::Backspace) && !keys.just_pressed(KeyCode::Delete))
    {
        return;
    }
    // keep track of which entities we have despawned so we don't despawn any twice
    let mut despawned_entities = HashSet::new();
    let mut deleted_a_path = false;
    for (entity, cp_left, cp_right) in q_selected.iter_mut() {
        // unlink ourselves if we are a kmp path node so we don't have any stale references before we delete
        if let Ok(kmp_path_node) = q_kmp_path_node.get(entity) {
            kmp_path_node.clone().delete(entity, &mut q_kmp_path_node);
        }
        // if we are a checkpoint, get the other checkpoint entity
        if let Some(other_cp) = cp_left.map(|x| x.right).or_else(|| cp_right.map(|x| x.left)) {
            // unlink that checkpoint first
            if let Ok(kmp_path_node) = q_kmp_path_node.get(other_cp) {
                kmp_path_node.clone().delete(other_cp, &mut q_kmp_path_node);
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
}
