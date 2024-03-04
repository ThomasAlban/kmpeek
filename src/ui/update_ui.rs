use super::{file_dialog::ShowFileDialog, menu_bar::ShowMenuBar, tabs::ShowDockArea};
use bevy::prelude::*;
use bevy_egui_next::{egui::TextureId, EguiContexts};
use std::path::PathBuf;

#[derive(SystemSet, Hash, PartialEq, Eq, Clone, Debug)]
pub struct UpdateUiSet;

pub struct UpdateUIPlugin;
impl Plugin for UpdateUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<KmpFileSelected>()
            .add_event::<KclFileSelected>()
            .add_systems(Startup, setup_ui_images)
            .add_systems(Update, update_ui.in_set(UpdateUiSet));
    }
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

fn update_ui(mut p: ParamSet<(ShowMenuBar, ShowDockArea, ShowFileDialog)>) {
    p.p0().show();
    p.p1().show();
    p.p2().show();
}
