use bevy::prelude::*;

use crate::ui::app_state::AppSettings;

pub struct NormalizePlugin;
impl Plugin for NormalizePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_normalize);
    }
}

/// Marker struct that marks entities with meshes that should be scaled relative to the camera.
#[derive(Component, Debug)]
pub struct Normalize {
    /// Length of the object in world space units
    pub size_in_world: f32,
    /// Desired length of the object in pixels
    pub desired_pixel_size: f32,
    pub axes: BVec3,
}
impl Normalize {
    pub fn new(size_in_world: f32, desired_pixel_size: f32, axes: BVec3) -> Self {
        Self {
            size_in_world,
            desired_pixel_size,
            axes,
        }
    }
}

fn update_normalize(
    mut query: ParamSet<(
        Query<(&GlobalTransform, &Camera)>,
        Query<(&mut Transform, &mut GlobalTransform, &Normalize)>,
    )>,
    settings: Res<AppSettings>,
) {
    if !settings.kmp_model.normalize {
        for (mut transform, _, normalize) in query.p1().iter_mut() {
            let scale_before = transform.scale;
            transform.scale = Vec3::ONE * settings.kmp_model.point_scale;

            if !normalize.axes.x {
                transform.scale.x = scale_before.x;
            }
            if !normalize.axes.y {
                transform.scale.y = scale_before.y;
            }
            if !normalize.axes.z {
                transform.scale.z = scale_before.z;
            }
        }
        return;
    }

    let (mut camera_position, mut camera) = (None, None);
    for cam in query.p0().iter() {
        if cam.1.is_active {
            if camera.is_some() {
                panic!("More than one active camera");
            }
            (camera_position, camera) = (Some(cam.0.to_owned()), Some(cam.1.to_owned()));
        }
    }
    let camera_position = camera_position.expect("Could not find active camera");
    let camera = camera.expect("Could not find active camera");

    let view = camera_position.compute_matrix().inverse();

    for (mut transform, mut global_transform, normalize) in query.p1().iter_mut() {
        let distance = view.transform_point3(global_transform.translation()).z;
        let gt = global_transform.compute_transform();

        let Some(pixel_end) = camera.world_to_viewport(
            &GlobalTransform::default(),
            Vec3::new(normalize.size_in_world * gt.scale.x, 0.0, distance),
        ) else {
            continue;
        };

        let Some(pixel_root) =
            camera.world_to_viewport(&GlobalTransform::default(), Vec3::new(0.0, 0.0, distance))
        else {
            continue;
        };

        let actual_pixel_size = pixel_root.distance(pixel_end);

        let required_scale =
            (normalize.desired_pixel_size * settings.kmp_model.point_scale) / actual_pixel_size;

        let scale_before = transform.scale; // save what the scale was before we change it

        transform.scale = gt.scale * Vec3::splat(required_scale); // change the scale

        // reset the scale if we didn't want to affect any axes
        if !normalize.axes.x {
            transform.scale.x = scale_before.x;
        }
        if !normalize.axes.y {
            transform.scale.y = scale_before.y;
        }
        if !normalize.axes.z {
            transform.scale.z = scale_before.z;
        }

        *global_transform = (*transform).into();
    }
}