use bevy::{ecs::event::ManualEventReader, input::mouse::MouseMotion, prelude::*};

// keeps track of mouse motion events, pitch, and yaw
#[derive(Resource, Default)]
pub struct InputState {
    pub reader_motion: ManualEventReader<MouseMotion>,
}

// mouse sensitivity and movement speed
#[derive(Resource)]
pub struct MovementSettings {
    pub sensitivity: f32,
    pub speed: f32,
    pub speed_boost: f32,
}
impl Default for MovementSettings {
    fn default() -> Self {
        Self {
            sensitivity: 0.00012,
            speed: 10000.,
            speed_boost: 3.,
        }
    }
}

// key configuration
#[derive(Resource)]
pub struct KeyBindings {
    pub move_forward: Vec<KeyCode>,
    pub move_backward: Vec<KeyCode>,
    pub move_left: Vec<KeyCode>,
    pub move_right: Vec<KeyCode>,
    pub move_ascend: Vec<KeyCode>,
    pub move_descend: Vec<KeyCode>,
    pub speed_boost: Vec<KeyCode>,
    pub mouse_button: MouseButton,
}

impl Default for KeyBindings {
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
