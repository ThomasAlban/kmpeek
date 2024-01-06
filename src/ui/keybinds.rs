use super::{app_state::AppState, file_dialog::ShowFileDialog};
use bevy::prelude::*;

pub struct KeybindsPlugin;
impl Plugin for KeybindsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, keybinds);
    }
}

fn keybinds(keys: Res<Input<KeyCode>>, mut app_state: ResMut<AppState>) {
    // keybinds
    // if the control/command key is pressed
    if (!cfg!(target_os = "macos")
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight)))
        || (cfg!(target_os = "macos")
            && (keys.pressed(KeyCode::SuperLeft) || keys.pressed(KeyCode::SuperRight)))
    {
        if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
            // keybinds with shift held
            if keys.just_pressed(KeyCode::Z) {
                // redo!();
            }
        // keybinds without shift held
        } else if keys.just_pressed(KeyCode::O) {
            if app_state.file_dialog.is_none() {
                ShowFileDialog::open_kmp_kcl(&mut app_state);
            } else {
                ShowFileDialog::close(&mut app_state);
            }
        }
        // } else if keys.just_pressed(KeyCode::S) {
        //     // save!();
        // } else if keys.just_pressed(KeyCode::Z) {
        //     // undo!();
        // }
    }
}
