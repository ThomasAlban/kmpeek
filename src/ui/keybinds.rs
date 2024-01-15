use crate::viewer::transform::EditMode;

use super::{file_dialog::ShowFileDialog, ui_state::FileDialogRes};
use bevy::prelude::*;

pub struct KeybindsPlugin;
impl Plugin for KeybindsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, keybinds);
    }
}

fn keybinds(
    keys: Res<Input<KeyCode>>,
    mut file_dialog: ResMut<FileDialogRes>,
    mut edit_mode: ResMut<EditMode>,
) {
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
            if file_dialog.0.is_none() {
                ShowFileDialog::open_kmp_kcl(&mut file_dialog);
            } else {
                ShowFileDialog::close(&mut file_dialog);
            }
        }
        // } else if keys.just_pressed(KeyCode::S) {
        //     // save!();
        // } else if keys.just_pressed(KeyCode::Z) {
        //     // undo!();
        // }
    }
    if keys.just_pressed(KeyCode::G) {
        *edit_mode = match *edit_mode {
            EditMode::Tweak => EditMode::SelectBox,
            EditMode::SelectBox => EditMode::Translate,
            EditMode::Translate => EditMode::Rotate,
            EditMode::Rotate => EditMode::Tweak,
        }
    }
}
