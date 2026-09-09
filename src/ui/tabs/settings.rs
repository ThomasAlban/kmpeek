use crate::{
    ui::{file_dialog::FileDialogManager, settings::AppSettings},
    util::kcl_file::KclFlag,
    viewer::{
        camera::{CameraSettings, FlyCam, FlySettings, OrbitCam, OrbitSettings, TopDownCam, TopDownSettings},
        kcl_model::KclModelUpdated,
        kmp::settings::{MAX_GIZMO_LINE_WIDTH, MAX_GIZMO_SIZE, MIN_GIZMO_LINE_WIDTH, MIN_GIZMO_SIZE},
    },
};
use bevy::{ecs::system::SystemState, prelude::*};
use bevy_egui::egui::{self, Ui};
use bevy_pkv::PkvStore;
use strum::IntoEnumIterator;

pub fn show_settings_tab(ui: &mut Ui, world: &mut World) {
    let mut ss = SystemState::<(
        ResMut<AppSettings>,
        ResMut<PkvStore>,
        Res<ButtonInput<KeyCode>>,
        FileDialogManager,
        Query<&mut Transform, (With<FlyCam>, Without<OrbitCam>, Without<TopDownCam>)>,
        Query<&mut Transform, (Without<FlyCam>, With<OrbitCam>, Without<TopDownCam>)>,
        Query<(&mut Transform, &'static mut Projection), (Without<FlyCam>, Without<OrbitCam>, With<TopDownCam>)>,
        MessageWriter<KclModelUpdated>,
    )>::new(world);
    let (
        mut settings,
        mut pkv,
        keys,
        mut file_dialog,
        mut q_fly_cam,
        mut q_orbit_cam,
        mut q_topdown_cam,
        mut ev_kcl_model_updated,
    ) = ss.get_mut(world);

    let (Ok(mut fly_cam), Ok(mut orbit_cam), Ok(mut topdown_cam)) = (
        q_fly_cam.single_mut(),
        q_orbit_cam.single_mut(),
        q_topdown_cam.single_mut(),
    ) else {
        return;
    };

    egui::CollapsingHeader::new("KMP Viewer")
        .default_open(true)
        .show(ui, |ui| {
            ui.add(
                egui::Slider::new(&mut settings.kmp_model.point_scale, 0.01..=2.)
                    .text("Point Scale"),
            );
            ui.add(
                egui::Slider::new(
                    &mut settings.kmp_model.gizmo_size,
                    MIN_GIZMO_SIZE..=MAX_GIZMO_SIZE,
                )
                .text("Transform Gizmo Size")
                .suffix(" px"),
            );
            ui.add(
                egui::Slider::new(
                    &mut settings.kmp_model.gizmo_line_width,
                    MIN_GIZMO_LINE_WIDTH..=MAX_GIZMO_LINE_WIDTH,
                )
                .text("Transform Gizmo Line Width")
                .suffix(" px")
                .step_by(0.5),
            );
            ui.checkbox(
                &mut settings.open_course_kcl_in_dir,
                "Auto open course.kcl",
            ).on_hover_text_at_pointer("If enabled, when opening a KMP file, if there is a 'course.kcl' file in the same directory, it will also be opened");

        });

    egui::CollapsingHeader::new("Collision Model")
        .default_open(true)
        .show(ui, |ui| {
            let kcl_model_settings_before = settings.kcl_model.clone();
            ui.checkbox(&mut settings.kcl_model.backface_culling, "Backface Culling")
                .on_hover_text_at_pointer("Whether or not the back faces of the collision model are shown");

            let visible = &mut settings.kcl_model.visible;

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
            let show_death_barriers_changed = ui.checkbox(&mut show_death_barriers, "Show Death Barriers").changed();
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
                        settings.kcl_model.visible = [true; 32];
                    }
                    if ui.button("Uncheck All").clicked() {
                        settings.kcl_model.visible = [false; 32];
                    }
                    if ui.button("Reset").clicked() {
                        settings.kcl_model = default();
                    }
                });
                // show colour edit and visibility toggle for each kcl flag variant
                for (i, kcl_flag) in KclFlag::iter().enumerate() {
                    ui.horizontal(|ui| {
                        let mut color = settings.kcl_model.color[i].to_srgba().to_f32_array();
                        ui.color_edit_button_rgba_unmultiplied(&mut color);
                        settings.kcl_model.color[i] = Srgba::from_f32_array(color).into();
                        ui.checkbox(&mut settings.kcl_model.visible[i], kcl_flag.to_string());
                    });
                }
            });
            if settings.kcl_model != kcl_model_settings_before {
                ev_kcl_model_updated.write_default();
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
                *topdown_cam.0 = Transform::from_translation(topdown_default.start_pos).looking_at(Vec3::ZERO, Vec3::Z);
                *topdown_cam.1 = Projection::Orthographic(OrthographicProjection {
                    near: topdown_default.near,
                    far: topdown_default.far,
                    scale: topdown_default.scale,
                    ..OrthographicProjection::default_3d()
                });
            }
            if ui.button("Reset Settings").clicked() {
                settings.camera = CameraSettings::default();
            }
        });
        ui.collapsing("Fly Camera", |ui| {
            ui.horizontal(|ui| {
                ui.label("Look Sensitivity")
                    .on_hover_text_at_pointer("How sensitive the camera rotation is to mouse movements");
                ui.add(egui::DragValue::new(&mut settings.camera.fly.look_sensitivity).speed(0.1));
            });
            ui.horizontal(|ui| {
                ui.label("Speed").on_hover_text_at_pointer("How fast the camera moves");
                ui.add(egui::DragValue::new(&mut settings.camera.fly.speed).speed(0.1));
            });
            ui.horizontal(|ui| {
                ui.label("Speed Multiplier")
                    .on_hover_text_at_pointer("How much faster the camera moves when holding the speed boost button");
                ui.add(egui::DragValue::new(&mut settings.camera.fly.speed_boost).speed(0.1));
            });
            ui.checkbox(&mut settings.camera.fly.hold_mouse_to_move, "Hold Mouse To Move")
                .on_hover_text_at_pointer(
                    "Whether or not the mouse button needs to be pressed in order to move the camera",
                );
            mouse_button_row(
                ui,
                "fly_mouse_button",
                "Mouse Button",
                "The mouse button that needs to be pressed to move the camera",
                &mut settings.camera.fly.key_bindings.mouse_button,
            );
            ui.separator();
            ui.label(egui::RichText::new("Key Bindings").strong())
                .on_hover_text("Add any number of alternative keys. Click a key to remove it.");
            key_binding_row(
                ui,
                &keys,
                CameraKeyBinding::FlyForward,
                "Move Forward",
                &mut settings.camera.fly.key_bindings.move_forward,
            );
            key_binding_row(
                ui,
                &keys,
                CameraKeyBinding::FlyBackward,
                "Move Backward",
                &mut settings.camera.fly.key_bindings.move_backward,
            );
            key_binding_row(
                ui,
                &keys,
                CameraKeyBinding::FlyLeft,
                "Move Left",
                &mut settings.camera.fly.key_bindings.move_left,
            );
            key_binding_row(
                ui,
                &keys,
                CameraKeyBinding::FlyRight,
                "Move Right",
                &mut settings.camera.fly.key_bindings.move_right,
            );
            key_binding_row(
                ui,
                &keys,
                CameraKeyBinding::FlyAscend,
                "Ascend",
                &mut settings.camera.fly.key_bindings.move_ascend,
            );
            key_binding_row(
                ui,
                &keys,
                CameraKeyBinding::FlyDescend,
                "Descend",
                &mut settings.camera.fly.key_bindings.move_descend,
            );
            key_binding_row(
                ui,
                &keys,
                CameraKeyBinding::FlySpeedBoost,
                "Speed Boost",
                &mut settings.camera.fly.key_bindings.speed_boost,
            );
        });
        ui.collapsing("Orbit Camera", |ui| {
            ui.horizontal(|ui| {
                ui.label("Rotate Sensitivity")
                    .on_hover_text_at_pointer("How sensitive the camera rotation is to mouse movements");
                ui.add(egui::DragValue::new(&mut settings.camera.orbit.rotate_sensitivity).speed(0.1));
            });
            ui.horizontal(|ui| {
                ui.label("Pan Sensitivity:")
                    .on_hover_text_at_pointer("How sensitive the camera panning is to mouse movements");
                ui.add(egui::DragValue::new(&mut settings.camera.orbit.pan_sensitivity).speed(0.1));
            });
            ui.horizontal(|ui| {
                ui.label("Scroll Sensitivity")
                    .on_hover_text_at_pointer("How sensitive the camera zoom is to scrolling");
                ui.add(egui::DragValue::new(&mut settings.camera.orbit.scroll_sensitivity).speed(0.1));
            });
            mouse_button_row(
                ui,
                "orbit_mouse_button",
                "Mouse Button",
                "The mouse button used to rotate or pan the camera",
                &mut settings.camera.orbit.key_bindings.mouse_button,
            );
            ui.separator();
            ui.label(egui::RichText::new("Key Bindings").strong())
                .on_hover_text("Add any number of alternative keys. Click a key to remove it.");
            key_binding_row(
                ui,
                &keys,
                CameraKeyBinding::OrbitPan,
                "Pan Modifier",
                &mut settings.camera.orbit.key_bindings.pan,
            );
        });
        ui.collapsing("Top Down Camera", |ui| {
            ui.horizontal(|ui| {
                ui.label("Move Sensitivity")
                    .on_hover_text_at_pointer("How sensitive the camera movement is to mouse movements");
                ui.add(egui::DragValue::new(&mut settings.camera.top_down.move_sensitivity).speed(0.1));
            });
            ui.horizontal(|ui| {
                ui.label("Scroll Sensitivity")
                    .on_hover_text_at_pointer("How sensitive the camera zoom is to scrolling");
                ui.add(egui::DragValue::new(&mut settings.camera.top_down.scroll_sensitivity).speed(0.1));
            });
            mouse_button_row(
                ui,
                "top_down_mouse_button",
                "Mouse Button",
                "The mouse button used to move the camera",
                &mut settings.camera.top_down.key_bindings.mouse_button,
            );
        });
    });

    ui.horizontal(|ui| {
        if ui.button("Export Settings").clicked() {
            file_dialog.export_settings();
        }

        if ui.button("Import Settings").clicked() {
            file_dialog.import_settings();
        }
    });
    ui.horizontal(|ui| {
        if ui.button("Save Settings").clicked() {
            if let Err(error) = pkv.set("settings", settings.as_ref()) {
                error!("could not save application settings: {error}");
            }
        }
        if ui.button("Reset Settings").clicked() {
            *settings = AppSettings::default();
            if let Err(error) = pkv.set("settings", settings.as_ref()) {
                error!("could not save reset application settings: {error}");
            }
        }
    });

    ss.apply(world);
}

#[derive(Clone, Copy, PartialEq)]
enum CameraKeyBinding {
    FlyForward,
    FlyBackward,
    FlyLeft,
    FlyRight,
    FlyAscend,
    FlyDescend,
    FlySpeedBoost,
    OrbitPan,
}

fn key_binding_row(
    ui: &mut Ui,
    keys: &ButtonInput<KeyCode>,
    binding: CameraKeyBinding,
    label: &str,
    key_codes: &mut Vec<KeyCode>,
) {
    let capture_id = egui::Id::new("camera_key_binding_capture");
    let mut is_capturing = ui
        .ctx()
        .data(|data| data.get_temp::<CameraKeyBinding>(capture_id) == Some(binding));

    if is_capturing {
        if let Some(key_code) = keys.get_just_pressed().next().copied() {
            if key_code != KeyCode::Escape && !key_codes.contains(&key_code) {
                key_codes.push(key_code);
            }
            ui.ctx().data_mut(|data| data.remove::<CameraKeyBinding>(capture_id));
            is_capturing = false;
        }
    }

    let mut remove_index = None;
    ui.horizontal_wrapped(|ui| {
        ui.add_sized([110.0, ui.spacing().interact_size.y], egui::Label::new(label));

        if key_codes.is_empty() {
            ui.label(egui::RichText::new("Unbound").italics().weak());
        } else {
            for (index, key_code) in key_codes.iter().enumerate() {
                if ui
                    .small_button(format!("{}  ×", key_code_label(*key_code)))
                    .on_hover_text("Remove this key")
                    .clicked()
                {
                    remove_index = Some(index);
                }
            }
        }

        if is_capturing {
            if ui
                .button("Press a key…")
                .on_hover_text("Press Escape or click to cancel")
                .clicked()
            {
                ui.ctx().data_mut(|data| data.remove::<CameraKeyBinding>(capture_id));
            }
        } else if ui.small_button("+ Add key").clicked() {
            ui.ctx().data_mut(|data| data.insert_temp(capture_id, binding));
        }
    });

    if let Some(index) = remove_index {
        key_codes.remove(index);
    }
}

fn mouse_button_row(ui: &mut Ui, id_salt: &'static str, label: &str, hover_text: &str, mouse_button: &mut MouseButton) {
    ui.horizontal(|ui| {
        ui.label(label).on_hover_text_at_pointer(hover_text);
        egui::ComboBox::from_id_salt(id_salt)
            .selected_text(mouse_button_label(*mouse_button))
            .width(80.0)
            .show_ui(ui, |ui| {
                for button in [
                    MouseButton::Left,
                    MouseButton::Middle,
                    MouseButton::Right,
                    MouseButton::Back,
                    MouseButton::Forward,
                ] {
                    ui.selectable_value(mouse_button, button, mouse_button_label(button));
                }
            });
    });
}

fn key_code_label(key_code: KeyCode) -> String {
    match key_code {
        KeyCode::ShiftLeft => "Left Shift".into(),
        KeyCode::ShiftRight => "Right Shift".into(),
        KeyCode::ControlLeft => "Left Ctrl".into(),
        KeyCode::ControlRight => "Right Ctrl".into(),
        KeyCode::AltLeft => "Left Alt".into(),
        KeyCode::AltRight => "Right Alt".into(),
        KeyCode::SuperLeft => "Left Super".into(),
        KeyCode::SuperRight => "Right Super".into(),
        KeyCode::ArrowUp => "Up Arrow".into(),
        KeyCode::ArrowDown => "Down Arrow".into(),
        KeyCode::ArrowLeft => "Left Arrow".into(),
        KeyCode::ArrowRight => "Right Arrow".into(),
        KeyCode::PageUp => "Page Up".into(),
        KeyCode::PageDown => "Page Down".into(),
        key_code => {
            let debug_name = format!("{key_code:?}");
            debug_name
                .strip_prefix("Key")
                .or_else(|| debug_name.strip_prefix("Digit"))
                .unwrap_or(&debug_name)
                .to_owned()
        }
    }
}

fn mouse_button_label(mouse_button: MouseButton) -> String {
    match mouse_button {
        MouseButton::Left => "Left".into(),
        MouseButton::Right => "Right".into(),
        MouseButton::Middle => "Middle".into(),
        MouseButton::Back => "Back".into(),
        MouseButton::Forward => "Forward".into(),
        MouseButton::Other(number) => format!("Button {number}"),
    }
}
