use super::select::{cast_ray_from_cam, scale_viewport_pos, Selected};
use crate::{
    ui::ui_state::{MouseInViewport, ViewportRect},
    viewer::{
        kcl_model::KCLModelSection,
        kmp::{
            components::{AreaPoint, BattleFinishPoint, CannonPoint, KmpCamera, Object, Spawnable, StartPoint},
            meshes_materials::KmpMeshesMaterials,
            path::KmpPathNode,
            sections::{KmpEditMode, KmpModelSections},
        },
    },
};
use bevy::{prelude::*, window::PrimaryWindow};
use bevy_mod_raycast::prelude::*;

pub fn create_point(
    keys: Res<Input<KeyCode>>,
    mouse_buttons: Res<Input<MouseButton>>,
    q_camera: Query<(&Camera, &GlobalTransform)>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    viewport_rect: Res<ViewportRect>,
    mut raycast: Raycast,
    q_kcl: Query<With<KCLModelSection>>,
    kmp_edit_mode: Res<KmpEditMode>,
    kmp_meshes_materials: Res<KmpMeshesMaterials>,
    mut commands: Commands,
) {
    if !keys.pressed(KeyCode::AltLeft) || !mouse_buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let window = q_window.single();
    let Some(mouse_pos) = window.cursor_position() else {
        return;
    };
    let scaled_mouse_pos = scale_viewport_pos(mouse_pos, window, viewport_rect.0);

    // get the active camera
    let cam = q_camera
        .iter()
        .filter(|cam| cam.0.is_active)
        .collect::<Vec<(&Camera, &GlobalTransform)>>()[0];

    let mouse_ray = cast_ray_from_cam(cam, scaled_mouse_pos, &mut raycast, |_| true);

    let Some(kcl_intersection) = mouse_ray.iter().find(|e| q_kcl.contains(e.0)) else {
        return;
    };

    let mouse_3d_pos = kcl_intersection.1.position();

    use KmpModelSections::*;
    match kmp_edit_mode.0 {
        StartPoints => StartPoint::spawn(&mut commands, &kmp_meshes_materials, mouse_3d_pos),
        // EnemyPaths =>
        Objects => Object::spawn(&mut commands, &kmp_meshes_materials, mouse_3d_pos),
        Areas => AreaPoint::spawn(&mut commands, &kmp_meshes_materials, mouse_3d_pos),
        Cameras => KmpCamera::spawn(&mut commands, &kmp_meshes_materials, mouse_3d_pos),
        CannonPoints => CannonPoint::spawn(&mut commands, &kmp_meshes_materials, mouse_3d_pos),
        BattleFinishPoints => BattleFinishPoint::spawn(&mut commands, &kmp_meshes_materials, mouse_3d_pos),
        _ => Entity::PLACEHOLDER,
    };
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
