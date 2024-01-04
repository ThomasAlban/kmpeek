use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_egui::{
    egui::{self},
    EguiContexts,
};

use strum::IntoEnumIterator;

use super::{
    app_state::AppState,
    file_dialog::open_kmp_kcl_file_dialog,
    tabs::{DockTree, Tab},
};

#[derive(SystemParam)]
pub struct MenuBarParams<'w, 's> {
    contexts: EguiContexts<'w, 's>,
    tree: ResMut<'w, DockTree>,
    app_state: ResMut<'w, AppState>,
}

pub fn show_menu_bar(mut p: MenuBarParams) {
    let ctx = p.contexts.ctx_mut();

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
                    open_kmp_kcl_file_dialog(&mut p.app_state);
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
                    let tab_in_tree = p.tree.find_tab(&tab);
                    if ui
                        .selectable_label(tab_in_tree.is_some(), tab.to_string())
                        .clicked()
                    {
                        // remove if it exists, else create it
                        if let Some(index) = tab_in_tree {
                            p.tree.remove_tab(index);
                        } else {
                            p.tree.push_to_focused_leaf(tab);
                        }
                    }
                }
            });
        });
    });
}
