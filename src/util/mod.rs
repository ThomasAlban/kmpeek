#![allow(dead_code)]

pub mod kcl_file;
pub mod kmp_file;
pub mod read_write_arrays;
pub mod shapes;

use bevy::{
    ecs::{
        component::Tick,
        entity::EntityHashSet,
        query::{QueryData, WorldQuery},
    },
    math::vec2,
    prelude::*,
    window::PrimaryWindow,
};
use bevy_egui::{
    egui::{self, Pos2},
    EguiContext,
};
use bevy_mod_raycast::{
    immediate::{Raycast, RaycastSettings},
    primitives::IntersectionData,
};
use derive_new::new;

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

// pub trait VisibilityToBool {
//     fn to_bool(self) -> bool;
// }
// impl VisibilityToBool for Visibility {
//     fn to_bool(self) -> bool {
//         self == Visibility::Visible
//     }
// }
// pub trait BoolToVisibility {
//     fn to_visibility(self) -> Visibility;
// }
// impl BoolToVisibility for bool {
//     fn to_visibility(self) -> Visibility {
//         if self {
//             Visibility::Visible
//         } else {
//             Visibility::Hidden
//         }
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

// Ray related stuff
pub fn get_ray_from_cam(cam: (&Camera, &GlobalTransform), ndc: Vec2) -> Option<Ray3d> {
    let world_near_plane = cam.0.ndc_to_world(cam.1, ndc.extend(1.))?;
    let world_far_plane = cam.0.ndc_to_world(cam.1, ndc.extend(f32::EPSILON))?;

    (!world_near_plane.is_nan() && !world_far_plane.is_nan()).then_some(Ray3d::new(
        world_near_plane,
        (world_far_plane - world_near_plane).normalize(),
    ))
}

#[derive(new)]
pub struct RaycastFromCam<'a, 'w, 's> {
    cam: (&'a Camera, &'a GlobalTransform),
    ndc: Vec2,
    raycast: &'a mut Raycast<'w, 's>,
    #[new(default)]
    settings: RaycastSettings<'a>,
}
impl<'a, 'w, 's> RaycastFromCam<'a, 'w, 's> {
    pub fn filter(mut self, filter: &'a impl Fn(Entity) -> bool) -> Self {
        self.settings.filter = filter;
        self
    }
    pub fn ray(&self) -> Option<Ray3d> {
        get_ray_from_cam(self.cam, self.ndc)
    }
    pub fn cast(self) -> Vec<(Entity, IntersectionData)> {
        let Some(ray) = self.ray() else {
            return Vec::new();
        };
        self.raycast.cast_ray(ray, &self.settings).to_vec()
    }
}

/// Just give me a mut, damn it! (I really am at the end of my tether)
pub fn give_me_a_mut<'a, T: 'a, R>(items: impl IntoIterator<Item = &'a mut T>, f: impl FnOnce(Vec<Mut<T>>) -> R) -> R {
    let mut items: Vec<_> = items.into_iter().collect();

    let mut ticks = Vec::with_capacity(items.len());
    for _ in 0..items.len() {
        ticks.push((Tick::default(), Tick::default()))
    }
    let mut items_mut = Vec::with_capacity(items.len());
    for (item, ticks) in items.iter_mut().zip(ticks.iter_mut()) {
        let m = Mut::new(*item, &mut ticks.0, &mut ticks.1, Tick::default(), Tick::default());
        items_mut.push(m);
    }
    f(items_mut)
}

pub fn iter_mut_from_entities<'a, R: QueryData>(
    entities: &EntityHashSet,
    q: &'a mut Query<(Entity, R)>,
) -> Vec<<R as WorldQuery>::Item<'a>> {
    let mut items = Vec::new();
    for (e, item) in q.iter_mut() {
        if entities.contains(&e) {
            items.push(item);
        }
    }
    items
}

pub fn egui_has_primary_context(query: Query<(), (With<EguiContext>, With<PrimaryWindow>)>) -> bool {
    !query.is_empty()
}

pub fn try_despawn(commands: &mut Commands, entity: Entity) {
    commands.add(move |world: &mut World| {
        if let Some(e) = world.get_entity_mut(entity) {
            e.despawn_recursive();
        }
    });
}
