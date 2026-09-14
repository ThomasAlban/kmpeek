use super::{
    update_ui::{KclFileSelected, KmpFileSelected},
    util::get_egui_ctx,
};
use crate::viewer::kmp::{document, SaveFile};
use bevy::prelude::*;
use bevy_egui::egui;
use std::path::PathBuf;

/// Work that would replace the current document or end the application.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DocumentAction {
    OpenKmp {
        path: PathBuf,
        companion_kcl: Option<PathBuf>,
    },
    Exit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum PromptState {
    Confirm { error: Option<String> },
    Saving,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PendingAction {
    action: DocumentAction,
    state: PromptState,
}

/// The one document-replacing action waiting for confirmation or save completion.
#[derive(Resource, Default)]
pub struct PendingDocumentAction(Option<PendingAction>);

impl PendingDocumentAction {
    /// Used by non-exclusive shortcut systems to avoid opening dialogs behind the modal.
    pub fn is_pending(&self) -> bool {
        self.0.is_some()
    }
}

pub fn unsaved_changes_plugin(app: &mut App) {
    app.init_resource::<PendingDocumentAction>();
}

/// Request an action without replacing a request that is already being handled.
/// Clean documents commit immediately; dirty documents and failed checks prompt.
pub fn request(world: &mut World, action: DocumentAction) {
    world.init_resource::<PendingDocumentAction>();
    if is_pending(world) {
        return;
    }

    match document::has_unsaved_changes(world) {
        Ok(false) => commit(world, action),
        Ok(true) => begin_confirmation(world, action, None),
        Err(error) => begin_confirmation(
            world,
            action,
            Some(format!("Could not check for unsaved changes: {error:#}")),
        ),
    }
}

/// Finish the save started by the warning. Success performs the queued action;
/// failure returns to confirmation so the user can retry or choose another option.
pub fn complete_save(world: &mut World, result: Result<(), String>) {
    let Some(mut pending_resource) = world.get_resource_mut::<PendingDocumentAction>() else {
        return;
    };
    let Some(pending) = pending_resource.0.take() else {
        return;
    };
    drop(pending_resource);
    if pending.state != PromptState::Saving {
        world.resource_mut::<PendingDocumentAction>().0 = Some(pending);
        return;
    }

    match result {
        Ok(()) => commit(world, pending.action),
        Err(error) => begin_confirmation(world, pending.action, Some(error)),
    }
}

pub fn is_pending(world: &World) -> bool {
    world
        .get_resource::<PendingDocumentAction>()
        .is_some_and(|pending| pending.0.is_some())
}

fn begin_confirmation(world: &mut World, action: DocumentAction, error: Option<String>) {
    world.resource_mut::<PendingDocumentAction>().0 = Some(PendingAction {
        action,
        state: PromptState::Confirm { error },
    });
}

fn commit(world: &mut World, action: DocumentAction) {
    match action {
        DocumentAction::OpenKmp { path, companion_kcl } => {
            world.write_message(KmpFileSelected(path));
            if let Some(path) = companion_kcl {
                world.write_message(KclFileSelected(path));
            }
        }
        DocumentAction::Exit => {
            world.write_message(AppExit::Success);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PromptResponse {
    Save,
    Discard,
    Cancel,
}

pub(super) fn show_unsaved_changes(world: &mut World) {
    let Some(pending) = world
        .get_resource::<PendingDocumentAction>()
        .and_then(|pending| pending.0.clone())
    else {
        return;
    };

    let ctx = get_egui_ctx(world);
    let modal = egui::Modal::new("unsaved_changes".into()).show(&ctx, |ui| {
        ui.set_min_width(360.0);
        ui.heading("Save changes?");
        match &pending.action {
            DocumentAction::Exit => ui.label("Save changes to the current KMP before closing KMPeek?"),
            DocumentAction::OpenKmp { path, .. } => ui.label(format!(
                "Save changes to the current KMP before opening {}?",
                path.file_name().unwrap_or(path.as_os_str()).to_string_lossy()
            )),
        };
        ui.add_space(8.0);

        match &pending.state {
            PromptState::Confirm { error } => {
                if let Some(error) = error {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                    ui.add_space(8.0);
                }
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        Some(PromptResponse::Save)
                    } else if ui.button("Don't Save").clicked() {
                        Some(PromptResponse::Discard)
                    } else if ui.button("Cancel").clicked() {
                        Some(PromptResponse::Cancel)
                    } else {
                        None
                    }
                })
                .inner
            }
            PromptState::Saving => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Saving…");
                });
                None
            }
        }
    });

    let response = if modal.should_close() {
        Some(PromptResponse::Cancel)
    } else {
        modal.inner
    };
    apply_prompt_response(world, response);
}

fn apply_prompt_response(world: &mut World, response: Option<PromptResponse>) {
    match response {
        Some(PromptResponse::Save) => {
            let mut pending_resource = world.resource_mut::<PendingDocumentAction>();
            let Some(pending) = pending_resource.0.as_mut() else {
                return;
            };
            if matches!(pending.state, PromptState::Confirm { .. }) {
                pending.state = PromptState::Saving;
                drop(pending_resource);
                world.write_message(SaveFile(None));
            }
        }
        Some(PromptResponse::Discard) => {
            if let Some(pending) = world.resource_mut::<PendingDocumentAction>().0.take() {
                commit(world, pending.action);
            }
        }
        Some(PromptResponse::Cancel) => {
            world.resource_mut::<PendingDocumentAction>().0 = None;
        }
        None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<PendingDocumentAction>()
            .add_message::<KmpFileSelected>()
            .add_message::<KclFileSelected>()
            .add_message::<SaveFile>()
            .add_message::<AppExit>();
        app
    }

    #[test]
    fn save_success_commits_open_with_companion() {
        let mut app = app();
        let kmp = PathBuf::from("course.kmp");
        let kcl = PathBuf::from("course.kcl");
        app.world_mut().resource_mut::<PendingDocumentAction>().0 = Some(PendingAction {
            action: DocumentAction::OpenKmp {
                path: kmp.clone(),
                companion_kcl: Some(kcl.clone()),
            },
            state: PromptState::Saving,
        });

        complete_save(app.world_mut(), Ok(()));

        assert!(!is_pending(app.world()));
        assert_eq!(
            app.world_mut()
                .resource_mut::<Messages<KmpFileSelected>>()
                .drain()
                .next()
                .map(|selected| selected.0),
            Some(kmp)
        );
        assert_eq!(
            app.world_mut()
                .resource_mut::<Messages<KclFileSelected>>()
                .drain()
                .next()
                .map(|selected| selected.0),
            Some(kcl)
        );
    }

    #[test]
    fn save_failure_returns_to_confirmation_with_error() {
        let mut app = app();
        app.world_mut().resource_mut::<PendingDocumentAction>().0 = Some(PendingAction {
            action: DocumentAction::Exit,
            state: PromptState::Saving,
        });

        complete_save(app.world_mut(), Err("disk full".into()));

        let pending = app.world().resource::<PendingDocumentAction>().0.as_ref().unwrap();
        assert_eq!(
            pending.state,
            PromptState::Confirm {
                error: Some("disk full".into())
            }
        );
        assert!(app
            .world_mut()
            .resource_mut::<Messages<AppExit>>()
            .drain()
            .next()
            .is_none());
    }

    #[test]
    fn discard_commits_exit_and_cancel_clears_it() {
        let mut app = app();
        begin_confirmation(app.world_mut(), DocumentAction::Exit, None);
        apply_prompt_response(app.world_mut(), Some(PromptResponse::Cancel));
        assert!(!is_pending(app.world()));
        assert!(app
            .world_mut()
            .resource_mut::<Messages<AppExit>>()
            .drain()
            .next()
            .is_none());

        begin_confirmation(app.world_mut(), DocumentAction::Exit, None);
        apply_prompt_response(app.world_mut(), Some(PromptResponse::Discard));
        assert!(!is_pending(app.world()));
        assert_eq!(
            app.world_mut().resource_mut::<Messages<AppExit>>().drain().next(),
            Some(AppExit::Success)
        );
    }

    #[test]
    fn save_response_enters_saving_once() {
        let mut app = app();
        begin_confirmation(app.world_mut(), DocumentAction::Exit, None);

        apply_prompt_response(app.world_mut(), Some(PromptResponse::Save));
        apply_prompt_response(app.world_mut(), Some(PromptResponse::Save));

        let pending = app.world().resource::<PendingDocumentAction>().0.as_ref().unwrap();
        assert_eq!(pending.state, PromptState::Saving);
        assert_eq!(app.world_mut().resource_mut::<Messages<SaveFile>>().drain().count(), 1);
    }
}
