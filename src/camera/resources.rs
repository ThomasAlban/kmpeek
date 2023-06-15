use bevy::{math::vec3, prelude::*};

#[derive(PartialEq, Clone, Copy)]
pub enum CameraMode {
    Fly,
    Orbit,
    TopDown,
}
impl Default for CameraMode {
    fn default() -> Self {
        Self::Fly
    }
}

#[derive(Resource, Default)]
pub struct CameraSettings {
    pub mode: CameraMode,
    pub fly: FlySettings,
    pub orbit: OrbitSettings,
    pub top_down: TopDownSettings,
}

pub struct FlySettings {
    pub start_pos: Vec3,
    pub look_sensitivity: f32,
    pub speed: f32,
    pub speed_boost: f32,
    pub key_bindings: FlyKeyBindings,
}
impl Default for FlySettings {
    fn default() -> Self {
        Self {
            start_pos: vec3(50000., 50000., 0.),
            look_sensitivity: 1.,
            speed: 1.,
            speed_boost: 3.,
            key_bindings: FlyKeyBindings::default(),
        }
    }
}
pub struct FlyKeyBindings {
    pub move_forward: Vec<KeyCode>,
    pub move_backward: Vec<KeyCode>,
    pub move_left: Vec<KeyCode>,
    pub move_right: Vec<KeyCode>,
    pub move_ascend: Vec<KeyCode>,
    pub move_descend: Vec<KeyCode>,
    pub speed_boost: Vec<KeyCode>,
    pub mouse_button: MouseButton,
}
impl Default for FlyKeyBindings {
    fn default() -> Self {
        Self {
            move_forward: vec![KeyCode::W, KeyCode::Up],
            move_backward: vec![KeyCode::S, KeyCode::Down],
            move_left: vec![KeyCode::A, KeyCode::Left],
            move_right: vec![KeyCode::D, KeyCode::Right],
            move_ascend: vec![KeyCode::Space, KeyCode::E, KeyCode::PageUp],
            move_descend: vec![KeyCode::LControl, KeyCode::Q, KeyCode::PageDown],
            speed_boost: vec![KeyCode::LShift, KeyCode::RShift],
            mouse_button: MouseButton::Right,
        }
    }
}

pub struct OrbitSettings {
    pub start_pos: Vec3,
    pub rotate_sensitivity: f32,
    pub pan_sensitivity: f32,
    pub scroll_sensitivity: f32,
    pub key_bindings: OrbitKeyBindings,
}
impl Default for OrbitSettings {
    fn default() -> Self {
        Self {
            start_pos: vec3(50000., 50000., 0.),
            rotate_sensitivity: 1.,
            pan_sensitivity: 1.,
            scroll_sensitivity: 1.,
            key_bindings: OrbitKeyBindings::default(),
        }
    }
}

pub struct OrbitKeyBindings {
    pub mouse_button: MouseButton,
    pub pan: Vec<KeyCode>,
}
impl Default for OrbitKeyBindings {
    fn default() -> Self {
        Self {
            mouse_button: MouseButton::Right,
            pan: vec![KeyCode::LShift, KeyCode::RShift],
        }
    }
}

pub struct TopDownSettings {
    pub start_pos: Vec3,
    pub near: f32,
    pub far: f32,
    pub scale: f32,
    pub move_sensitivity: f32,
    pub scroll_sensitivity: f32,
    pub key_bindings: TopDownKeyBindings,
}
impl Default for TopDownSettings {
    fn default() -> Self {
        Self {
            start_pos: vec3(0., 10000., 0.),
            near: 0.00001,
            far: 100000.,
            scale: 100.,
            move_sensitivity: 1.,
            scroll_sensitivity: 1.,
            key_bindings: TopDownKeyBindings::default(),
        }
    }
}
pub struct TopDownKeyBindings {
    pub mouse_button: MouseButton,
}
impl Default for TopDownKeyBindings {
    fn default() -> Self {
        Self {
            mouse_button: MouseButton::Right,
        }
    }
}
