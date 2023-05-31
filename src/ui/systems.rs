use super::resources::UIOptions;
use crate::utils::*;
use crate::{kcl::*, kmp::*};
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

pub fn update_ui(
    mut contexts: EguiContexts,
    mut kcl: ResMut<Kcl>,
    mut kmp: ResMut<Kmp>,
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
        });
    });

    egui::Window::new("Customise Collision Model")
        .open(&mut ui_options.customise_kcl_open)
        .collapsible(false)
        .min_width(300.)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Check All").clicked() {
                        for vertex_group in kcl.vertex_groups.iter_mut() {
                            vertex_group.visible = true;
                        }
                    }
                    if ui.button("Uncheck All").clicked() {
                        for vertex_group in kcl.vertex_groups.iter_mut() {
                            vertex_group.visible = false;
                        }
                    }
                    if ui.button("Reset").clicked() {
                        for (i, vertex_group) in kcl.vertex_groups.iter_mut().enumerate() {
                            vertex_group.visible = true;
                            vertex_group.colour = KCL_COLOURS[i];
                        }
                    }
                });
                ui.separator();

                // this macro means that the same ui options can be repeated without copy and pasting it 32 times
                macro_rules! kcl_type_options {
                    ($name:expr, $i:expr) => {
                        ui.horizontal(|ui| {
                            let (mut colour, mut visible) =
                                (kcl.vertex_groups[$i].colour, kcl.vertex_groups[$i].visible);
                            ui.color_edit_button_rgba_unmultiplied(&mut colour);
                            ui.checkbox(&mut visible, $name);
                            // only update the kcl if the variables have been changed in the UI
                            if colour != kcl.vertex_groups[$i].colour
                                || visible != kcl.vertex_groups[$i].visible
                            {
                                kcl.vertex_groups[$i].colour = colour;
                                kcl.vertex_groups[$i].visible = visible;
                            }
                        });
                        ui.separator();
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
        });

    egui::SidePanel::left("side_panel")
        .resizable(true)
        .show(ctx, |ui| {
            ui.heading("KMPeek");
            ui.separator();
            ui.collapsing("Collision Model", |ui| {
                let (
                    mut show_walls,
                    mut show_invisible_walls,
                    mut show_death_barriers,
                    mut show_effects_triggers,
                ) = (
                    ui_options.show_walls,
                    ui_options.show_invisible_walls,
                    ui_options.show_death_barriers,
                    ui_options.show_effects_triggers,
                );
                ui.checkbox(&mut show_walls, "Show Walls");
                ui.checkbox(&mut show_invisible_walls, "Show Invisible Walls");
                ui.checkbox(&mut show_death_barriers, "Show Death Barriers");
                ui.checkbox(&mut show_effects_triggers, "Show Effects & Triggers");
                if show_walls != ui_options.show_walls {
                    ui_options.show_walls = show_walls;
                    kcl.vertex_groups[KclFlag::Wall1 as usize].visible = show_walls;
                    kcl.vertex_groups[KclFlag::Wall2 as usize].visible = show_walls;
                    kcl.vertex_groups[KclFlag::WeakWall as usize].visible = show_walls;
                }
                if show_invisible_walls != ui_options.show_invisible_walls {
                    ui_options.show_invisible_walls = show_invisible_walls;
                    kcl.vertex_groups[KclFlag::InvisibleWall1 as usize].visible =
                        show_invisible_walls;
                    kcl.vertex_groups[KclFlag::InvisibleWall2 as usize].visible =
                        show_invisible_walls;
                }
                if show_death_barriers != ui_options.show_death_barriers {
                    ui_options.show_death_barriers = show_death_barriers;
                    kcl.vertex_groups[KclFlag::SolidFall as usize].visible = show_death_barriers;
                    kcl.vertex_groups[KclFlag::FallBoundary as usize].visible = show_death_barriers;
                }
                if show_effects_triggers != ui_options.show_effects_triggers {
                    ui_options.show_effects_triggers = show_effects_triggers;
                    kcl.vertex_groups[KclFlag::ItemStateModifier as usize].visible =
                        show_effects_triggers;
                    kcl.vertex_groups[KclFlag::EffectTrigger as usize].visible =
                        show_effects_triggers;
                    kcl.vertex_groups[KclFlag::SoundTrigger as usize].visible =
                        show_effects_triggers;
                    kcl.vertex_groups[KclFlag::CannonTrigger as usize].visible =
                        show_effects_triggers;
                }
                if ui.button("Customise").clicked() {
                    ui_options.customise_kcl_open = true;
                }
            });

            ui.collapsing("STGI - Stage Info", |ui| {
                ui.horizontal(|ui| {
                    let mut buf = kmp.stgi.entries[0].lap_count.to_string();
                    ui.label("Lap Count: ");
                    ui.add(egui::TextEdit::singleline(&mut buf).desired_width(40.0f32));
                    sanitize_string(&mut buf, 1);
                    if buf != kmp.stgi.entries[0].lap_count.to_string() {
                        kmp.stgi.entries[0].lap_count = buf.parse().unwrap_or(3);
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Pole Position: ");
                    let mut pole_pos = kmp.stgi.entries[0].pole_pos;
                    ui.selectable_value(&mut pole_pos, 0, "Left");
                    ui.selectable_value(&mut pole_pos, 1, "Right");
                    if pole_pos != kmp.stgi.entries[0].pole_pos {
                        kmp.stgi.entries[0].pole_pos = pole_pos;
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Driver Distance: ");
                    let mut driver_distance = kmp.stgi.entries[0].driver_distance;
                    ui.selectable_value(&mut driver_distance, 0, "Normal");
                    ui.selectable_value(&mut driver_distance, 1, "Narrow");
                    if driver_distance != kmp.stgi.entries[0].driver_distance {
                        kmp.stgi.entries[0].driver_distance = driver_distance;
                    }
                });

                let mut lens_flare_flashing = kmp.stgi.entries[0].lens_flare_flashing != 0;
                ui.checkbox(&mut lens_flare_flashing, "Lens Flare Flashing");
                if (kmp.stgi.entries[0].lens_flare_flashing != 0) != lens_flare_flashing {
                    kmp.stgi.entries[0].lens_flare_flashing =
                        if lens_flare_flashing { 1 } else { 0 };
                }

                ui.horizontal(|ui| {
                    let mut rgba = kmp.stgi.entries[0].flare_colour;
                    ui.label("Flare Colour: ");
                    ui.color_edit_button_srgba_unmultiplied(&mut rgba);
                    if rgba != kmp.stgi.entries[0].flare_colour {
                        kmp.stgi.entries[0].flare_colour = rgba
                    }
                });

                ui.horizontal(|ui| {
                    let mut buf = kmp.stgi.entries[0].speed_mod.to_string();
                    if !buf.contains('.') {
                        // add on a .0 at the end if it is a whole number
                        buf += ".0";
                    }
                    ui.label("Speed Mod: ");
                    ui.add(egui::TextEdit::singleline(&mut buf).desired_width(40.0f32));
                    if buf.parse().unwrap_or(0.) != kmp.stgi.entries[0].speed_mod {
                        kmp.stgi.entries[0].speed_mod = buf.parse().unwrap_or(0.);
                    }
                });
            });
        });
}
