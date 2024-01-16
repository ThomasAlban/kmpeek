use bevy::ecs::system::SystemParam;

use super::UiSubSection;

#[derive(SystemParam)]
pub struct ShowEditTab {
    //
}
impl UiSubSection for ShowEditTab {
    fn show(&mut self, _ui: &mut bevy_egui::egui::Ui) {
        // this is where ui for the currently selected point(s) will be
    }
}
