use bevy::{ecs::system::SystemParam, prelude::*};
use egui_gizmo::{Gizmo, GizmoMode, GizmoOrientation, GizmoResult, GizmoVisuals};
use strum_macros::{Display, EnumIter};

use super::{select::Selected, EditMode};
use crate::ui::tabs::UiSubSection;

pub struct TransformGizmoPlugin;
impl Plugin for TransformGizmoPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GizmoOptions>();
    }
}

#[derive(EnumIter, Display, PartialEq, Clone)]
pub enum GizmoOrigin {
    Mean,
    FirstSelected,
    Individual,
}

#[derive(Resource)]
pub struct GizmoOptions {
    pub gizmo_origin: GizmoOrigin,
    pub gizmo_orientation: GizmoOrientation,
    pub last_result: Option<GizmoResult>,
    pub snap_angle: f32,
    pub snap_distance: f32,
}
impl Default for GizmoOptions {
    fn default() -> Self {
        Self {
            gizmo_origin: GizmoOrigin::Mean,
            gizmo_orientation: GizmoOrientation::Local,
            last_result: None,
            snap_angle: 22.5,
            snap_distance: 100.,
        }
    }
}

#[derive(SystemParam)]
pub struct ShowGizmo<'w, 's> {
    keys: Res<'w, Input<KeyCode>>,
    q_selected: Query<'w, 's, &'static mut Transform, (With<Selected>, Without<Camera>)>,
    q_camera: Query<'w, 's, (&'static Camera, &'static Transform)>,
    edit_mode: Res<'w, EditMode>,
    gizmo_options: ResMut<'w, GizmoOptions>,
}
impl UiSubSection for ShowGizmo<'_, '_> {
    fn show(&mut self, ui: &mut bevy_egui::egui::Ui) {
        if *self.edit_mode == EditMode::Tweak || *self.edit_mode == EditMode::SelectBox {
            return;
        }
        ui.allocate_ui_at_rect(ui.max_rect(), |ui| {
            ui.set_clip_rect(ui.max_rect());

            // get the active camera
            let (camera, transform) = self
                .q_camera
                .iter()
                .filter(|cam| cam.0.is_active)
                .collect::<Vec<(&Camera, &Transform)>>()[0];
            let (projection_matrix, view_matrix) =
                { (camera.projection_matrix(), transform.compute_matrix().inverse()) };

            // Snapping is enabled with ctrl key.
            let snapping = self.keys.pressed(KeyCode::ControlLeft);
            let precise_snap = snapping && self.keys.pressed(KeyCode::ShiftLeft);

            // Snap angle to use for rotation when snapping is enabled.
            // Smaller snap angle is used when shift key is pressed.
            let mut snap_angle = f32::to_radians(self.gizmo_options.snap_angle);
            if precise_snap {
                snap_angle /= 2.0
            }

            // Snap distance to use for translation when snapping is enabled.
            // Smaller snap distance is used when shift key is pressed.
            let mut snap_distance = self.gizmo_options.snap_distance;
            if precise_snap {
                snap_distance /= 2.0;
            }

            let visuals = GizmoVisuals::default();

            let gizmo_transform_point: Transform;
            let mut single_selected = false;
            if let Ok(selected) = self.q_selected.get_single() {
                // if we only have a single point selected, then pass the transform of that single point to the gizmo
                gizmo_transform_point = *selected;
                single_selected = true;
            } else {
                gizmo_transform_point = match self.gizmo_options.gizmo_origin {
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
            let model_matrix = gizmo_transform_point.compute_matrix();

            let mode = match *self.edit_mode {
                EditMode::Translate => GizmoMode::Translate,
                EditMode::Rotate => GizmoMode::Rotate,
                _ => GizmoMode::Translate,
            };

            let gizmo = Gizmo::new("My gizmo")
                .view_matrix(view_matrix.to_cols_array_2d().into())
                .projection_matrix(projection_matrix.to_cols_array_2d().into())
                .model_matrix(model_matrix.to_cols_array_2d().into())
                .mode(mode)
                .orientation(self.gizmo_options.gizmo_orientation)
                .snapping(snapping)
                .snap_angle(snap_angle)
                .snap_distance(snap_distance)
                .visuals(visuals);

            self.gizmo_options.last_result = gizmo.interact(ui);

            if let Some(gizmo_response) = self.gizmo_options.last_result {
                let gizmo_transform: [[f32; 4]; 4] = gizmo_response.transform().into();
                let gizmo_transform = Transform::from_matrix(Mat4::from_cols_array_2d(&gizmo_transform));

                if single_selected {
                    // if we have a single point selected assign the gizmo response directly
                    *self.q_selected.single_mut() = gizmo_transform;
                } else {
                    // otherwise, calculate the delta, and apply that delta to each selected point
                    let translation_delta = gizmo_transform.translation - gizmo_transform_point.translation;

                    for mut selected in self.q_selected.iter_mut() {
                        selected.translation += translation_delta;
                        if self.gizmo_options.gizmo_origin == GizmoOrigin::Individual {
                            selected.rotate(gizmo_transform.rotation);
                        } else {
                            selected.rotate_around(gizmo_transform.translation, gizmo_transform.rotation);
                            // someone went onto my laptop and wrote this when I was gone
                            // so I guess I'm leaving it here
                            // cum on my ballsac and call me a meercat
                        }
                        // this prevents a weird issue where the points would slowly squash
                        selected.rotation = selected.rotation.normalize();
                    }
                }
            }
        });
    }
}
