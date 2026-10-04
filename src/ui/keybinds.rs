use crate::viewer::{edit::EditorMode, kmp::SaveFile};

use super::{
    file_dialog::FileDialogManager, settings::AppSettings, ui_state::KmpFilePath,
    unsaved_changes::PendingDocumentAction
};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct EditorKeyBindings {
    #[serde(alias = "select_box")]
    pub default_mode: Vec<KeyCode>,
    pub select_painter: Vec<KeyCode>,
    #[serde(alias = "gizmo")]
    pub transform: Vec<KeyCode>,
    pub translate: Vec<KeyCode>,
    pub rotate: Vec<KeyCode>,
    pub scale: Vec<KeyCode>,
}

impl Default for EditorKeyBindings {
    fn default() -> Self {
        Self {
            default_mode: vec![KeyCode::KeyV],
            select_painter: vec![KeyCode::KeyP],
            transform: vec![KeyCode::KeyG],
            translate: vec![KeyCode::KeyT],
            rotate: vec![KeyCode::KeyR],
            scale: vec![KeyCode::KeyX],
        }
    }
}

pub fn keybinds_plugin(app: &mut App) {
    app.add_systems(Update, keybinds);
}

fn keybinds(
    keys: Res<ButtonInput<KeyCode>>,
    mut file_dialog: FileDialogManager,
    mut editor_mode: ResMut<EditorMode>,
    settings: Res<AppSettings>,
    kmp_path: Option<Res<KmpFilePath>>,
    mut save: MessageWriter<SaveFile>,
    pending_action: Option<Res<PendingDocumentAction>>,
    mut contexts: Query<&mut bevy_egui::EguiContext, With<bevy_egui::PrimaryEguiContext>>,
) {
    // A modal owns the user's decision; no global file shortcut may create a
    // second action or dialog behind it.
    if pending_action.is_some_and(|pending| pending.is_pending()) {
        return;
    }
    // Capture this before Open can close a dialog in the same frame.
    let dialog_was_open = file_dialog.is_open();
    // Single-key tool shortcuts still yield to any focused egui widget.
    // History shortcuts run inside the egui pass, before widgets see input.
    let text_editing = contexts
        .single_mut()
        .is_ok_and(|mut ctx| ctx.get_mut().egui_wants_keyboard_input());

    if keys.keybind_pressed([Modifier::Ctrl], [KeyCode::KeyO]) {
        // open or close file dialog
        if file_dialog.is_open() {
            file_dialog.close();
        } else {
            file_dialog.open_kmp_kcl();
        }
    }

    // Save shortcuts share the menu's request path and never run behind an open
    // file dialog. Ctrl also accepts Cmd through the platform modifier helper.
    if kmp_path.is_some()
        && !dialog_was_open
        && !file_dialog.is_open()
        && keys.keybind_pressed([Modifier::Ctrl], [KeyCode::KeyS])
    {
        // Modifier matching permits extra modifiers, so select exactly one action.
        if keys.shift_pressed() {
            file_dialog.save_kmp();
        } else {
            save.write(SaveFile(None));
        }
    }

    // Editor tool bindings are single-key alternatives. Suppress them while a
    // command modifier is held so bindings such as S do not conflict with Save.
    if !dialog_was_open && !text_editing && !keys.control_or_super_pressed() && !keys.alt_pressed() {
        let bindings = &settings.editor_key_bindings;
        if binding_pressed(&keys, &bindings.default_mode) {
            *editor_mode = EditorMode::Default;
        }
        if binding_pressed(&keys, &bindings.select_painter) {
            *editor_mode = EditorMode::SelectPainter;
        }
        if binding_pressed(&keys, &bindings.translate) {
            *editor_mode = EditorMode::Translate;
        }
        if binding_pressed(&keys, &bindings.rotate) {
            *editor_mode = EditorMode::Rotate;
        }
        if binding_pressed(&keys, &bindings.scale) {
            *editor_mode = EditorMode::Scale;
        }
        if binding_pressed(&keys, &bindings.transform) {
            *editor_mode = EditorMode::Transform;
        }
    }
}

/// Run after egui begins its pass, but before TextEdit handles shortcuts.
/// Merely focusing the next field with Tab must not steal document history.
/// Both this path and the Edit menu queue the same end-of-frame operation.
pub(crate) fn history_shortcuts(ctx: &bevy_egui::egui::Context, world: &mut World) {
    use crate::viewer::kmp::history::{requests_blocked, DocumentHistory};
    use bevy_egui::egui::{Event, Key};
    if requests_blocked(world) {
        return;
    }
    let Some(mut history) = world.get_resource_mut::<DocumentHistory>() else {
        return;
    };
    if history.owns_text_undo(ctx.memory(|m| m.focused())) {
        return; // Preserve native undo AND redo throughout the active session.
    }
    let mut request = None;
    ctx.input_mut(|input| {
        input.events.retain(|event| {
            if let Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } = event
            {
                if !modifiers.alt && (modifiers.ctrl || modifiers.command || modifiers.mac_cmd) {
                    let redo = match key {
                        Key::Z => Some(modifiers.shift),
                        Key::Y => Some(true),
                        _ => None,
                    };
                    if let Some(redo) = redo {
                        request.get_or_insert(redo);
                        return false; // Never let native and document undo both run.
                    }
                }
            }
            true
        });
    });
    if let Some(redo) = request {
        history.request = Some(redo);
        // Drop the numeric widget's edit buffer before restoring document data.
        // Otherwise its stale text can write the undone value back next frame.
        ctx.memory_mut(|m| m.stop_text_input());
    }
}

fn binding_pressed(keys: &ButtonInput<KeyCode>, bindings: &[KeyCode]) -> bool {
    bindings.iter().any(|key| keys.just_pressed(*key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::file_dialog::FileDialogRes;
    use crate::ui::util::FileSource;

    fn save_app(modifier: KeyCode, shift: bool, loaded: bool, dialog_open: bool) -> App {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<FileDialogRes>()
            .init_resource::<EditorMode>()
            .init_resource::<AppSettings>()
            .add_message::<SaveFile>()
            .add_systems(Update, keybinds);
        if loaded {
            app.insert_resource(KmpFilePath("course.kmp".into()));
        }
        if dialog_open {
            app.world_mut().resource_mut::<FileDialogRes>().0 =
                Some((egui_file::FileDialog::open_file(), FileSource::OpenKmpKclDialog));
        }
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(modifier);
        if shift {
            keys.press(KeyCode::ShiftLeft);
        }
        keys.press(KeyCode::KeyS);
        app
    }

    #[test]
    fn undo_shortcuts_choose_one_action_and_do_not_run_behind_dialogs() {
        use crate::viewer::kmp::history::DocumentHistory;
        use bevy_egui::egui::{Context, Event, Key, Modifiers, RawInput};
        for (key, shift, expected) in [(Key::Z, false, false), (Key::Z, true, true), (Key::Y, false, true)] {
            for blocked in [false, true] {
                let mut app = save_app(KeyCode::ControlLeft, shift, true, blocked);
                app.init_resource::<DocumentHistory>();
                let ctx = Context::default();
                ctx.begin_pass(RawInput {
                    events: vec![Event::Key {
                        key,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: Modifiers {
                            ctrl: true,
                            command: true,
                            shift,
                            ..Default::default()
                        },
                    }],
                    ..Default::default()
                });
                history_shortcuts(&ctx, app.world_mut());
                assert_eq!(ctx.input(|i| i.key_pressed(key)), blocked);
                ctx.end_pass().textures_delta.clear();
                assert_eq!(
                    app.world().resource::<DocumentHistory>().request,
                    if blocked { None } else { Some(expected) }
                );
            }
        }
    }

    #[test]
    fn save_shortcuts_select_exactly_one_action() {
        for modifier in [KeyCode::ControlLeft, KeyCode::SuperLeft] {
            for shift in [false, true] {
                let mut app = save_app(modifier, shift, true, false);
                app.update();
                let messages: Vec<_> = app.world_mut().resource_mut::<Messages<SaveFile>>().drain().collect();
                if shift {
                    assert!(messages.is_empty());
                    assert!(matches!(
                        app.world().resource::<FileDialogRes>().0,
                        Some((_, FileSource::SaveKmpDialog))
                    ));
                } else {
                    assert_eq!(messages.len(), 1);
                    assert!(messages[0].0.is_none());
                    assert!(app.world().resource::<FileDialogRes>().0.is_none());
                }
            }
        }
    }

    #[test]
    fn save_shortcuts_require_loaded_kmp_and_no_dialog() {
        for shift in [false, true] {
            for (loaded, dialog_open) in [(false, false), (true, true)] {
                let mut app = save_app(KeyCode::ControlLeft, shift, loaded, dialog_open);
                app.update();
                assert!(app.world().resource::<Messages<SaveFile>>().is_empty());
                let dialog = &app.world().resource::<FileDialogRes>().0;
                if dialog_open {
                    assert!(matches!(dialog, Some((_, FileSource::OpenKmpKclDialog))));
                } else {
                    assert!(dialog.is_none());
                }
            }
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
