pub mod kcl_file;
pub mod kmp_file;
pub mod read_write_arrays;
pub mod shapes;

use bevy::{math::vec2, prelude::*};
use bevy_egui_next::egui::{self, Pos2};
use bevy_mod_raycast::{
    immediate::{Raycast, RaycastSettings},
    primitives::{IntersectionData, Ray3d},
};

// World <-> Ui Viewport
pub fn world_to_ui_viewport(cam: (&Camera, &GlobalTransform), viewport_rect: Rect, world_pos: Vec3) -> Option<Vec2> {
    let Some(ndc) = cam.0.world_to_ndc(cam.1, world_pos) else {
        return None;
    };
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
    // make (0,0) be the top left corner of the viewport
    let mut screen_pos = viewport_pos - viewport_rect.min;
    screen_pos = screen_pos.clamp(Vec2::ZERO, viewport_rect.max);
    screen_pos *= window.scale_factor() as f32;
    screen_pos
}
/// Convert a point from the overall screenspace to the UI viewport rect space
pub fn screen_to_ui_viewport(screen_pos: Vec2, window: &Window, viewport_rect: Rect) -> Vec2 {
    let mut viewport_pos = screen_pos / window.scale_factor() as f32;
    viewport_pos += viewport_rect.min;
    viewport_pos
}

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

// Ray related stuff
pub fn get_ray_from_cam(cam: (&Camera, &GlobalTransform), ndc: Vec2) -> Option<Ray3d> {
    let Some(world_near_plane) = cam.0.ndc_to_world(cam.1, ndc.extend(1.)) else {
        return None;
    };
    let Some(world_far_plane) = cam.0.ndc_to_world(cam.1, ndc.extend(f32::EPSILON)) else {
        return None;
    };
    let ray = (!world_near_plane.is_nan() && !world_far_plane.is_nan()).then_some(Ray {
        origin: world_near_plane,
        direction: (world_far_plane - world_near_plane).normalize(),
    });
    ray.map(Ray3d::from)
}

pub fn cast_ray_from_cam(
    cam: (&Camera, &GlobalTransform),
    ndc: Vec2,
    raycast: &mut Raycast,
    filter: impl Fn(Entity) -> bool,
) -> Vec<(Entity, IntersectionData)> {
    let Some(ray) = get_ray_from_cam(cam, ndc) else {
        return Vec::new();
    };
    raycast
        .cast_ray(ray, &RaycastSettings::default().with_filter(&filter))
        .to_vec()
}
