use super::{
    file_dialog::{self, FileDialogManager},
    tabs::{DockTree, Tab},
    ui_state::{KmpFilePath, ResetDockTree, SaveDockTree},
    util::get_egui_ctx,
};
use bevy::ecs::system::{SystemParam, SystemState};
use bevy::prelude::*;
use bevy_egui::{
    egui::{self, Align, Button, Layout},
    EguiContexts,
};
use strum::IntoEnumIterator;

pub fn show_menu_bar(world: &mut World) {
    let ctx = &get_egui_ctx(world);

    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            let mut sc_btn = "Ctrl";
            if cfg!(target_os = "macos") {
                sc_btn = "Cmd";
            }
            ui.menu_button("File", |ui| {
                if ui
                    .add(Button::new("Open KMP/KCL").shortcut_text(format!("{sc_btn}+O")))
                    .clicked()
                {
                    let mut ss = SystemState::<FileDialogManager>::new(world);
                    let mut file_dialog = ss.get_mut(world);

                    file_dialog.open_kmp_kcl();

                    ui.close_menu();
                }
                if !world.contains_resource::<KmpFilePath>() {
                    ui.disable();
                }

                // haven't implemented this yet
                // ui.disable();
                if ui
                    .add(Button::new("Save").shortcut_text(format!("{sc_btn}+S")))
                    .clicked()
                {
                    ui.close_menu();
                }

                if ui
                    .add(Button::new("Save as...").shortcut_text(format!("{sc_btn}+Shift+S")))
                    .clicked()
                {
                    ui.close_menu();
                }
            });
            ui.menu_button("Edit", |ui| {
                // haven't implemented undo/redo yet
                ui.disable();
                if ui
                    .add(Button::new("Undo").shortcut_text(format!("{sc_btn}+Z")))
                    .clicked()
                {
                    // undo!();
                }
                if ui
                    .add(Button::new("Redo").shortcut_text(format!("{sc_btn}+Shift+Z")))
                    .clicked()
                {
                    // redo!();
                }
            });

            ui.menu_button("Window", |ui| {
                if ui.button("Save Tab Layout").clicked() {
                    world.send_event_default::<SaveDockTree>();
                    ui.close_menu();
                }
                if ui.button("Reset Tab Layout").clicked() {
                    world.send_event_default::<ResetDockTree>();
                    ui.close_menu();
                }
                // toggle each tab on or off
                let mut tree = world.resource_mut::<DockTree>();
                for tab in Tab::iter() {
                    // search for the tab and see if it currently exists
                    let tab_in_tree = tree.find_tab(&tab);
                    let mut show_tab = tab_in_tree.is_some();
                    let changed = ui.checkbox(&mut show_tab, tab.to_string()).changed();

                    // if we've changed the check box to checked, create it
                    // else if we've changed the check box and unchecked it, remove the tab
                    if changed && show_tab {
                        tree.push_to_focused_leaf(tab);
                    }
                    if let Some(index) = tab_in_tree {
                        if changed && !show_tab {
                            tree.remove_tab(index);
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
