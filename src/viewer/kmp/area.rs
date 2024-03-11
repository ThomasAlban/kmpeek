use std::f32::consts::PI;

use super::{AreaPoint, AreaShape};
use crate::{
    ui::{tabs::UiSubSection, ui_state::ViewportRect},
    util::{get_ray_from_cam, ui_viewport_to_ndc, world_to_ui_viewport, ToEguiPos2},
    viewer::edit::{gizmo::GizmoOptions, select::Selected},
};
use bevy::{
    ecs::system::SystemParam,
    math::{vec3, DVec3},
    prelude::*,
};
use bevy_egui::egui::{Color32, Ui};

// convert a transform and scale to an area box transform
pub fn get_area_transform(transform: &Transform, scale: Vec3) -> Transform {
    let mut gizmo_transform = transform.with_scale(scale);
    gizmo_transform.translation += gizmo_transform.up() * gizmo_transform.scale.y / 2.;
    gizmo_transform
}

#[derive(Resource, Default)]
pub struct BoxGizmoOptions {
    pub mouse_interacting: bool,
}

pub fn show_area_boxes(mut gizmos: Gizmos, q_areas: Query<(&mut Transform, &mut AreaPoint, Has<Selected>)>) {
    for (transform, area, is_selected) in q_areas.iter() {
        if !is_selected && !area.show_area {
            continue;
        }
        // draw the box for the area
        let area_transform = get_area_transform(transform, area.scale);
        let cuboid_color = if area.scale.min_element() < 0. {
            Color::RED
        } else {
            Color::WHITE
        };
        match area.shape {
            AreaShape::Box => gizmos.cuboid(area_transform, cuboid_color),
            AreaShape::Cylinder => {
                // todo: Cylinder
            }
        }
        // gizmos.cuboid(area_transform, cuboid_color);
    }
}

// the area box handles are drawn in the UI layer because they need to be drawn on top of everything else
#[derive(SystemParam)]
pub struct ShowBoxHandles<'w, 's> {
    q_areas: Query<'w, 's, (Entity, &'static mut Transform, &'static mut AreaPoint), With<Selected>>,
    q_camera: Query<'w, 's, (&'static Camera, &'static GlobalTransform), Without<Selected>>,
    viewport_rect: Res<'w, ViewportRect>,
    q_window: Query<'w, 's, &'static Window>,
    box_gizmo_options: ResMut<'w, BoxGizmoOptions>,
    mouse_buttons: Res<'w, ButtonInput<MouseButton>>,
    gizmo_options: Res<'w, GizmoOptions>,

    current_interaction: Local<'s, Option<(Entity, usize, Vec2)>>,
    initial_mouse_pos: Local<'s, Vec2>,
}
impl ShowBoxHandles<'_, '_> {
    const HANDLE_RADIUS: f32 = 7.;
    const HANDLE_HOVER_RADIUS: f32 = 10.;
    const HANDLE_HITBOX_RADIUS: f32 = 10.;
    const LENIENCY_BEFORE_DRAG: f32 = 3.;

    const HANDLE_COORDS: [Vec3; 5] = [
        vec3(-0.5, 0., 0.),
        vec3(0.5, 0., 0.),
        vec3(0., 0.5, 0.),
        vec3(0., 0., -0.5),
        vec3(0., 0., 0.5),
    ];
}
impl UiSubSection for ShowBoxHandles<'_, '_> {
    fn show(&mut self, ui: &mut Ui) {
        ui.set_clip_rect(ui.max_rect());

        // get the active camera
        let cam = self.q_camera.iter().find(|cam| cam.0.is_active).unwrap();

        let window = self.q_window.single();

        let mut interacted = false;

        if !self.mouse_buttons.pressed(MouseButton::Left) {
            *self.current_interaction = None;
        }

        // go through each area point which is selected
        for (entity, mut transform, mut area) in self.q_areas.iter_mut() {
            let area_box = get_area_transform(&transform, area.scale);

            let mut handles_pos = [Vec3::default(); 5];
            let mut handles_normal = [Vec3::default(); 5];
            let mut handles_vp_pos = [None; 5];
            let mut radii = [Self::HANDLE_RADIUS; 5];

            // calculate the position, normal and viewport pos of each handle
            for (i, handle_coord) in Self::HANDLE_COORDS.iter().enumerate() {
                let mut transform = Transform::from_translation(*handle_coord * area_box.scale).mul_transform(area_box);
                transform.rotate_around(area_box.translation, transform.rotation);
                let pos = transform.translation;

                let normal = match i {
                    0 => *area_box.left(),
                    1 => *area_box.right(),
                    2 => *area_box.up(),
                    3 => *area_box.forward(),
                    4 => *area_box.back(),
                    _ => unreachable!(),
                };

                let viewport_pos = world_to_ui_viewport(cam, self.viewport_rect.0, pos);

                handles_pos[i] = pos;
                handles_normal[i] = normal;
                handles_vp_pos[i] = viewport_pos;
            }

            if let Some(mouse_pos) = window.cursor_position() {
                for i in 0..5 {
                    let Some(vp_pos) = handles_vp_pos[i] else {
                        continue;
                    };
                    let mouse_over = (mouse_pos.x - vp_pos.x).powi(2) + (mouse_pos.y - vp_pos.y).powi(2)
                        < Self::HANDLE_HITBOX_RADIUS.powi(2);
                    // if we are hovering over a handle and not currently interacting with any other handle
                    if mouse_over && self.current_interaction.is_none() {
                        *self.current_interaction = Some((entity, i, (vp_pos - mouse_pos)));
                        radii[i] = Self::HANDLE_HOVER_RADIUS;
                        break;
                    }
                }
                if self.mouse_buttons.just_pressed(MouseButton::Left) {
                    *self.initial_mouse_pos = mouse_pos;
                }
            }

            if self.mouse_buttons.pressed(MouseButton::Left) && self.gizmo_options.last_result.is_none() {
                if let (Some((e, i, mouse_offset)), Some(mouse_pos)) =
                    (*self.current_interaction, window.cursor_position())
                {
                    if entity == e {
                        let pos = handles_pos[i];
                        let normal = handles_normal[i];

                        // this adds a certain amount of 'wiggle room' in the mouse position before it actually starts
                        // dragging the point
                        let mouse_ndc = if self.initial_mouse_pos.distance(mouse_pos) > Self::LENIENCY_BEFORE_DRAG {
                            ui_viewport_to_ndc(mouse_pos + mouse_offset, self.viewport_rect.0)
                        } else {
                            ui_viewport_to_ndc(*self.initial_mouse_pos + mouse_offset, self.viewport_rect.0)
                        };

                        // send out a ray from the mouse
                        if let Some(mouse_ray) = get_ray_from_cam(cam, mouse_ndc) {
                            // get the ray of the normal to the point we are dragging
                            let normal_ray = Ray3d::new(pos, normal);
                            // find the closest points on both the rays to each otther
                            let (_ray_t, normal_t) = ray_to_ray(mouse_ray, normal_ray);
                            // the new pos is the position along the normal ray that is the closest to the mouse ray
                            let new_pos = normal_ray.get_point(normal_t as f32);

                            let delta = new_pos - pos;

                            let mut new_handles_pos = handles_pos;
                            new_handles_pos[i] = new_pos;

                            // let scale_x = (new_handles_pos[1] - new_handles_pos[]).ang
                            let dist_with_dir = |p1: Vec3, p2: Vec3, dir: Vec3| {
                                let mut angle = (p2 - p1).angle_between(dir);
                                angle %= 2. * PI;

                                let mul = if (0.5 * PI..1.5 * PI).contains(&angle) { -1. } else { 1. };
                                p1.distance(p2) * mul
                            };

                            area.scale = vec3(
                                dist_with_dir(new_handles_pos[0], new_handles_pos[1], handles_normal[1]),
                                dist_with_dir(new_handles_pos[2], transform.translation, *transform.down()),
                                dist_with_dir(new_handles_pos[3], new_handles_pos[4], handles_normal[4]),
                            );

                            // we don't update the transform if we're editing the Y handle
                            if i != 2 {
                                transform.translation += delta / 2.;
                            }

                            interacted = true;
                            radii[i] = Self::HANDLE_HOVER_RADIUS;
                        }
                    }
                }
            }

            for i in 0..5 {
                if let Some(vp_pos) = handles_vp_pos[i] {
                    ui.painter()
                        .circle_filled(vp_pos.to_egui_pos2(), radii[i], Color32::RED);
                }
            }
        }
        self.box_gizmo_options.mouse_interacting = interacted;
    }
}

/// Finds points on two rays that are closest to each other.
/// This can be used to determine the shortest distance between those two rays.
/// Taken from egui-gizmo
pub fn ray_to_ray(a_ray: Ray3d, b_ray: Ray3d) -> (f64, f64) {
    let a1: DVec3 = a_ray.origin.into();
    let adir: DVec3 = a_ray.direction.as_dvec3();
    let b1: DVec3 = b_ray.origin.into();
    let bdir: DVec3 = b_ray.direction.as_dvec3();

    let b = adir.dot(bdir);
    let w = a1 - b1;
    let d = adir.dot(w);
    let e = bdir.dot(w);
    let dot = 1.0 - b * b;
    let ta;
    let tb;

    if dot < 1e-8 {
        ta = 0.0;
        tb = e;
    } else {
        ta = (b * e - d) / dot;
        tb = (e - b * d) / dot;
    }

    (ta, tb)
}
