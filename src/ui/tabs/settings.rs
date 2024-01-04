use super::super::app_state::AppSettings;
use crate::{
    ui::{
        app_state::AppState,
        file_dialog::{export_settings_file_dialog, import_settings_file_dialog},
    },
    util::kcl_file::KclFlag,
    viewer::{
        camera::{
            CameraMode, CameraModeChanged, CameraSettings, FlyCam, FlySettings, OrbitCam,
            OrbitSettings, TopDownCam, TopDownSettings,
        },
        kcl_model::KclModelUpdated,
        kmp::{sections::KmpSections, settings::KmpSectionSettings},
    },
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui;
use strum::IntoEnumIterator;

#[derive(SystemParam)]
pub struct SettingsParams<'w, 's> {
    settings: ResMut<'w, AppSettings>,
    app_state: ResMut<'w, AppState>,
    ev_kcl_model_updated: EventWriter<'w, KclModelUpdated>,
    ev_camera_mode_changed: EventWriter<'w, CameraModeChanged>,
    cams: (
        // fly cam
        Query<
            'w,
            's,
            &'static mut Transform,
            (With<FlyCam>, Without<OrbitCam>, Without<TopDownCam>),
        >,
        // orbit cam
        Query<
            'w,
            's,
            &'static mut Transform,
            (Without<FlyCam>, With<OrbitCam>, Without<TopDownCam>),
        >,
        // topdown cam
        Query<
            'w,
            's,
            (&'static mut Transform, &'static mut Projection),
            (Without<FlyCam>, Without<OrbitCam>, With<TopDownCam>),
        >,
    ),
}

pub fn show_settings_tab(ui: &mut egui::Ui, p: &mut SettingsParams) {
    let mut fly_cam = p.cams.0.get_single_mut().unwrap();
    let mut orbit_cam = p.cams.1.get_single_mut().unwrap();
    let mut topdown_cam = p.cams.2.get_single_mut().unwrap();

    ui.label("These settings will be saved when you close the app.");

    egui::CollapsingHeader::new("KMP Viewer")
        .default_open(true)
        .show(ui, |ui| {
            ui.add(
                egui::Slider::new(&mut p.settings.kmp_model.point_scale, 0.01..=2.)
                    .text("Point Scale"),
            );
            ui.checkbox(&mut p.settings.kmp_model.normalize, "Normalize points");

            ui.checkbox(
                &mut p.settings.open_course_kcl_in_directory,
                "Auto open course.kcl",
            ).on_hover_text("If enabled, when opening a KMP file, if there is a 'course.kcl' file in the same directory, it will also be opened");

            ui.collapsing("Customise Colours", |ui| {
                for (i, section_name) in KmpSections::iter().enumerate() {
                    let Some(section) = p.settings.kmp_model.sections.field_at_mut(i) else {
                        continue;
                    };

                    if let Some(section) = section.downcast_mut::<KmpSectionSettings<Color>>() {
                        ui.horizontal(|ui| {
                            let mut color = section.color.as_rgba_f32();
                            ui.color_edit_button_rgba_unmultiplied(&mut color);
                            section.color = color.into();
                            ui.label(section_name.to_string());
                        });
                    }

                }
            });
        });

    egui::CollapsingHeader::new("Collision Model")
        .default_open(true)
        .show(ui, |ui| {
            ui.checkbox(
                &mut p.settings.kcl_model.backface_culling,
                "Backface Culling",
            )
            .on_hover_text("Whether or not the back faces of the collision model are shown");
            let kcl_model_settings_before = p.settings.kcl_model.clone();

            let visible = &mut p.settings.kcl_model.visible;

            use KclFlag::*;

            let mut show_walls =
                visible[Wall1 as usize] && visible[Wall2 as usize] && visible[WeakWall as usize];
            let mut show_invis_walls =
                visible[InvisibleWall1 as usize] && visible[InvisibleWall2 as usize];
            let mut show_death_barriers =
                visible[SolidFall as usize] && visible[FallBoundary as usize];
            let mut show_effects_triggers = visible[ItemStateModifier as usize]
                && visible[EffectTrigger as usize]
                && visible[SoundTrigger as usize]
                && visible[KclFlag::CannonTrigger as usize];

            let show_walls_changed = ui.checkbox(&mut show_walls, "Show Walls").changed();
            let show_invis_walls_changed = ui
                .checkbox(&mut show_invis_walls, "Show Invisible Walls")
                .changed();
            let show_death_barriers_changed = ui
                .checkbox(&mut show_death_barriers, "Show Death Barriers")
                .changed();
            let show_effects_triggers_changed = ui
                .checkbox(&mut show_effects_triggers, "Show Effects & Triggers")
                .changed();

            if show_walls_changed {
                [
                    visible[Wall1 as usize],
                    visible[Wall2 as usize],
                    visible[WeakWall as usize],
                ] = [show_walls; 3];
            }

            if show_invis_walls_changed {
                [
                    visible[InvisibleWall1 as usize],
                    visible[InvisibleWall2 as usize],
                ] = [show_invis_walls; 2];
            }
            if show_death_barriers_changed {
                [visible[SolidFall as usize], visible[FallBoundary as usize]] =
                    [show_death_barriers; 2];
            }
            if show_effects_triggers_changed {
                [
                    visible[ItemStateModifier as usize],
                    visible[EffectTrigger as usize],
                    visible[SoundTrigger as usize],
                    visible[CannonTrigger as usize],
                ] = [show_effects_triggers; 4];
            }

            ui.collapsing("Customise Colours", |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Check All").clicked() {
                        p.settings.kcl_model.visible = [true; 32];
                    }
                    if ui.button("Uncheck All").clicked() {
                        p.settings.kcl_model.visible = [false; 32];
                    }
                    if ui.button("Reset").clicked() {
                        p.settings.kcl_model = default();
                    }
                });
                // show colour edit and visibility toggle for each kcl flag variant
                for (i, kcl_flag) in KclFlag::iter().enumerate() {
                    ui.horizontal(|ui| {
                        let mut color = p.settings.kcl_model.color[i].as_rgba_f32();
                        ui.color_edit_button_rgba_unmultiplied(&mut color);
                        p.settings.kcl_model.color[i] = color.into();
                        ui.checkbox(&mut p.settings.kcl_model.visible[i], kcl_flag.to_string());
                    });
                }
            });
            if p.settings.kcl_model != kcl_model_settings_before {
                p.ev_kcl_model_updated.send_default();
            }
        });

    egui::CollapsingHeader::new("Camera").default_open(true).show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.selectable_value(&mut p.settings.camera.mode, CameraMode::Fly, "Fly").clicked() {
                    p.ev_camera_mode_changed.send(CameraModeChanged(CameraMode::Fly));
                }
                if ui.selectable_value(&mut p.settings.camera.mode, CameraMode::Orbit, "Orbit").clicked() {
                    p.ev_camera_mode_changed.send(CameraModeChanged(CameraMode::Orbit));
                }
                if ui.selectable_value(&mut p.settings.camera.mode, CameraMode::TopDown, "Top Down").clicked() {
                    p.ev_camera_mode_changed.send(CameraModeChanged(CameraMode::TopDown));
                }
            });

            ui.horizontal(|ui| {
                if ui.button("Reset Positions").clicked() {
                    let fly_default = FlySettings::default();
                    let orbit_default = OrbitSettings::default();
                    let topdown_default = TopDownSettings::default();
                    *fly_cam = Transform::from_translation(fly_default.start_pos)
                        .looking_at(Vec3::ZERO, Vec3::Y);
                    *orbit_cam = Transform::from_translation(orbit_default.start_pos)
                        .looking_at(Vec3::ZERO, Vec3::Y);
                    *topdown_cam.0 = Transform::from_translation(topdown_default.start_pos)
                        .looking_at(Vec3::ZERO, Vec3::Z);
                    *topdown_cam.1 = Projection::Orthographic(OrthographicProjection {
                        near: topdown_default.near,
                        far: topdown_default.far,
                        scale: topdown_default.scale,
                        ..default()
                    });
                }
                if ui.button("Reset Settings").clicked() {
                    p.settings.camera = CameraSettings::default();
                }
            });
            ui.collapsing("Fly Camera", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Look Sensitivity")
                        .on_hover_text("How sensitive the camera rotation is to mouse movements");
                    ui.add(
                        egui::DragValue::new(&mut p.settings.camera.fly.look_sensitivity).speed(0.1),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Speed").on_hover_text("How fast the camera moves");
                    ui.add(egui::DragValue::new(&mut p.settings.camera.fly.speed).speed(0.1));
                });
                ui.horizontal(|ui| {
                    ui.label("Speed Multiplier").on_hover_text(
                        "How much faster the camera moves when holding the speed boost button",
                    );
                    ui.add(egui::DragValue::new(&mut p.settings.camera.fly.speed_boost).speed(0.1));
                });
                ui.checkbox(&mut p.settings.camera.fly.hold_mouse_to_move, "Hold Mouse To Move")
                    .on_hover_text("Whether or not the mouse button needs to be pressed in order to move the camera");

                ui.horizontal(|ui| {
                    ui.label("Mouse Button").on_hover_text("The mouse button that needs to be pressed to move the camera");
                    egui::ComboBox::from_id_source("Mouse Button")
                    .selected_text(format!("{:?}", p.settings.camera.fly.key_bindings.mouse_button))
                    .width(60.)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut p.settings.camera.fly.key_bindings.mouse_button, MouseButton::Left, "Left");
                        ui.selectable_value(&mut p.settings.camera.fly.key_bindings.mouse_button, MouseButton::Middle, "Middle");
                        ui.selectable_value(&mut p.settings.camera.fly.key_bindings.mouse_button, MouseButton::Right, "Right");
                    });
                });
            });
            ui.collapsing("Orbit Camera", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Rotate Sensitivity")
                        .on_hover_text("How sensitive the camera rotation is to mouse movements");
                    ui.add(
                        egui::DragValue::new(&mut p.settings.camera.orbit.rotate_sensitivity)
                            .speed(0.1),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Pan Sensitivity:")
                        .on_hover_text("How sensitive the camera panning is to mouse movements");
                    ui.add(
                        egui::DragValue::new(&mut p.settings.camera.orbit.pan_sensitivity).speed(0.1),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label("Scroll Sensitivity")
                        .on_hover_text("How sensitive the camera zoom is to scrolling");
                    ui.add(
                        egui::DragValue::new(&mut p.settings.camera.orbit.scroll_sensitivity)
                            .speed(0.1),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label("Mouse Button").on_hover_text("The mouse button that needs to be pressed to move the camera");
                    egui::ComboBox::from_id_source("Mouse Button")
                    .selected_text(format!("{:?}", p.settings.camera.orbit.key_bindings.mouse_button))
                    .width(60.)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut p.settings.camera.orbit.key_bindings.mouse_button, MouseButton::Left, "Left");
                        ui.selectable_value(&mut p.settings.camera.orbit.key_bindings.mouse_button, MouseButton::Middle, "Middle");
                        ui.selectable_value(&mut p.settings.camera.orbit.key_bindings.mouse_button, MouseButton::Right, "Right");
                    });
                });
            });
            ui.collapsing("Top Down Camera", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Move Sensitivity")
                        .on_hover_text("How sensitive the camera movement is to mouse movements");
                    ui.add(
                        egui::DragValue::new(&mut p.settings.camera.top_down.move_sensitivity)
                            .speed(0.1),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Scroll Sensitivity")
                        .on_hover_text("How sensitive the camera zoom is to scrolling");
                    ui.add(
                        egui::DragValue::new(&mut p.settings.camera.top_down.scroll_sensitivity)
                            .speed(0.1),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Mouse Button").on_hover_text("The mouse button that needs to be pressed to move the camera");
                    egui::ComboBox::from_id_source("Mouse Button")
                    .selected_text(format!("{:?}", p.settings.camera.top_down.key_bindings.mouse_button))
                    .width(60.)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut p.settings.camera.top_down.key_bindings.mouse_button, MouseButton::Left, "Left");
                        ui.selectable_value(&mut p.settings.camera.top_down.key_bindings.mouse_button, MouseButton::Middle, "Middle");
                        ui.selectable_value(&mut p.settings.camera.top_down.key_bindings.mouse_button, MouseButton::Right, "Right");
                    });
                });
            });
        });

    ui.horizontal(|ui| {
        if ui.button("Export Settings").clicked() {
            export_settings_file_dialog(&mut p.app_state);
        }

        if ui.button("Import Settings").clicked() {
            import_settings_file_dialog(&mut p.app_state);
        }
    });
    ui.horizontal(|ui| {
        if ui.button("Reset Tab Layout").clicked() {
            p.settings.reset_tree = true;
        }
        if ui.button("Reset All Settings").clicked() {
            *p.settings = AppSettings::default();
            p.settings.reset_tree = true;
        }
    });
}
