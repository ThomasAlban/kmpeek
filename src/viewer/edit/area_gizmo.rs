use crate::{
    ui::viewport::ViewportInfo,
    util::{get_ray_from_cam, ui_viewport_to_ndc, world_to_ui_viewport},
    viewer::{
        camera::Gizmo2dCam,
        edit::select::Selected,
        kmp::components::{AreaPoint, AreaShape},
    },
};
use bevy::{
    math::{vec2, vec3, DVec3},
    prelude::*,
    render::view::RenderLayers,
    transform::TransformSystem,
};
use bevy_vector_shapes::{
    painter::{ShapeConfig, ShapePainter},
    shapes::DiscPainter,
    Shape2dPlugin,
};
use std::f32::consts::{PI, TAU};
use transform_gizmo_bevy::GizmoTarget;

pub struct AreaGizmoPlugin;
impl Plugin for AreaGizmoPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Shape2dPlugin {
            base_config: ShapeConfig {
                // render shapes to the 2d gizmo camera
                render_layers: Some(RenderLayers::layer(1)),
                ..ShapeConfig::default_2d()
            },
        })
        .init_resource::<AreaGizmoOptions>()
        .add_systems(Update, draw_area_bounds)
        // drawing handles after TransformPropagate fixes an issue where they would lag behind the camera position for 1 frame
        .add_systems(PostUpdate, draw_area_handles.after(TransformSystem::TransformPropagate));
    }
}

#[derive(Resource, Default)]
pub struct AreaGizmoOptions {
    pub mouse_hovering: bool,
    pub mouse_interacting: bool,
}

#[derive(Clone, Copy)]
pub struct AreaGizmoInteraction {
    area_entity: Entity,
    handle_index: usize,
    /// the difference between the viewport pos of the point and the mouse pos. This is so you
    /// don't have to be perfectly accurate with the mouse when interacting with a handle.
    mouse_offset: Vec2,
}

// draw the boxes/ellipses for each area which is selected or has the 'Always Show Area' option enabled
fn draw_area_bounds(mut gizmos: Gizmos, q_areas: Query<(&mut Transform, &mut AreaPoint, Has<Selected>)>) {
    for (transform, area, is_selected) in q_areas.iter() {
        if !is_selected && !area.show_area {
            continue;
        }
        // draw the box for the area
        let area_transform = get_area_transform(transform, area.scale);
        let gizmo_color = if area.scale.min_element() < 0. {
            Color::RED
        } else {
            Color::WHITE
        };

        match area.shape {
            AreaShape::Box => gizmos.cuboid(area_transform, gizmo_color),
            AreaShape::Cylinder => {
                let segments = 32;
                let ellipse_h_size = vec2(area.scale.x, area.scale.z) / 2.;
                let ellipse_rot = transform.rotation * Quat::from_rotation_x(PI / 2.);
                let top_pos = transform.translation + transform.up() * area.scale.y;
                let bottom_pos = transform.translation;
                // draw the top ellipse
                gizmos
                    .ellipse(top_pos, ellipse_rot, ellipse_h_size, gizmo_color)
                    .segments(segments);
                // draw the bottom ellipse
                gizmos
                    .ellipse(bottom_pos, ellipse_rot, ellipse_h_size, gizmo_color)
                    .segments(segments);
                // draw the lines going between the top and bottom ellipses
                ellipse_inner(ellipse_h_size, segments)
                    .map(|vec2| ellipse_rot * vec2.extend(0.))
                    .map(|vec3| (vec3 + bottom_pos, vec3 + top_pos))
                    .for_each(|(bottom, top)| gizmos.line(bottom, top, gizmo_color));
            }
        }
    }
}

// draw the handles for each selected area
// these are drawn using the 2d gizmo camera which renders above the main camera
fn draw_area_handles(
    mut q_areas: Query<(Entity, &mut Transform, &mut AreaPoint), With<Selected>>,
    q_cam: Query<(&Camera, &GlobalTransform), (Without<Selected>, Without<Gizmo2dCam>)>,
    q_gizmo_cam: Query<(&Camera, &GlobalTransform), With<Gizmo2dCam>>,
    viewport_info: Res<ViewportInfo>,
    q_window: Query<&Window>,
    mut area_gizmo_opts: ResMut<AreaGizmoOptions>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut current_interaction: Local<Option<AreaGizmoInteraction>>,
    mut initial_mouse_pos: Local<Vec2>,
    q_transform_gizmos: Query<&GizmoTarget>,
    mut painter: ShapePainter,
) {
    const HANDLE_RADIUS: f32 = 12.;
    const HANDLE_HOVER_RADIUS: f32 = 15.;
    const HANDLE_HITBOX_RADIUS: f32 = 10.;
    const LENIENCY_BEFORE_DRAG: f32 = 3.;

    const HANDLE_COORDS: [Vec3; 5] = [
        vec3(-0.5, 0., 0.),
        vec3(0.5, 0., 0.),
        vec3(0., 0.5, 0.),
        vec3(0., 0., -0.5),
        vec3(0., 0., 0.5),
    ];

    // get the active camera
    let cam = q_cam.iter().find(|cam| cam.0.is_active).unwrap();

    let window = q_window.single();

    let mut interacted = false;
    area_gizmo_opts.mouse_hovering = false;

    if !mouse_buttons.pressed(MouseButton::Left) {
        *current_interaction = None;
    }

    // go through each area point which is selected
    for (entity, mut transform, mut area) in q_areas.iter_mut() {
        let area_box = get_area_transform(&transform, area.scale);

        let mut handles_pos = [Vec3::default(); 5];
        let mut handles_normal = [Vec3::default(); 5];
        let mut handles_vp_pos = [None; 5];
        let mut handles_ndc_pos = [None; 5];
        let mut radii = [HANDLE_RADIUS; 5];

        // calculate the position, normal and viewport pos of each handle
        for (i, handle_coord) in HANDLE_COORDS.iter().enumerate() {
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

            let ndc_pos = cam.0.world_to_ndc(cam.1, pos);
            let viewport_pos = world_to_ui_viewport(cam, viewport_info.viewport_rect, pos);

            handles_pos[i] = pos;
            handles_normal[i] = normal;
            handles_vp_pos[i] = viewport_pos;
            handles_ndc_pos[i] = ndc_pos;
        }

        if let Some(mouse_pos) = window.cursor_position() {
            for i in 0..5 {
                let Some(vp_pos) = handles_vp_pos[i] else {
                    continue;
                };
                // use the circle equation to work out if the mouse is over the hitbox of the handle
                let mouse_over =
                    (mouse_pos.x - vp_pos.x).powi(2) + (mouse_pos.y - vp_pos.y).powi(2) < HANDLE_HITBOX_RADIUS.powi(2);
                if mouse_over {
                    area_gizmo_opts.mouse_hovering = true;
                }

                // if we are hovering over a handle and not currently interacting with any other handle
                if mouse_over && current_interaction.is_none() {
                    // set the current interaction, and increase the radius of the
                    *current_interaction = Some(AreaGizmoInteraction {
                        area_entity: entity,
                        handle_index: i,
                        mouse_offset: vp_pos - mouse_pos,
                    });
                    radii[i] = HANDLE_HOVER_RADIUS;
                    area_gizmo_opts.mouse_hovering = true;
                    break;
                }
            }
            if mouse_buttons.just_pressed(MouseButton::Left) {
                *initial_mouse_pos = mouse_pos;
            }
        }

        // if the mouse button is pressed and we aren't interacting with any transform gizmos
        if mouse_buttons.pressed(MouseButton::Left) && !q_transform_gizmos.iter().any(|x| x.is_focused()) {
            if let (
                Some(AreaGizmoInteraction {
                    area_entity: e,
                    handle_index: i,
                    mouse_offset,
                }),
                Some(mouse_pos),
            ) = (*current_interaction, window.cursor_position())
            {
                // if the we are interacting with this area gizmo
                if entity == e {
                    let pos = handles_pos[i];
                    let normal = handles_normal[i];

                    // this adds a certain amount of 'wiggle room' in the mouse position before it actually starts
                    // dragging the point
                    let mouse_ndc = if initial_mouse_pos.distance(mouse_pos) > LENIENCY_BEFORE_DRAG {
                        ui_viewport_to_ndc(mouse_pos + mouse_offset, viewport_info.viewport_rect)
                    } else {
                        ui_viewport_to_ndc(*initial_mouse_pos + mouse_offset, viewport_info.viewport_rect)
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
                        radii[i] = HANDLE_HOVER_RADIUS;
                    }
                }
            }
        }

        // actually render the 5 handles
        painter.color = Color::RED;
        let gizmo_cam = q_gizmo_cam.single();
        for i in 0..5 {
            if let Some(ndc_pos) = handles_ndc_pos[i] {
                // convert the position from ndc to 2d camera coords
                let pos = gizmo_cam.0.ndc_to_world(gizmo_cam.1, ndc_pos);
                if let Some(pos) = pos {
                    painter.transform.translation = pos;
                    painter.circle(radii[i]);
                }
            }
        }
    }
    area_gizmo_opts.mouse_interacting = interacted;
}

/// Convert a transform and scale to an area box/cylinder transform.
fn get_area_transform(transform: &Transform, scale: Vec3) -> Transform {
    let mut gizmo_transform = transform.with_scale(scale);
    gizmo_transform.translation += gizmo_transform.up() * gizmo_transform.scale.y / 2.;
    gizmo_transform
}

/// Work out where each corner of an ellipse is with a given number of segments.
fn ellipse_inner(half_size: Vec2, segments: usize) -> impl Iterator<Item = Vec2> {
    (0..segments + 1).map(move |i| {
        let angle = i as f32 * TAU / segments as f32;
        let (x, y) = angle.sin_cos();
        Vec2::new(x, y) * half_size
    })
}

/// Finds points on two rays that are closest to each other.
/// This can be used to determine the shortest distance between those two rays.
/// Taken from `transform-gizmo`.
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
