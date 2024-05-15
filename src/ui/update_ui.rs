use super::{file_dialog::ShowFileDialog, menu_bar::ShowMenuBar, tabs::ShowDockArea};
use bevy::{prelude::*, window::PrimaryWindow};
use bevy_egui::{egui::TextureId, EguiContext, EguiContexts};
use std::path::PathBuf;

#[derive(SystemSet, Hash, PartialEq, Eq, Clone, Debug)]
pub struct UpdateUiSet;

pub struct UpdateUIPlugin;
impl Plugin for UpdateUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<KmpFileSelected>()
            .add_event::<KclFileSelected>()
            .add_systems(Startup, setup_ui_images)
            .add_systems(Update, update_ui.in_set(UpdateUiSet).run_if(egui_has_primary_context));
    }
}

fn egui_has_primary_context(query: Query<(), (With<EguiContext>, With<PrimaryWindow>)>) -> bool {
    !query.is_empty()
}

pub trait UiSection {
    fn show(&mut self);
}

#[derive(Event)]
pub struct KmpFileSelected(pub PathBuf);

#[derive(Event)]
pub struct KclFileSelected(pub PathBuf);

#[derive(Resource)]
pub struct UiImages {
    pub translate: TextureId,
}

fn setup_ui_images(mut contexts: EguiContexts) {
    egui_extras::install_image_loaders(contexts.ctx_mut());
}

// this function may look small, but this displays all the UI and by extension displays the viewport too. Don't be fooled!
fn update_ui(mut p: ParamSet<(ShowMenuBar, ShowDockArea, ShowFileDialog)>) {
    p.p0().show();
    p.p1().show();
    p.p2().show();
}
