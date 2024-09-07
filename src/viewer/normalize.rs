use crate::ui::settings::AppSettings;
use bevy::prelude::*;
use derive_new::new;

use super::camera::Gizmo2dCam;

pub fn normalize_plugin(app: &mut App) {
    app.add_systems(Last, update_normalize);
}

/// Marker struct that marks entities with meshes that should be scaled relative to the camera.
#[derive(Component, Debug, new)]
pub struct Normalize {
    /// Length of the object in world space units
    pub size_in_world: f32,
    /// Desired length of the object in pixels
    pub desired_pixel_size: f32,
    pub axes: BVec3,
}

/// Marker struct that marks entities which should inherit their normalization from the parent
#[derive(Component, Debug)]
pub struct NormalizeInheritParent;

// since this update normalize function runs last in the schedule, it doesn't care about parent/child relationships,
// only about whether individual entities are marked with the normalize component. This is useful because we can have children
// of entities which follow the transform of the parent but aren't necesssarily normalized
fn update_normalize(
    mut p: ParamSet<(
        Query<(&GlobalTransform, &Camera), Without<Gizmo2dCam>>,
        Query<(&mut GlobalTransform, &Normalize, Option<&Children>)>,
        Query<(&mut GlobalTransform, &Transform, &ViewVisibility), With<NormalizeInheritParent>>,
    )>,
    settings: Res<AppSettings>,
    q_window: Query<&Window>,
) {
    let Ok(window) = q_window.get_single() else { return };

    let (camera_position, camera) = {
        let q_cam = p.p0();
        let res = q_cam.iter().find(|x| x.1.is_active).unwrap();
        (res.0.to_owned(), res.1.to_owned())
    };

    let view = camera_position.compute_matrix().inverse();

    let mut children_to_deal_with = Vec::new();

    for (mut gt, normalize, children) in p.p1().iter_mut() {
        // should check here if ViewVisibility is hidden (to make it more efficient),
        // but for some reason that doesn't work for checkpoint arrows and I couldn't
        // be bothered to fix it

        let mut transform_cp = gt.compute_transform();

        let distance = view.transform_point3(transform_cp.translation).z;

        let Some(pixel_end) = camera.world_to_viewport(
            &GlobalTransform::default(),
            Vec3::new(normalize.size_in_world * transform_cp.scale.x, 0.0, distance),
        ) else {
            continue;
        };

        let Some(pixel_root) = camera.world_to_viewport(&GlobalTransform::default(), Vec3::new(0.0, 0.0, distance))
        else {
            continue;
        };

        let actual_pixel_size = pixel_root.distance(pixel_end);

        let required_scale = (normalize.desired_pixel_size * settings.kmp_model.point_scale) / actual_pixel_size;

        let scale_before = transform_cp.scale; // save what the scale was before we change it

        transform_cp.scale = transform_cp.scale * required_scale * window.scale_factor() / 2.; // change the scale

        // reset the scale if we didn't want to affect any axes
        if !normalize.axes.x {
            transform_cp.scale.x = scale_before.x;
        }
        if !normalize.axes.y {
            transform_cp.scale.y = scale_before.y;
        }
        if !normalize.axes.z {
            transform_cp.scale.z = scale_before.z;
        }
        transform_cp.rotation = transform_cp.rotation.normalize();

        gt.set_if_neq(transform_cp.into());

        let Some(children) = children else { continue };
        let children: Vec<_> = children.iter().copied().collect();
        children_to_deal_with.push((*gt, children));
    }

    // now we propogate the change in scale to any children of the normalized points with the 'NormalizeInheritParent' component
    // this may cause issues if there are grandchildren in the heirarchy but for now it's fine, lets cross that bridge when we get there
    let mut p2 = p.p2();
    for (gt, children) in children_to_deal_with.iter() {
        for child in children.iter() {
            let Ok((mut child_gt, child_transform, _)) = p2.get_mut(*child) else {
                continue;
            };
            // multiply the global transform of the parent which the local transform of the child
            *child_gt = gt.mul_transform(*child_transform);
        }
    }
}
