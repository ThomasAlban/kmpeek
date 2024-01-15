use bevy::{prelude::*, window::PrimaryWindow};

use crate::ui::{settings::AppSettings, update_ui::UpdateUiSet};

pub struct NormalizePlugin;
impl Plugin for NormalizePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (update_normalize, apply_deferred)
                .in_set(UpdateNormalizeSet)
                .before(UpdateUiSet),
        );
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

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub struct UpdateNormalizeSet;

fn update_normalize(
    mut q_normalize: ParamSet<(
        Query<(&GlobalTransform, &Camera)>,
        Query<(
            &mut Transform,
            &mut GlobalTransform,
            &Normalize,
            &ViewVisibility,
        )>,
    )>,
    settings: Res<AppSettings>,
    q_window: Query<&Window, With<PrimaryWindow>>,
) {
    if !settings.kmp_model.normalize {
        for (mut transform, _, normalize, visibility) in q_normalize.p1().iter_mut() {
            if *visibility == ViewVisibility::HIDDEN {
                continue;
            }
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
    let window = q_window.single();

    let (mut camera_position, mut camera) = (None, None);
    for cam in q_normalize.p0().iter() {
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

    for (mut transform, mut global_transform, normalize, visibility) in q_normalize.p1().iter_mut()
    {
        if *visibility == ViewVisibility::HIDDEN {
            continue;
        }
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

        transform.scale = gt.scale * required_scale * window.scale_factor() as f32 / 2.; // change the scale

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
