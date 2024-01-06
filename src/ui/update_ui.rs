use super::{
    file_dialog::ShowFileDialog, menu_bar::ShowMenuBar, tabs::ShowDockArea, top_bar::ShowTopBar,
};
use bevy::prelude::*;
use std::path::PathBuf;

pub struct UpdateUIPlugin;
impl Plugin for UpdateUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<KmpFileSelected>()
            .add_event::<KclFileSelected>()
            .add_systems(Update, update_ui);
    }
}

pub trait UiSection {
    fn show(&mut self);
}

#[derive(Event)]
pub struct KmpFileSelected(pub PathBuf);

#[derive(Event)]
pub struct KclFileSelected(pub PathBuf);

fn update_ui(mut p: ParamSet<(ShowMenuBar, ShowTopBar, ShowDockArea, ShowFileDialog)>) {
    p.p0().show();
    p.p1().show();
    p.p2().show();
    p.p3().show();
}
