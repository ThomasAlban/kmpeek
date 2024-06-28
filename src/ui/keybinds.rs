use crate::viewer::edit::EditMode;

use super::{file_dialog::ShowFileDialog, ui_state::FileDialogRes};
use bevy::prelude::*;

pub fn keybinds_plugin(app: &mut App) {
    app.add_systems(Update, keybinds);
}

fn keybinds(keys: Res<ButtonInput<KeyCode>>, mut file_dialog: ResMut<FileDialogRes>, mut edit_mode: ResMut<EditMode>) {
    if keys.keybind_pressed([Modifier::Ctrl], [KeyCode::KeyZ]) {
        // undo
    }
    if keys.keybind_pressed([Modifier::Ctrl, Modifier::Shift], [KeyCode::KeyZ])
        || keys.keybind_pressed([Modifier::Ctrl], [KeyCode::KeyY])
    {
        // redo
    }

    if keys.keybind_pressed([Modifier::Ctrl], [KeyCode::KeyO]) {
        // open or close file dialog
        if file_dialog.0.is_none() {
            ShowFileDialog::open_kmp_kcl(&mut file_dialog);
        } else {
            ShowFileDialog::close(&mut file_dialog);
        }
    }

    if keys.keybind_pressed([Modifier::Ctrl], [KeyCode::KeyS]) {
        // save
    }

    if keys.keybind_pressed([], [KeyCode::KeyG]) {
        *edit_mode = match *edit_mode {
            EditMode::Tweak => EditMode::SelectBox,
            EditMode::SelectBox => EditMode::Translate,
            EditMode::Translate => EditMode::Rotate,
            EditMode::Rotate => EditMode::Tweak,
        }
    }
}

#[derive(PartialEq)]
pub enum Modifier {
    Ctrl,
    Alt,
    Shift,
}

pub trait ModifiersPressed {
    fn control_pressed(&self) -> bool;
    fn alt_pressed(&self) -> bool;
    fn super_pressed(&self) -> bool;
    fn shift_pressed(&self) -> bool;
    fn control_or_super_pressed(&self) -> bool;
    fn keybind_pressed(
        &self,
        mods: impl IntoIterator<Item = Modifier>,
        pressed: impl IntoIterator<Item = KeyCode>,
    ) -> bool;
}

impl ModifiersPressed for ButtonInput<KeyCode> {
    fn control_pressed(&self) -> bool {
        self.pressed(KeyCode::ControlLeft) || self.pressed(KeyCode::ControlRight)
    }
    fn alt_pressed(&self) -> bool {
        self.pressed(KeyCode::AltLeft) || self.pressed(KeyCode::AltRight)
    }
    fn super_pressed(&self) -> bool {
        self.pressed(KeyCode::SuperLeft) || self.pressed(KeyCode::SuperRight)
    }
    fn shift_pressed(&self) -> bool {
        self.pressed(KeyCode::ShiftLeft) || self.pressed(KeyCode::ShiftRight)
    }
    fn control_or_super_pressed(&self) -> bool {
        self.control_pressed() || self.super_pressed()
    }
    fn keybind_pressed(
        &self,
        mods: impl IntoIterator<Item = Modifier>,
        pressed: impl IntoIterator<Item = KeyCode>,
    ) -> bool {
        let mods: Vec<Modifier> = mods.into_iter().collect();
        if mods.contains(&Modifier::Ctrl) && !self.control_or_super_pressed()
            || mods.contains(&Modifier::Alt) && !self.alt_pressed()
            || mods.contains(&Modifier::Shift) && !self.shift_pressed()
        {
            return false;
        }
        for pressed_key in pressed.into_iter() {
            if !self.just_pressed(pressed_key) {
                return false;
            }
        }
        true
    }
}
