use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_egui::{
    egui::{self, Align, Layout},
    EguiContexts,
};

use strum::IntoEnumIterator;

use super::{
    app_state::AppState,
    file_dialog::ShowFileDialog,
    tabs::{DockTree, Tab},
    update_ui::UiSection,
};

#[derive(SystemParam)]
pub struct ShowMenuBar<'w, 's> {
    contexts: EguiContexts<'w, 's>,
    tree: ResMut<'w, DockTree>,
    app_state: ResMut<'w, AppState>,
}
impl UiSection for ShowMenuBar<'_, '_> {
    fn show(&mut self) {
        let ctx = self.contexts.ctx_mut();

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                let mut sc_btn = "Ctrl";
                if cfg!(target_os = "macos") {
                    sc_btn = "Cmd";
                }
                ui.menu_button("File", |ui| {
                    if ui
                        .add(egui::Button::new("Open KMP/KCL").shortcut_text(format!("{sc_btn}+O")))
                        .clicked()
                    {
                        ShowFileDialog::open_kmp_kcl(&mut self.app_state);
                        ui.close_menu();
                    }
                    if ui
                        .add(egui::Button::new("Save").shortcut_text(format!("{sc_btn}+S")))
                        .clicked()
                    {
                        ui.close_menu();
                    }
                });
                ui.menu_button("Edit", |ui| {
                    if ui
                        .add(egui::Button::new("Undo").shortcut_text(format!("{sc_btn}+Z")))
                        .clicked()
                    {
                        // undo!();
                    }
                    if ui
                        .add(egui::Button::new("Redo").shortcut_text(format!("{sc_btn}+Shift+Z")))
                        .clicked()
                    {
                        // redo!();
                    }
                });

                ui.menu_button("Window", |ui| {
                    // toggle each tab on or off
                    for tab in Tab::iter() {
                        // search for the tab and see if it currently exists
                        let tab_in_tree = self.tree.find_tab(&tab);
                        if ui
                            .selectable_label(tab_in_tree.is_some(), tab.to_string())
                            .clicked()
                        {
                            // remove if it exists, else create it
                            if let Some(index) = tab_in_tree {
                                self.tree.remove_tab(index);
                            } else {
                                self.tree.push_to_focused_leaf(tab);
                            }
                        }
                    }
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.hyperlink_to("Thomas Alban", "https://github.com/ThomasAlban");
                    ui.label("Made by");
                });
            });
        });
    }
}
