use super::select::{cast_ray_from_cam, scale_viewport_pos, Selected};
use crate::{
    ui::ui_state::ViewportRect,
    viewer::{
        kcl_model::KCLModelSection,
        kmp::{
            components::StartPoint,
            meshes_materials::KmpMeshesMaterials,
            path::KmpPathNode,
            point::spawn_point,
            sections::{KmpEditMode, KmpModelSections},
            settings::OutlineSettings,
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

    if kmp_edit_mode.0 != KmpModelSections::StartPoints {
        return;
    }

    spawn_point(
        &mut commands,
        &kmp_meshes_materials.meshes,
        &kmp_meshes_materials.materials.start_points,
        mouse_3d_pos,
        Quat::default(),
        StartPoint::default(),
        &OutlineSettings::default(),
        true,
    );
}

pub fn delete_point(
    keys: Res<Input<KeyCode>>,
    mut q_selected: Query<(Entity, Option<&mut KmpPathNode>), With<Selected>>,
    mut q_kmp_node: Query<&mut KmpPathNode, Without<Selected>>,
    mut commands: Commands,
) {
    if !keys.just_pressed(KeyCode::Back) && !keys.just_pressed(KeyCode::Delete) {
        return;
    }
    for (entity, kmp_path_node) in q_selected.iter_mut() {
        if let Some(mut kmp_path_node) = kmp_path_node {
            kmp_path_node.delete(&mut q_kmp_node);
        }
        commands.entity(entity).despawn_recursive();
    }
}
