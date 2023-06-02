use super::resources::AppState;
use crate::{
    camera::{CameraMode, CameraSettings, FlyCam, OrbitCam},
    kcl::*,
    kmp::*,
};
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use num_traits::{Float, PrimInt};

// annoyingly egui doesn't have a number input widget, so we have to make our own
fn float_input<T>(ui: &mut egui::Ui, num: &mut T, default: T, buf: &mut String)
where
    T: Float + std::str::FromStr + std::fmt::Display + Default,
{
    // save what was previously in the buffer
    let buf_before = buf.clone();
    ui.add(
        egui::TextEdit::singleline(buf)
            .desired_width(40.0f32)
            .hint_text(default.to_string()),
    );
    // if the buffer has changed, try to parse it
    if *buf != buf_before {
        match buf.parse() {
            // if the buf is still valid, set the actual number to the parsed number
            Ok(parsed) => *num = parsed,
            Err(_) => {
                if buf.is_empty() {
                    // this allows the user to delete the number without the default suddely appearing in the field
                    *num = default;
                } else {
                    // if the buf is invalid, set it back to what it was before
                    // this stops them from typing anything that doesn't parse to a number into the field
                    *buf = buf_before;
                }
            }
        }
        return;
    }
    // if num has been changed externally, update the buffer so that the displayed value doesn't get out
    // of sync with the actual number
    if *num != buf.parse().unwrap_or(default) {
        *buf = num.to_string();
    }
    // the reason why we have all this shite rather than just having a local buf variable, using that in the
    // textedit and then parsing it, is because let's say the user types in "1" and wants to add a decimal
    // to it, every time they try to add a "." it would be removed because "1." parses to 1
}
fn int_input<T>(ui: &mut egui::Ui, num: &mut T, default: T, buf: &mut String)
where
    T: PrimInt + std::str::FromStr + std::fmt::Display + Default,
{
    let buf_before = buf.clone();
    ui.add(
        egui::TextEdit::singleline(buf)
            .desired_width(40.0f32)
            .hint_text(default.to_string()),
    );
    if *buf != buf_before {
        match buf.parse() {
            Ok(parsed) => *num = parsed,
            Err(_) => {
                if buf.is_empty() {
                    *num = default;
                } else {
                    *buf = buf_before;
                }
            }
        }
        return;
    }
    if *num != buf.parse().unwrap_or(default) {
        *buf = num.to_string();
    }
}

pub fn update_ui(
    mut contexts: EguiContexts,
    mut kcl: ResMut<Kcl>,
    mut kmp: ResMut<Kmp>,
    mut app_state: ResMut<AppState>,
    mut camera_settings: ResMut<CameraSettings>,
    mut fly_cam: Query<&mut Camera, (With<FlyCam>, Without<OrbitCam>)>,
    mut orbit_cam: Query<&mut Camera, (With<OrbitCam>, Without<FlyCam>)>,
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
        .open(&mut app_state.customise_kcl_open)
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
            egui::CollapsingHeader::new("View Options")
                .default_open(true)
                .show_background(true)
                .show(ui, |ui| {
                    ui.collapsing("Collision Model", |ui| {
                        let (
                            mut show_walls,
                            mut show_invisible_walls,
                            mut show_death_barriers,
                            mut show_effects_triggers,
                        ) = (
                            app_state.show_walls,
                            app_state.show_invisible_walls,
                            app_state.show_death_barriers,
                            app_state.show_effects_triggers,
                        );
                        ui.checkbox(&mut show_walls, "Show Walls");
                        ui.checkbox(&mut show_invisible_walls, "Show Invisible Walls");
                        ui.checkbox(&mut show_death_barriers, "Show Death Barriers");
                        ui.checkbox(&mut show_effects_triggers, "Show Effects & Triggers");
                        if show_walls != app_state.show_walls {
                            app_state.show_walls = show_walls;
                            kcl.vertex_groups[KclFlag::Wall1 as usize].visible = show_walls;
                            kcl.vertex_groups[KclFlag::Wall2 as usize].visible = show_walls;
                            kcl.vertex_groups[KclFlag::WeakWall as usize].visible = show_walls;
                        }
                        if show_invisible_walls != app_state.show_invisible_walls {
                            app_state.show_invisible_walls = show_invisible_walls;
                            kcl.vertex_groups[KclFlag::InvisibleWall1 as usize].visible =
                                show_invisible_walls;
                            kcl.vertex_groups[KclFlag::InvisibleWall2 as usize].visible =
                                show_invisible_walls;
                        }
                        if show_death_barriers != app_state.show_death_barriers {
                            app_state.show_death_barriers = show_death_barriers;
                            kcl.vertex_groups[KclFlag::SolidFall as usize].visible =
                                show_death_barriers;
                            kcl.vertex_groups[KclFlag::FallBoundary as usize].visible =
                                show_death_barriers;
                        }
                        if show_effects_triggers != app_state.show_effects_triggers {
                            app_state.show_effects_triggers = show_effects_triggers;
                            kcl.vertex_groups[KclFlag::ItemStateModifier as usize].visible =
                                show_effects_triggers;
                            kcl.vertex_groups[KclFlag::EffectTrigger as usize].visible =
                                show_effects_triggers;
                            kcl.vertex_groups[KclFlag::SoundTrigger as usize].visible =
                                show_effects_triggers;
                            kcl.vertex_groups[KclFlag::CannonTrigger as usize].visible =
                                show_effects_triggers;
                        }
                        if ui.button("Customise...").clicked() {
                            app_state.customise_kcl_open = true;
                        }
                    });

                    ui.collapsing("Camera", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Mode:");
                            if ui.button("Fly").clicked() {
                                camera_settings.mode = CameraMode::Fly;
                                if let Ok(mut fly_cam) = fly_cam.get_single_mut() {
                                    fly_cam.is_active = true;
                                }
                                if let Ok(mut orbit_cam) = orbit_cam.get_single_mut() {
                                    orbit_cam.is_active = false;
                                }
                            }
                            if ui.button("Orbit").clicked() {
                                camera_settings.mode = CameraMode::Orbit;
                                if let Ok(mut fly_cam) = fly_cam.get_single_mut() {
                                    fly_cam.is_active = false;
                                }
                                if let Ok(mut orbit_cam) = orbit_cam.get_single_mut() {
                                    orbit_cam.is_active = true;
                                }
                            }
                        });
                    });
                });

            ui.separator();

            // ui.collapsing("STGI - Stage Info", |ui| {
            //     ui.reset_style();
            //     ui.horizontal(|ui| {
            //         let mut buf = kmp.stgi.entries[0].lap_count.to_string();
            //         ui.label("Lap Count: ");
            //         int_input(
            //             ui,
            //             &mut kmp.stgi.entries[0].lap_count,
            //             3,
            //             &mut app_state.lap_count_buf,
            //         );
            //         ui.add(egui::TextEdit::singleline(&mut buf).desired_width(40.0f32));
            //         sanitize_string(&mut buf, 1);
            //         if buf != kmp.stgi.entries[0].lap_count.to_string() {
            //             kmp.stgi.entries[0].lap_count = buf.parse().unwrap_or(3);
            //         }
            //     });

            //     ui.horizontal(|ui| {
            //         ui.label("Pole Position: ");
            //         let mut pole_pos = kmp.stgi.entries[0].pole_pos;
            //         ui.selectable_value(&mut pole_pos, 0, "Left");
            //         ui.selectable_value(&mut pole_pos, 1, "Right");
            //         if pole_pos != kmp.stgi.entries[0].pole_pos {
            //             kmp.stgi.entries[0].pole_pos = pole_pos;
            //         }
            //     });

            //     ui.horizontal(|ui| {
            //         ui.label("Driver Distance: ");
            //         let mut driver_distance = kmp.stgi.entries[0].driver_distance;
            //         ui.selectable_value(&mut driver_distance, 0, "Normal");
            //         ui.selectable_value(&mut driver_distance, 1, "Narrow");
            //         if driver_distance != kmp.stgi.entries[0].driver_distance {
            //             kmp.stgi.entries[0].driver_distance = driver_distance;
            //         }
            //     });

            //     let mut lens_flare_flashing = kmp.stgi.entries[0].lens_flare_flashing != 0;
            //     ui.checkbox(&mut lens_flare_flashing, "Lens Flare Flashing");
            //     if (kmp.stgi.entries[0].lens_flare_flashing != 0) != lens_flare_flashing {
            //         kmp.stgi.entries[0].lens_flare_flashing =
            //             if lens_flare_flashing { 1 } else { 0 };
            //     }

            //     ui.horizontal(|ui| {
            //         let mut rgba = kmp.stgi.entries[0].flare_colour;
            //         ui.label("Flare Colour: ");
            //         ui.color_edit_button_srgba_unmultiplied(&mut rgba);
            //         if rgba != kmp.stgi.entries[0].flare_colour {
            //             kmp.stgi.entries[0].flare_colour = rgba
            //         }
            //     });
            //     ui.horizontal(|ui| {
            //         ui.label("Speed Mod: ");
            //         float_input(
            //             ui,
            //             &mut kmp.stgi.entries[0].speed_mod,
            //             0.,
            //             &mut app_state.speed_mod_buf,
            //         );
            //     });
            // });
        });
}
