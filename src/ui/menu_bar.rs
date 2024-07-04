use super::{
    file_dialog::FileDialogManager,
    tabs::{DockTree, Tab},
    ui_state::{ResetDockTree, SaveDockTree},
    update_ui::UiSection,
};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_egui::{
    egui::{self, Align, Layout},
    EguiContexts,
};
use strum::IntoEnumIterator;

#[derive(SystemParam)]
pub struct ShowMenuBar<'w, 's> {
    contexts: EguiContexts<'w, 's>,
    tree: ResMut<'w, DockTree>,
    file_dialog: FileDialogManager<'w>,
    ev_save_docktree: EventWriter<'w, SaveDockTree>,
    ev_reset_docktree: EventWriter<'w, ResetDockTree>,
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
                        self.file_dialog.open_kmp_kcl();
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
                    if ui.button("Save Tab Layout").clicked() {
                        self.ev_save_docktree.send_default();
                        ui.close_menu();
                    }
                    if ui.button("Reset Tab Layout").clicked() {
                        self.ev_reset_docktree.send_default();
                        ui.close_menu();
                    }
                    // toggle each tab on or off
                    for tab in Tab::iter() {
                        // search for the tab and see if it currently exists
                        let tab_in_tree = self.tree.find_tab(&tab);
                        let mut show_tab = tab_in_tree.is_some();
                        let changed = ui.checkbox(&mut show_tab, tab.to_string()).changed();

                        // if we've changed the check box to checked, create it
                        // else if we've changed the check box and unchecked it, remove the tab
                        if changed && show_tab {
                            self.tree.push_to_focused_leaf(tab);
                        }
                        if let Some(index) = tab_in_tree {
                            if changed && !show_tab {
                                self.tree.remove_tab(index);
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
