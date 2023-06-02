use bevy::prelude::*;

#[derive(PartialEq)]
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
    pub topdown: TopDownSettings,
}

pub struct FlySettings {
    pub sensitivity: f32,
    pub speed: f32,
    pub speed_boost: f32,
    pub key_bindings: FlyKeyBindings,
}
impl Default for FlySettings {
    fn default() -> Self {
        Self {
            sensitivity: 0.00012,
            speed: 10000.,
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
#[derive(Default)]
pub struct OrbitSettings {
    pub key_bindings: OrbitKeyBindings,
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

#[derive(Default)]
pub struct TopDownSettings {
    pub key_bindings: TopDownKeyBindings,
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
