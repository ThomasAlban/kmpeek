use super::resources::UIOptions;
use crate::kcl::Kcl;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

pub fn update_ui(
    mut contexts: EguiContexts,
    mut kcl: ResMut<Kcl>,
    mut ui_options: ResMut<UIOptions>,
) {
    let ctx = contexts.ctx_mut();

    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Open").clicked() {
                    // …
                }
            });
            ui.menu_button("Edit", |ui| {
                if ui.button("Undo").clicked() {
                    // …
                }
                if ui.button("Redo").clicked() {
                    // …
                }
            });
            if ui.button("Settings").clicked() {
                ui_options.settings_open = !ui_options.settings_open;
            }
        });
    });

    egui::Window::new("Settings")
        .open(&mut ui_options.settings_open)
        .collapsible(false)
        .min_width(300.)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.collapsing("KCL Options", |ui| {
                    // this macro means that the same ui options can be repeated without copy and pasting it 32 times
                    macro_rules! kcl_type_options {
                        ($name:expr, $i:expr) => {
                            ui.horizontal(|ui| {
                                let (mut color, mut visible) =
                                    (kcl.vertex_groups[$i].color, kcl.vertex_groups[$i].visible);
                                ui.color_edit_button_rgba_unmultiplied(&mut color);
                                ui.checkbox(&mut visible, $name);
                                // only update the kcl if the variables have been changed in the UI
                                if color != kcl.vertex_groups[$i].color
                                    || visible != kcl.vertex_groups[$i].visible
                                {
                                    kcl.vertex_groups[$i].color = color;
                                    kcl.vertex_groups[$i].visible = visible;
                                }
                            });
                        };
                    }
                    kcl_type_options!("Road1", 0);
                    kcl_type_options!("SlipperyRoad1", 1);
                    kcl_type_options!("WeakOffroad", 2);
                    kcl_type_options!("Offroad", 3);
                    kcl_type_options!("HeavyOffroad", 4);
                    kcl_type_options!("SlipperyRoad2", 5);
                    kcl_type_options!("BoostPanel", 6);
                    kcl_type_options!("BoostRamp", 7);
                    kcl_type_options!("SlowRamp", 8);
                    kcl_type_options!("ItemRoad", 9);
                    kcl_type_options!("SolidFall", 10);
                    kcl_type_options!("MovingWater", 11);
                    kcl_type_options!("Wall1", 12);
                    kcl_type_options!("InvisibleWall1", 13);
                    kcl_type_options!("ItemWall", 14);
                    kcl_type_options!("Wall2", 15);
                    kcl_type_options!("FallBoundary", 16);
                    kcl_type_options!("CannonTrigger", 17);
                    kcl_type_options!("ForceRecalculation", 18);
                    kcl_type_options!("HalfPipeRamp", 19);
                    kcl_type_options!("PlayerOnlyWall", 20);
                    kcl_type_options!("MovingRoad", 21);
                    kcl_type_options!("StickyRoad", 22);
                    kcl_type_options!("Road2", 23);
                    kcl_type_options!("SoundTrigger", 24);
                    kcl_type_options!("WeakWall", 25);
                    kcl_type_options!("EffectTrigger", 26);
                    kcl_type_options!("ItemStateModifier", 27);
                    kcl_type_options!("HalfPipeInvisibleWall", 28);
                    kcl_type_options!("RotatingRoad", 29);
                    kcl_type_options!("SpecialWall", 30);
                    kcl_type_options!("InvisibleWall2", 31);
                });
                ui.separator();
                ui.collapsing("Camera Options", |ui| {
                    ui.label("Camera Sensitivity");
                })
            })
        });

    egui::SidePanel::left("side_panel")
        .resizable(true)
        .show(ctx, |ui| {
            ui.heading("KMPeek");
            ui.separator();
        });
}
