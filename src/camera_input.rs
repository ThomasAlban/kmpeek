use bevy::{input::mouse::MouseMotion, prelude::*, window::CursorGrabMode};
use smooth_bevy_cameras::controllers::fps::{ControlEvent, FpsCameraController};

/// this system is run every frame and controls the movement of the camera in the 3d scene
pub fn camera_input(
    mut events: EventWriter<ControlEvent>,
    mut windows: Query<&mut Window>,
    mut mouse_motion_events: EventReader<MouseMotion>,
    keyboard: Res<Input<KeyCode>>,
    mouse_buttons: Res<Input<MouseButton>>,
    controllers: Query<&FpsCameraController>,
) {
    // Can only control one camera at a time.
    let controller = if let Some(controller) = controllers.iter().find(|c| c.enabled) {
        controller
    } else {
        return;
    };

    let mut window = windows.single_mut();

    // if the right mouse is not pressed then unlock the cursor and return
    if !mouse_buttons.pressed(MouseButton::Right) {
        window.cursor.visible = true;
        window.cursor.grab_mode = CursorGrabMode::None;
        return;
    }
    // hide the cursor and lock its position
    window.cursor.visible = false;
    window.cursor.grab_mode = CursorGrabMode::Locked;

    let FpsCameraController {
        translate_sensitivity,
        mouse_rotate_sensitivity,
        ..
    } = *controller;

    let mut cursor_delta = Vec2::ZERO;
    for event in mouse_motion_events.iter() {
        cursor_delta += event.delta;
    }

    events.send(ControlEvent::Rotate(
        mouse_rotate_sensitivity * cursor_delta,
    ));

    // if either shift key is pressed, increase the speed 3x
    let mut shift_speed = 1.;
    if keyboard.pressed(KeyCode::LShift) || keyboard.pressed(KeyCode::RShift) {
        shift_speed = 3.;
    }

    // translate the camera depending on which key is pressed
    for (keys, dir) in [
        (vec![KeyCode::W, KeyCode::Up], Vec3::Z),
        (vec![KeyCode::A, KeyCode::Left], Vec3::X),
        (vec![KeyCode::S, KeyCode::Down], -Vec3::Z),
        (vec![KeyCode::D, KeyCode::Right], -Vec3::X),
        (vec![KeyCode::Q, KeyCode::PageDown], -Vec3::Y),
        (vec![KeyCode::E, KeyCode::Space, KeyCode::PageUp], Vec3::Y),
    ]
    .iter()
    .cloned()
    {
        for key in keys {
            if keyboard.pressed(key) {
                events.send(ControlEvent::TranslateEye(
                    translate_sensitivity * shift_speed * dir,
                ));
                break;
            }
        }
    }
}
