use super::UiSubSection;
use crate::{
    ui::{file_dialog::FileDialogManager, settings::AppSettings},
    util::kcl_file::KclFlag,
    viewer::{
        camera::{CameraSettings, FlyCam, FlySettings, OrbitCam, OrbitSettings, TopDownCam, TopDownSettings},
        kcl_model::KclModelUpdated,
    },
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui;
use bevy_pkv::PkvStore;
use strum::IntoEnumIterator;

#[derive(SystemParam)]
pub struct ShowSettingsTab<'w, 's> {
    settings: ResMut<'w, AppSettings>,
    ev_kcl_model_updated: EventWriter<'w, KclModelUpdated>,
    q_cams: (
        // fly cam
        Query<'w, 's, &'static mut Transform, (With<FlyCam>, Without<OrbitCam>, Without<TopDownCam>)>,
        // orbit cam
        Query<'w, 's, &'static mut Transform, (Without<FlyCam>, With<OrbitCam>, Without<TopDownCam>)>,
        // topdown cam
        Query<
            'w,
            's,
            (&'static mut Transform, &'static mut Projection),
            (Without<FlyCam>, Without<OrbitCam>, With<TopDownCam>),
        >,
    ),
    pkv: ResMut<'w, PkvStore>,
    file_dialog: FileDialogManager<'w>,
}
impl UiSubSection for ShowSettingsTab<'_, '_> {
    fn show(&mut self, ui: &mut egui::Ui) {
        let mut fly_cam = self.q_cams.0.get_single_mut().unwrap();
        let mut orbit_cam = self.q_cams.1.get_single_mut().unwrap();
        let mut topdown_cam = self.q_cams.2.get_single_mut().unwrap();

        egui::CollapsingHeader::new("KMP Viewer")
            .default_open(true)
            .show(ui, |ui| {
                ui.add(
                    egui::Slider::new(&mut self.settings.kmp_model.point_scale, 0.01..=2.)
                        .text("Point Scale"),
                );
                //ui.checkbox(&mut self.settings.kmp_model.normalize, "Normalize points");
                ui.checkbox(
                    &mut self.settings.open_course_kcl_in_dir,
                    "Auto open course.kcl",
                ).on_hover_text("If enabled, when opening a KMP file, if there is a 'course.kcl' file in the same directory, it will also be opened");

            });

        egui::CollapsingHeader::new("Collision Model")
            .default_open(true)
            .show(ui, |ui| {
                ui.checkbox(&mut self.settings.kcl_model.backface_culling, "Backface Culling")
                    .on_hover_text("Whether or not the back faces of the collision model are shown");
                let kcl_model_settings_before = self.settings.kcl_model.clone();

                let visible = &mut self.settings.kcl_model.visible;

                use KclFlag::*;

                let mut show_walls = visible[Wall1 as usize] && visible[Wall2 as usize] && visible[WeakWall as usize];
                let mut show_invis_walls = visible[InvisibleWall1 as usize] && visible[InvisibleWall2 as usize];
                let mut show_death_barriers = visible[SolidFall as usize] && visible[FallBoundary as usize];
                let mut show_effects_triggers = visible[ItemStateModifier as usize]
                    && visible[EffectTrigger as usize]
                    && visible[SoundTrigger as usize]
                    && visible[KclFlag::CannonTrigger as usize];

                let show_walls_changed = ui.checkbox(&mut show_walls, "Show Walls").changed();
                let show_invis_walls_changed = ui.checkbox(&mut show_invis_walls, "Show Invisible Walls").changed();
                let show_death_barriers_changed =
                    ui.checkbox(&mut show_death_barriers, "Show Death Barriers").changed();
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
                    [visible[InvisibleWall1 as usize], visible[InvisibleWall2 as usize]] = [show_invis_walls; 2];
                }
                if show_death_barriers_changed {
                    [visible[SolidFall as usize], visible[FallBoundary as usize]] = [show_death_barriers; 2];
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
                            self.settings.kcl_model.visible = [true; 32];
                        }
                        if ui.button("Uncheck All").clicked() {
                            self.settings.kcl_model.visible = [false; 32];
                        }
                        if ui.button("Reset").clicked() {
                            self.settings.kcl_model = default();
                        }
                    });
                    // show colour edit and visibility toggle for each kcl flag variant
                    for (i, kcl_flag) in KclFlag::iter().enumerate() {
                        ui.horizontal(|ui| {
                            let mut color = self.settings.kcl_model.color[i].to_srgba().to_f32_array();
                            ui.color_edit_button_rgba_unmultiplied(&mut color);
                            self.settings.kcl_model.color[i] = Srgba::from_f32_array(color).into();
                            ui.checkbox(&mut self.settings.kcl_model.visible[i], kcl_flag.to_string());
                        });
                    }
                });
                if self.settings.kcl_model != kcl_model_settings_before {
                    self.ev_kcl_model_updated.send_default();
                }
            });

        egui::CollapsingHeader::new("Camera").default_open(true).show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Reset Positions").clicked() {
                    let fly_default = FlySettings::default();
                    let orbit_default = OrbitSettings::default();
                    let topdown_default = TopDownSettings::default();
                    *fly_cam = Transform::from_translation(fly_default.start_pos).looking_at(Vec3::ZERO, Vec3::Y);
                    *orbit_cam = Transform::from_translation(orbit_default.start_pos).looking_at(Vec3::ZERO, Vec3::Y);
                    *topdown_cam.0 =
                        Transform::from_translation(topdown_default.start_pos).looking_at(Vec3::ZERO, Vec3::Z);
                    *topdown_cam.1 = Projection::Orthographic(OrthographicProjection {
                        near: topdown_default.near,
                        far: topdown_default.far,
                        scale: topdown_default.scale,
                        ..default()
                    });
                }
                if ui.button("Reset Settings").clicked() {
                    self.settings.camera = CameraSettings::default();
                }
            });
            ui.collapsing("Fly Camera", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Look Sensitivity")
                        .on_hover_text("How sensitive the camera rotation is to mouse movements");
                    ui.add(egui::DragValue::new(&mut self.settings.camera.fly.look_sensitivity).speed(0.1));
                });
                ui.horizontal(|ui| {
                    ui.label("Speed").on_hover_text("How fast the camera moves");
                    ui.add(egui::DragValue::new(&mut self.settings.camera.fly.speed).speed(0.1));
                });
                ui.horizontal(|ui| {
                    ui.label("Speed Multiplier")
                        .on_hover_text("How much faster the camera moves when holding the speed boost button");
                    ui.add(egui::DragValue::new(&mut self.settings.camera.fly.speed_boost).speed(0.1));
                });
                ui.checkbox(&mut self.settings.camera.fly.hold_mouse_to_move, "Hold Mouse To Move")
                    .on_hover_text("Whether or not the mouse button needs to be pressed in order to move the camera");
                ui.horizontal(|ui| {
                    ui.label("Mouse Button")
                        .on_hover_text("The mouse button that needs to be pressed to move the camera");
                    egui::ComboBox::from_id_source("Mouse Button")
                        .selected_text(format!("{:?}", self.settings.camera.fly.key_bindings.mouse_button))
                        .width(60.)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.settings.camera.fly.key_bindings.mouse_button,
                                MouseButton::Left,
                                "Left",
                            );
                            ui.selectable_value(
                                &mut self.settings.camera.fly.key_bindings.mouse_button,
                                MouseButton::Middle,
                                "Middle",
                            );
                            ui.selectable_value(
                                &mut self.settings.camera.fly.key_bindings.mouse_button,
                                MouseButton::Right,
                                "Right",
                            );
                        });
                });
            });
            ui.collapsing("Orbit Camera", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Rotate Sensitivity")
                        .on_hover_text("How sensitive the camera rotation is to mouse movements");
                    ui.add(egui::DragValue::new(&mut self.settings.camera.orbit.rotate_sensitivity).speed(0.1));
                });
                ui.horizontal(|ui| {
                    ui.label("Pan Sensitivity:")
                        .on_hover_text("How sensitive the camera panning is to mouse movements");
                    ui.add(egui::DragValue::new(&mut self.settings.camera.orbit.pan_sensitivity).speed(0.1));
                });
                ui.horizontal(|ui| {
                    ui.label("Scroll Sensitivity")
                        .on_hover_text("How sensitive the camera zoom is to scrolling");
                    ui.add(egui::DragValue::new(&mut self.settings.camera.orbit.scroll_sensitivity).speed(0.1));
                });
                ui.horizontal(|ui| {
                    ui.label("Mouse Button")
                        .on_hover_text("The mouse button that needs to be pressed to move the camera");
                    egui::ComboBox::from_id_source("Mouse Button")
                        .selected_text(format!("{:?}", self.settings.camera.orbit.key_bindings.mouse_button))
                        .width(60.)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.settings.camera.orbit.key_bindings.mouse_button,
                                MouseButton::Left,
                                "Left",
                            );
                            ui.selectable_value(
                                &mut self.settings.camera.orbit.key_bindings.mouse_button,
                                MouseButton::Middle,
                                "Middle",
                            );
                            ui.selectable_value(
                                &mut self.settings.camera.orbit.key_bindings.mouse_button,
                                MouseButton::Right,
                                "Right",
                            );
                        });
                });
            });
            ui.collapsing("Top Down Camera", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Move Sensitivity")
                        .on_hover_text("How sensitive the camera movement is to mouse movements");
                    ui.add(egui::DragValue::new(&mut self.settings.camera.top_down.move_sensitivity).speed(0.1));
                });
                ui.horizontal(|ui| {
                    ui.label("Scroll Sensitivity")
                        .on_hover_text("How sensitive the camera zoom is to scrolling");
                    ui.add(egui::DragValue::new(&mut self.settings.camera.top_down.scroll_sensitivity).speed(0.1));
                });
                ui.horizontal(|ui| {
                    ui.label("Mouse Button")
                        .on_hover_text("The mouse button that needs to be pressed to move the camera");
                    egui::ComboBox::from_id_source("Mouse Button")
                        .selected_text(format!("{:?}", self.settings.camera.top_down.key_bindings.mouse_button))
                        .width(60.)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.settings.camera.top_down.key_bindings.mouse_button,
                                MouseButton::Left,
                                "Left",
                            );
                            ui.selectable_value(
                                &mut self.settings.camera.top_down.key_bindings.mouse_button,
                                MouseButton::Middle,
                                "Middle",
                            );
                            ui.selectable_value(
                                &mut self.settings.camera.top_down.key_bindings.mouse_button,
                                MouseButton::Right,
                                "Right",
                            );
                        });
                });
            });
        });

        ui.horizontal(|ui| {
            if ui.button("Export Settings").clicked() {
                self.file_dialog.export_settings();
            }

            if ui.button("Import Settings").clicked() {
                self.file_dialog.import_settings();
            }
        });
        ui.horizontal(|ui| {
            if ui.button("Save Settings").clicked() {
                self.pkv.set("settings", self.settings.as_ref()).unwrap();
            }
            if ui.button("Reset Settings").clicked() {
                *self.settings = AppSettings::default();
                self.pkv.set("settings", self.settings.as_ref()).unwrap();
            }
        });
    }
}
