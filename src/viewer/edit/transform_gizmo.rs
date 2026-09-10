use super::{select::Selected, EditMode};
use crate::{
    ui::settings::AppSettings,
    viewer::{
        camera::EditorCamera,
        kmp::{
            checkpoints::{CheckpointLeft, CheckpointRight},
            settings::{
                DEFAULT_GIZMO_LINE_WIDTH, DEFAULT_GIZMO_SIZE, MAX_GIZMO_LINE_WIDTH, MAX_GIZMO_SIZE,
                MIN_GIZMO_LINE_WIDTH, MIN_GIZMO_SIZE,
            },
        },
    },
};
use bevy::{
    ecs::{entity::EntityHashMap, system::SystemState},
    prelude::*,
};
use bevy_egui::egui::{self, epaint::Vertex, Mesh, PointerButton, Rgba, Sense, Ui};
use transform_gizmo::{
    enum_set,
    math::{
        DMat4 as GizmoDMat4, DQuat as GizmoDQuat, DVec3 as GizmoDVec3, Pos2 as GizmoPos2, Rect as GizmoRect,
        Transform as GizmoTransform,
    },
    Gizmo, GizmoConfig, GizmoInteraction, GizmoMode, GizmoVisuals,
};

#[derive(Component)]
pub struct GizmoTransformable;

#[derive(Resource)]
pub struct TransformGizmoState {
    gizmo: Gizmo,
    individual_gizmos: EntityHashMap<Gizmo>,
    pub config: GizmoConfig,
    pub group_targets: bool,
    pub is_focused: bool,
    enabled: bool,
}

impl Default for TransformGizmoState {
    fn default() -> Self {
        Self {
            gizmo: Gizmo::default(),
            individual_gizmos: EntityHashMap::default(),
            config: GizmoConfig {
                modes: GizmoMode::all_translate(),
                visuals: GizmoVisuals {
                    gizmo_size: DEFAULT_GIZMO_SIZE,
                    stroke_width: DEFAULT_GIZMO_LINE_WIDTH,
                    ..default()
                },
                ..default()
            },
            group_targets: true,
            is_focused: false,
            enabled: false,
        }
    }
}

pub fn transform_gizmo_plugin(app: &mut App) {
    app.init_resource::<TransformGizmoState>()
        .add_systems(Update, update_gizmo_options);
}

fn update_gizmo_options(
    edit_mode: Res<EditMode>,
    q_selected_cp: Query<(), (With<Selected>, Or<(With<CheckpointLeft>, With<CheckpointRight>)>)>,
    mut state: ResMut<TransformGizmoState>,
    settings: Res<AppSettings>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    let checkpoint_selected = !q_selected_cp.is_empty();
    state.enabled = matches!(*edit_mode, EditMode::Translate | EditMode::Rotate);
    state.config.modes = if *edit_mode == EditMode::Translate && checkpoint_selected {
        enum_set!(GizmoMode::TranslateX | GizmoMode::TranslateZ | GizmoMode::TranslateXZ)
    } else if *edit_mode == EditMode::Rotate && checkpoint_selected {
        enum_set!(GizmoMode::RotateY)
    } else if *edit_mode == EditMode::Translate {
        GizmoMode::all_translate()
    } else if *edit_mode == EditMode::Rotate {
        GizmoMode::all_rotate()
    } else {
        state.config.modes
    };
    state.config.visuals.gizmo_size = if settings.kmp_model.gizmo_size.is_finite() {
        settings.kmp_model.gizmo_size.clamp(MIN_GIZMO_SIZE, MAX_GIZMO_SIZE)
    } else {
        DEFAULT_GIZMO_SIZE
    };
    state.config.visuals.stroke_width = if settings.kmp_model.gizmo_line_width.is_finite() {
        settings
            .kmp_model
            .gizmo_line_width
            .clamp(MIN_GIZMO_LINE_WIDTH, MAX_GIZMO_LINE_WIDTH)
    } else {
        DEFAULT_GIZMO_LINE_WIDTH
    };
    state.config.snapping = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
}

pub fn show_transform_gizmo(ui: &mut Ui, viewport: egui::Rect, world: &mut World) {
    let mut system_state = SystemState::<(
        ResMut<TransformGizmoState>,
        Query<(Entity, &mut Transform), (With<Selected>, With<GizmoTransformable>)>,
        Query<(&Camera, &GlobalTransform), With<EditorCamera>>,
    )>::new(world);
    let (mut state, mut q_targets, q_camera) = system_state.get_mut(world);

    if !state.enabled {
        state.is_focused = false;
        return;
    }

    let Some((camera, camera_transform)) = q_camera.iter().find(|(camera, _)| camera.is_active) else {
        state.is_focused = false;
        return;
    };

    if viewport.width() <= 1.0 || viewport.height() <= 1.0 {
        state.is_focused = false;
        return;
    }

    let targets = q_targets
        .iter_mut()
        .map(|(entity, transform)| (entity, to_gizmo_transform(&transform)))
        .collect::<Vec<_>>();
    if targets.is_empty() {
        state.is_focused = false;
        return;
    }

    let mut config = state.config;
    let view_matrix = camera_transform.to_matrix().inverse().as_dmat4();
    let projection_matrix = camera.clip_from_view().as_dmat4();
    config.view_matrix = GizmoDMat4::from_cols_array(&view_matrix.to_cols_array()).into();
    config.projection_matrix = GizmoDMat4::from_cols_array(&projection_matrix.to_cols_array()).into();
    config.viewport = GizmoRect::from_min_max(
        GizmoPos2::new(viewport.min.x, viewport.min.y),
        GizmoPos2::new(viewport.max.x, viewport.max.y),
    );
    config.pixels_per_point = ui.ctx().pixels_per_point();

    let cursor_pos = ui.input(|input| input.pointer.hover_pos()).unwrap_or_default();
    let interaction_response = ui.interact(
        egui::Rect::from_center_size(cursor_pos, egui::Vec2::splat(1.0)),
        ui.id().with("transform_gizmo_interaction"),
        Sense::click_and_drag(),
    );
    let interaction = GizmoInteraction {
        cursor_pos: (cursor_pos.x, cursor_pos.y),
        hovered: viewport.contains(cursor_pos) && interaction_response.hovered(),
        drag_started: ui.input(|input| input.pointer.button_pressed(PointerButton::Primary)),
        dragging: ui.input(|input| input.pointer.button_down(PointerButton::Primary)),
    };

    if state.group_targets {
        state.gizmo.update_config(config);
        let target_transforms = targets.iter().map(|(_, transform)| *transform).collect::<Vec<_>>();
        let result = state.gizmo.update(interaction, &target_transforms);
        state.is_focused = state.gizmo.is_focused();
        paint_gizmo(ui, viewport, &state.gizmo);

        if let Some((_, updated_transforms)) = result {
            for ((entity, _), updated) in targets.into_iter().zip(updated_transforms) {
                apply_gizmo_transform(&mut q_targets, entity, updated);
            }
        }
    } else {
        let mut focused = false;
        for (entity, target) in &targets {
            let gizmo = state.individual_gizmos.entry(*entity).or_default();
            gizmo.update_config(config);
            let result = gizmo.update(interaction, &[*target]);
            focused |= gizmo.is_focused();
            paint_gizmo(ui, viewport, gizmo);

            if let Some((_, updated_transforms)) = result {
                if let Some(updated) = updated_transforms.into_iter().next() {
                    apply_gizmo_transform(&mut q_targets, *entity, updated);
                }
            }
        }
        state
            .individual_gizmos
            .retain(|entity, _| targets.iter().any(|(target, _)| target == entity));
        state.is_focused = focused;
    }
}

fn paint_gizmo(ui: &Ui, viewport: egui::Rect, gizmo: &Gizmo) {
    let draw_data = gizmo.draw();
    egui::Painter::new(ui.ctx().clone(), ui.layer_id(), viewport).add(Mesh {
        indices: draw_data.indices,
        vertices: draw_data
            .vertices
            .into_iter()
            .zip(draw_data.colors)
            .map(|(position, [r, g, b, a])| Vertex {
                pos: position.into(),
                uv: egui::Pos2::default(),
                color: Rgba::from_rgba_premultiplied(r, g, b, a).into(),
            })
            .collect(),
        ..default()
    });
}

fn to_gizmo_transform(transform: &Transform) -> GizmoTransform {
    GizmoTransform {
        translation: GizmoDVec3::from_array(transform.translation.as_dvec3().to_array()).into(),
        rotation: GizmoDQuat::from_array(transform.rotation.as_dquat().to_array()).into(),
        scale: GizmoDVec3::from_array(transform.scale.as_dvec3().to_array()).into(),
    }
}

fn update_bevy_transform(transform: &mut Transform, updated: GizmoTransform) {
    let translation = GizmoDVec3::from(updated.translation);
    let rotation = GizmoDQuat::from(updated.rotation);
    let scale = GizmoDVec3::from(updated.scale);
    transform.translation = Vec3::from_array(translation.as_vec3().to_array());
    transform.rotation = Quat::from_array(rotation.as_quat().to_array());
    transform.scale = Vec3::from_array(scale.as_vec3().to_array());
}

fn apply_gizmo_transform(
    q_targets: &mut Query<(Entity, &mut Transform), (With<Selected>, With<GizmoTransformable>)>,
    entity: Entity,
    updated: GizmoTransform,
) {
    let Ok((_, mut transform)) = q_targets.get_mut(entity) else {
        return;
    };
    update_bevy_transform(&mut transform, updated);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transform_adapter_preserves_bevy_transform() {
        let original = Transform {
            translation: Vec3::new(12.5, -45.25, 1_000.75),
            rotation: Quat::from_euler(EulerRot::YXZ, 0.7, -1.1, 2.4),
            scale: Vec3::new(2.0, 0.5, 3.25),
        };
        let mut roundtrip = Transform::default();

        update_bevy_transform(&mut roundtrip, to_gizmo_transform(&original));

        assert_eq!(roundtrip.translation, original.translation);
        assert_eq!(roundtrip.rotation, original.rotation);
        assert_eq!(roundtrip.scale, original.scale);
    }

    #[test]
    fn gizmo_defaults_match_editor_behavior() {
        let state = TransformGizmoState::default();

        assert_eq!(state.config.modes, GizmoMode::all_translate());
        assert!(state.group_targets);
        assert!(!state.enabled);
        assert!(!state.is_focused);
    }
}
