use std::collections::HashSet;

use super::select::Selected;
use crate::{
    ui::ui_state::{MouseInViewport, ViewportRect},
    util::{cast_ray_from_cam, ui_viewport_to_ndc},
    viewer::{
        kcl_model::KCLModelSection,
        kmp::{
            components::{
                AreaPoint, BattleFinishPoint, CannonPoint, EnemyPathPoint, ItemPathPoint, KmpCamera, Object,
                RespawnPoint, Spawnable, StartPoint,
            },
            meshes_materials::KmpMeshesMaterials,
            path::{KmpPathNode, RecalculatePaths},
            sections::{KmpEditMode, KmpSections},
        },
    },
};
use bevy::prelude::*;
use bevy_mod_raycast::prelude::*;

pub fn create_point(
    keys: Res<Input<KeyCode>>,
    mouse_buttons: Res<Input<MouseButton>>,
    q_camera: Query<(&Camera, &GlobalTransform)>,
    q_window: Query<&Window>,
    viewport_rect: Res<ViewportRect>,
    mut raycast: Raycast,
    q_kcl: Query<With<KCLModelSection>>,
    kmp_edit_mode: Res<KmpEditMode>,
    kmp_meshes_materials: Res<KmpMeshesMaterials>,
    mut commands: Commands,
    q_selected_item_points: Query<Entity, (With<ItemPathPoint>, With<Selected>)>,
    q_selected_enemy_points: Query<Entity, (With<EnemyPathPoint>, With<Selected>)>,
    mut ev_recalc_paths: EventWriter<RecalculatePaths>,

    q_cannon_point: Query<With<CannonPoint>>,
    q_respawn_point: Query<With<CannonPoint>>,
    q_battle_finish_point: Query<With<BattleFinishPoint>>,
) {
    if !keys.pressed(KeyCode::AltLeft) || !mouse_buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let window = q_window.single();
    let Some(mouse_pos) = window.cursor_position() else {
        return;
    };

    // get the active camera
    let cam = q_camera.iter().find(|cam| cam.0.is_active).unwrap();

    let ndc_mouse_pos = ui_viewport_to_ndc(mouse_pos, viewport_rect.0);
    let mouse_ray = cast_ray_from_cam(cam, ndc_mouse_pos, &mut raycast, |_| true);

    let Some(kcl_intersection) = mouse_ray.iter().find(|e| q_kcl.contains(e.0)) else {
        return;
    };

    let mouse_3d_pos = kcl_intersection.1.position();

    use KmpSections::*;
    let created_entity = match kmp_edit_mode.0 {
        StartPoints => StartPoint::spawn(&mut commands, &kmp_meshes_materials, mouse_3d_pos),
        EnemyPaths => {
            let prev_nodes: HashSet<_> = q_selected_enemy_points.iter().collect();
            EnemyPathPoint::spawn(&mut commands, &kmp_meshes_materials, mouse_3d_pos, prev_nodes)
        }
        ItemPaths => {
            let prev_nodes: HashSet<_> = q_selected_item_points.iter().collect();
            ItemPathPoint::spawn(&mut commands, &kmp_meshes_materials, mouse_3d_pos, prev_nodes)
        }
        RespawnPoints => RespawnPoint::spawn(
            &mut commands,
            &kmp_meshes_materials,
            mouse_3d_pos,
            q_respawn_point.iter().count(),
        ),
        Objects => Object::spawn(&mut commands, &kmp_meshes_materials, mouse_3d_pos),
        Areas => AreaPoint::spawn(&mut commands, &kmp_meshes_materials, mouse_3d_pos),
        Cameras => KmpCamera::spawn(&mut commands, &kmp_meshes_materials, mouse_3d_pos),
        CannonPoints => CannonPoint::spawn(
            &mut commands,
            &kmp_meshes_materials,
            mouse_3d_pos,
            q_cannon_point.iter().count(),
        ),
        BattleFinishPoints => BattleFinishPoint::spawn(
            &mut commands,
            &kmp_meshes_materials,
            mouse_3d_pos,
            q_battle_finish_point.iter().count(),
        ),
        _ => Entity::PLACEHOLDER,
    };
    commands.entity(created_entity).insert(Selected);
    ev_recalc_paths.send_default();
}

pub fn delete_point(
    keys: Res<Input<KeyCode>>,
    mut q_selected: Query<Entity, With<Selected>>,
    mut q_kmp_path_node: Query<&mut KmpPathNode>,
    mut commands: Commands,
    mouse_in_viewport: Res<MouseInViewport>,
) {
    if !mouse_in_viewport.0 {
        return;
    };
    if !keys.just_pressed(KeyCode::Back) && !keys.just_pressed(KeyCode::Delete) {
        return;
    }
    for entity in q_selected.iter_mut() {
        // unlink ourselves if we are a kmp path node so we don't have any stale references before we delete
        if let Ok(kmp_path_node) = q_kmp_path_node.get(entity) {
            kmp_path_node.clone().delete(entity, &mut q_kmp_path_node);
        }
        commands.entity(entity).despawn_recursive();
    }
}
