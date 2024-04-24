#![allow(dead_code)]

pub mod kcl_file;
pub mod kmp_file;
pub mod read_write_arrays;
pub mod shapes;

use bevy::{math::vec2, prelude::*};
use bevy_egui::egui::{self, Pos2};
use bevy_mod_raycast::{
    immediate::{Raycast, RaycastSettings},
    primitives::IntersectionData,
};

// World <-> Ui Viewport
pub fn world_to_ui_viewport(cam: (&Camera, &GlobalTransform), viewport_rect: Rect, world_pos: Vec3) -> Option<Vec2> {
    let ndc = cam.0.world_to_ndc(cam.1, world_pos)?;
    if ndc.z < 0. || ndc.z > 1. {
        return None;
    };
    Some(ndc_to_ui_viewport(ndc.xy(), viewport_rect))
}
pub fn ui_viewport_to_world(cam: (&Camera, &GlobalTransform), viewport_rect: Rect, viewport_pos: Vec2) -> Option<Vec3> {
    let ndc = ui_viewport_to_ndc(viewport_pos, viewport_rect);
    cam.0.ndc_to_world(cam.1, ndc.extend(0.))
}

pub fn ndc_to_ui_viewport(ndc_pos: Vec2, viewport_rect: Rect) -> Vec2 {
    let x = (ndc_pos.x + 1.) * 0.5 * viewport_rect.width() + viewport_rect.min.x;
    let y = (1. - ndc_pos.y) * 0.5 * viewport_rect.height() + viewport_rect.min.y;
    vec2(x, y)
}
pub fn ui_viewport_to_ndc(viewport_pos: Vec2, viewport_rect: Rect) -> Vec2 {
    let x = ((viewport_pos.x - viewport_rect.min.x) / viewport_rect.width()) * 2. - 1.;
    let y = 1. - ((viewport_pos.y - viewport_rect.min.y) / viewport_rect.height()) * 2.;
    vec2(x, y)
}

/// Convert a point from the UI viewport rect space to the overall screenspace
pub fn ui_viewport_to_screen(viewport_pos: Vec2, window: &Window, viewport_rect: Rect) -> Vec2 {
    (viewport_pos - viewport_rect.min).clamp(Vec2::ZERO, viewport_rect.max) * window.scale_factor()
}
/// Convert a point from the overall screenspace to the UI viewport rect space
pub fn screen_to_ui_viewport(screen_pos: Vec2, window: &Window, viewport_rect: Rect) -> Vec2 {
    (screen_pos / window.scale_factor()) + viewport_rect.min
}

// pub trait ToBevyQuat {
//     fn to_bevy_quat(self) -> Quat;
// }
// impl ToBevyQuat for Quaternion<f64> {
//     fn to_bevy_quat(self) -> Quat {
//         Quat::from_array([self.v.x as f32, self.v.y as f32, self.v.z as f32, self.s as f32])
//     }
// }

// pub trait ToBevyVec3 {
//     fn to_bevy_vec3(self) -> Vec3;
// }
// impl ToBevyVec3 for Vector3<f64> {
//     fn to_bevy_vec3(self) -> Vec3 {
//         vec3(self.x as f32, self.y as f32, self.z as f32)
//     }
// }

pub trait ToBevyVec2 {
    fn to_bevy_vec2(self) -> Vec2;
}
impl ToBevyVec2 for Pos2 {
    fn to_bevy_vec2(self) -> Vec2 {
        vec2(self.x, self.y)
    }
}
impl ToBevyVec2 for egui::Vec2 {
    fn to_bevy_vec2(self) -> Vec2 {
        vec2(self.x, self.y)
    }
}

pub trait ToEguiVec2 {
    fn to_egui_vec2(self) -> egui::Vec2;
}
impl ToEguiVec2 for Pos2 {
    fn to_egui_vec2(self) -> egui::Vec2 {
        egui::vec2(self.x, self.y)
    }
}
impl ToEguiVec2 for Vec2 {
    fn to_egui_vec2(self) -> egui::Vec2 {
        egui::vec2(self.x, self.y)
    }
}

pub trait ToEguiPos2 {
    fn to_egui_pos2(self) -> Pos2;
}
impl ToEguiPos2 for Vec2 {
    fn to_egui_pos2(self) -> Pos2 {
        Pos2 { x: self.x, y: self.y }
    }
}
impl ToEguiPos2 for egui::Vec2 {
    fn to_egui_pos2(self) -> Pos2 {
        Pos2 { x: self.x, y: self.y }
    }
}

pub trait ToBevyRect {
    fn to_bevy_rect(self) -> Rect;
}
impl ToBevyRect for egui::Rect {
    fn to_bevy_rect(self) -> Rect {
        Rect::from_corners(self.min.to_bevy_vec2(), self.max.to_bevy_vec2())
    }
}

pub trait ToEguiRect {
    fn to_egui_rect(self) -> egui::Rect;
}
impl ToEguiRect for Rect {
    fn to_egui_rect(self) -> egui::Rect {
        egui::Rect::from_min_max(self.min.to_egui_pos2(), self.max.to_egui_pos2())
    }
}
pub trait ToBevyTransform {
    fn to_bevy_transform(self) -> bevy::prelude::Transform;
}
// impl ToBevyTransform for transform-gizmo-egui::math::Transform {
//     fn to_bevy_transform(self) -> bevy::prelude::Transform {
//         bevy::prelude::Transform {
//             translation: self.translation.to_bevy_vec3(),
//             rotation: self.rotation.to_bevy_quat(),
//             scale: self.scale.to_bevy_vec3(),
//         }
//     }
// }
// pub trait ToGizmoVec3 {
//     fn to_gizmo_vec3(self) -> Vector3<f64>;
// }
// impl ToGizmoVec3 for Vec3 {
//     fn to_gizmo_vec3(self) -> Vector3<f64> {
//         Vector3 {
//             x: self.x as f64,
//             y: self.y as f64,
//             z: self.z as f64,
//         }
//     }
// }
// pub trait ToGizmoVec4 {
//     fn to_gizmo_vec4(self) -> Vector4<f64>;
// }
// impl ToGizmoVec4 for Vec4 {
//     fn to_gizmo_vec4(self) -> Vector4<f64> {
//         Vector4 {
//             x: self.x as f64,
//             y: self.y as f64,
//             z: self.z as f64,
//             w: self.w as f64,
//         }
//     }
// }
// pub trait ToGizmoQuat {
//     fn to_gizmo_quat(self) -> transform-gizmo-egui::mint::Quaternion<f64>;
// }
// impl ToGizmoQuat for Quat {
//     fn to_gizmo_quat(self) -> transform-gizmo-egui::mint::Quaternion<f64> {
//         transform-gizmo-egui::mint::Quaternion {
//             v: self.xyz().to_gizmo_vec3(),
//             s: self.w as f64,
//         }
//     }
// }
// pub trait ToGizmoTransform {
//     fn to_gizmo_transform(self) -> transform-gizmo-egui::math::Transform;
// }
// impl ToGizmoTransform for Transform {
//     fn to_gizmo_transform(self) -> transform-gizmo-egui::math::Transform {
//         transform-gizmo-egui::math::Transform {
//             translation: self.translation.to_gizmo_vec3(),
//             rotation: self.rotation.to_gizmo_quat(),
//             scale: self.scale.to_gizmo_vec3(),
//         }
//     }
// }

// Ray related stuff
pub fn get_ray_from_cam(cam: (&Camera, &GlobalTransform), ndc: Vec2) -> Option<Ray3d> {
    let world_near_plane = cam.0.ndc_to_world(cam.1, ndc.extend(1.))?;
    let world_far_plane = cam.0.ndc_to_world(cam.1, ndc.extend(f32::EPSILON))?;

    (!world_near_plane.is_nan() && !world_far_plane.is_nan()).then_some(Ray3d::new(
        world_near_plane,
        (world_far_plane - world_near_plane).normalize(),
    ))
}

pub struct RaycastFromCam<'a, 'w, 's> {
    cam: (&'a Camera, &'a GlobalTransform),
    ndc: Vec2,
    raycast: &'a mut Raycast<'w, 's>,
    settings: RaycastSettings<'a>,
}
impl<'a, 'w, 's> RaycastFromCam<'a, 'w, 's> {
    pub fn new(cam: (&'a Camera, &'a GlobalTransform), ndc: Vec2, raycast: &'a mut Raycast<'w, 's>) -> Self {
        Self {
            cam,
            ndc,
            raycast,
            settings: RaycastSettings::default(),
        }
    }
    pub fn filter(mut self, filter: &'a impl Fn(Entity) -> bool) -> Self {
        self.settings.filter = filter;
        self
    }
    pub fn cast(self) -> Vec<(Entity, IntersectionData)> {
        let Some(ray) = get_ray_from_cam(self.cam, self.ndc) else {
            return Vec::new();
        };
        self.raycast.cast_ray(ray, &self.settings).to_vec()
    }
}
