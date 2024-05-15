use std::collections::HashSet;

use super::select::{SelectSet, Selected};
use crate::{
    ui::viewport::ViewportInfo,
    util::{ui_viewport_to_ndc, RaycastFromCam},
    viewer::{
        camera::Gizmo2dCam,
        kcl_model::KCLModelSection,
        kmp::{
            components::{
                AreaPoint, BattleFinishPoint, CannonPoint, CheckpointLeft, CheckpointRight, EnemyPathPoint,
                ItemPathPoint, KmpCamera, KmpSelectablePoint, Object, RespawnPoint, SpawnNewPath, SpawnNewPoint,
                SpawnNewWithId, StartPoint,
            },
            path::{KmpPathNode, RecalculatePaths},
            sections::{KmpEditMode, KmpSections},
        },
    },
};
use bevy::prelude::*;
use bevy_mod_raycast::prelude::*;

pub struct CreateDeletePlugin;
impl Plugin for CreateDeletePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<JustCreatedPoint>()
            .add_systems(Update, create_point.before(SelectSet))
            .add_systems(Update, delete_point.after(SelectSet));
    }
}

#[derive(Event)]
pub struct JustCreatedPoint(pub Entity);

fn create_point(
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    q_camera: Query<(&Camera, &GlobalTransform), Without<Gizmo2dCam>>,
    q_window: Query<&Window>,
    viewport_info: Res<ViewportInfo>,
    mut raycast: Raycast,
    q_kcl: Query<(), With<KCLModelSection>>,
    kmp_edit_mode: Res<KmpEditMode>,
    mut commands: Commands,
    q_selected_item_points: Query<Entity, (With<ItemPathPoint>, With<Selected>)>,
    q_selected_enemy_points: Query<Entity, (With<EnemyPathPoint>, With<Selected>)>,
    mut ev_recalc_paths: EventWriter<RecalculatePaths>,
    mut ev_just_created_point: EventWriter<JustCreatedPoint>,

    q_kmp_pts: (
        Query<(), With<KmpSelectablePoint>>,
        Query<(), With<CannonPoint>>,
        Query<(), With<RespawnPoint>>,
        Query<(), With<BattleFinishPoint>>,
    ),
) {
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
    let mouse_ray = RaycastFromCam::new(cam, ndc_mouse_pos, &mut raycast).cast();

    if mouse_ray.iter().any(|e| q_kmp_pts.0.contains(e.0)) {
        return;
    };

    let Some(kcl_intersection) = mouse_ray.iter().find(|e| q_kcl.contains(e.0)) else {
        return;
    };

    let mouse_3d_pos = kcl_intersection.1.position();

    use KmpSections::*;
    let created_entity = match kmp_edit_mode.0 {
        StartPoints => StartPoint::spawn(&mut commands, mouse_3d_pos),
        EnemyPaths => {
            let prev_nodes: HashSet<_> = q_selected_enemy_points.iter().collect();
            EnemyPathPoint::spawn(&mut commands, mouse_3d_pos, prev_nodes)
        }
        ItemPaths => {
            let prev_nodes: HashSet<_> = q_selected_item_points.iter().collect();
            ItemPathPoint::spawn(&mut commands, mouse_3d_pos, prev_nodes)
        }
        RespawnPoints => RespawnPoint::spawn(&mut commands, mouse_3d_pos, q_kmp_pts.1.iter().count()),
        Objects => Object::spawn(&mut commands, mouse_3d_pos),
        Areas => AreaPoint::spawn(&mut commands, mouse_3d_pos),
        Cameras => KmpCamera::spawn(&mut commands, mouse_3d_pos),
        CannonPoints => CannonPoint::spawn(&mut commands, mouse_3d_pos, q_kmp_pts.2.iter().count()),
        BattleFinishPoints => BattleFinishPoint::spawn(&mut commands, mouse_3d_pos, q_kmp_pts.3.iter().count()),
        _ => Entity::PLACEHOLDER,
    };
    commands.entity(created_entity).insert(Selected);
    // we send this event which is recieved by the Select system, so it knows to add the Selected component
    // we can't add it now, because then in the select system it will just be deselected again
    // the select system has to run after this so that we know which previous points we have to link to this one
    // if it ran after, everything would already be deselected by the time we create the point
    ev_just_created_point.send(JustCreatedPoint(created_entity));
    ev_recalc_paths.send_default();
}

fn delete_point(
    keys: Res<ButtonInput<KeyCode>>,
    mut q_selected: Query<(Entity, Option<&CheckpointLeft>, Option<&CheckpointRight>), With<Selected>>,
    mut q_kmp_path_node: Query<&mut KmpPathNode>,
    mut commands: Commands,
    viewport_info: Res<ViewportInfo>,
) {
    if !viewport_info.mouse_in_viewport
        || (!keys.just_pressed(KeyCode::Backspace) && !keys.just_pressed(KeyCode::Delete))
    {
        return;
    }
    // keep track of which entities we have despawned so we don't despawn any twice
    let mut despawned_entities = HashSet::new();
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
    }
}
