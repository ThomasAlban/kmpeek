use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::Ui;
use strum_macros::{Display, EnumIter};
use transform_gizmo_egui::{EnumSet, Gizmo, GizmoConfig, GizmoExt, GizmoMode};

use super::{select::Selected, EditMode};
use crate::{
    ui::tabs::UiSubSection,
    util::{ToBevyTransform, ToGizmoTransform},
    viewer::kmp::area::BoxGizmoOptions,
};

pub struct TransformGizmoPlugin;
impl Plugin for TransformGizmoPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GizmoRes>().init_resource::<GizmoOrigin>();
    }
}

#[derive(Resource, Deref, DerefMut, Default)]
pub struct GizmoRes(pub Gizmo);

#[derive(EnumIter, Display, PartialEq, Clone, Resource, Default)]
pub enum GizmoOrigin {
    #[default]
    Mean,
    FirstSelected,
    Individual,
}

#[derive(SystemParam)]
pub struct ShowGizmo<'w, 's> {
    keys: Res<'w, ButtonInput<KeyCode>>,
    q_selected: Query<'w, 's, &'static mut Transform, (With<Selected>, Without<Camera>)>,
    q_camera: Query<'w, 's, (&'static Camera, &'static Transform)>,
    edit_mode: Res<'w, EditMode>,
    box_gizmo_options: Res<'w, BoxGizmoOptions>,
    gizmo: ResMut<'w, GizmoRes>,
    gizmo_origin: Res<'w, GizmoOrigin>,
}
impl UiSubSection for ShowGizmo<'_, '_> {
    fn show(&mut self, ui: &mut Ui) {
        if *self.edit_mode == EditMode::Tweak
            || *self.edit_mode == EditMode::SelectBox
            || self.box_gizmo_options.mouse_interacting
        {
            return;
        }

        ui.set_clip_rect(ui.max_rect());

        // get the active camera
        let (camera, transform) = self.q_camera.iter().find(|cam| cam.0.is_active).unwrap();

        let (projection_matrix, view_matrix) = { (camera.projection_matrix(), transform.compute_matrix().inverse()) };

        // Snapping is enabled with ctrl key.
        let snapping = self.keys.pressed(KeyCode::ControlLeft) || self.keys.pressed(KeyCode::ControlRight);
        let precise_snap =
            snapping && (self.keys.pressed(KeyCode::ShiftLeft) || self.keys.pressed(KeyCode::ShiftRight));

        // Snap angle to use for rotation when snapping is enabled.
        // Smaller snap angle is used when shift key is pressed.
        let mut snap_angle = self.gizmo.0.config().snap_angle;
        if precise_snap {
            snap_angle /= 2.0
        }

        // Snap distance to use for translation when snapping is enabled.
        // Smaller snap distance is used when shift key is pressed.
        let mut snap_distance = self.gizmo.0.config().snap_distance;
        if precise_snap {
            snap_distance /= 2.0;
        }

        let gizmo_transform_point: Transform;
        let mut single_selected = false;

        match self.q_selected.iter().count() {
            0 => {
                return;
            }
            1 => {
                let selected = self.q_selected.single();
                // if we only have a single point selected, then pass the transform of that single point to the gizmo
                gizmo_transform_point = *selected;
                dbg!(gizmo_transform_point);
                single_selected = true;
            }
            // if there are more than 1
            _ => {
                gizmo_transform_point = match *self.gizmo_origin {
                    GizmoOrigin::Mean => {
                        // if we have multiple selected, calculate the average transform (ignoring rotation / scale) and pass that to the gizmo
                        let mut avg_transform = Transform::default();
                        let mut count = 0.;
                        for selected in self.q_selected.iter() {
                            avg_transform.translation += selected.translation;
                            count += 1.;
                        }
                        avg_transform.translation /= count;
                        avg_transform
                    }
                    _ => {
                        let Some(first_selected_transform) = self.q_selected.iter().next() else {
                            return;
                        };
                        Transform {
                            translation: first_selected_transform.translation,
                            ..default()
                        }
                    }
                };
            }
        }

        let mode = match *self.edit_mode {
            EditMode::Translate => GizmoMode::Translate,
            EditMode::Rotate => GizmoMode::Rotate,
            _ => GizmoMode::Translate,
        };

        self.gizmo.update_config(GizmoConfig {
            view_matrix: view_matrix.as_dmat4().transpose().to_cols_array_2d().into(),
            projection_matrix: projection_matrix.as_dmat4().transpose().to_cols_array_2d().into(),
            modes: EnumSet::only(mode),
            snapping,
            snap_angle,
            snap_distance,
            ..default()
        });

        let Some((_, transforms)) = self.gizmo.0.interact(ui, &[gizmo_transform_point.to_gizmo_transform()]) else {
            return;
        };
        let Some(gizmo_transform) = transforms.first().map(|x| x.to_bevy_transform()) else {
            return;
        };

        if single_selected {
            // if we have a single point selected assign the gizmo response directly
            *self.q_selected.single_mut() = gizmo_transform;
        } else {
            // otherwise, calculate the delta, and apply that delta to each selected point
            let translation_delta = gizmo_transform.translation - gizmo_transform_point.translation;

            for mut selected in self.q_selected.iter_mut() {
                selected.translation += translation_delta;
                if *self.gizmo_origin == GizmoOrigin::Individual {
                    selected.rotate(gizmo_transform.rotation);
                } else {
                    selected.rotate_around(gizmo_transform.translation, gizmo_transform.rotation);
                }
                // this prevents a weird issue where the points would slowly squash
                selected.rotation = selected.rotation.normalize();
            }
        }
    }
}
