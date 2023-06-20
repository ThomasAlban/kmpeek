use crate::{
    camera::{
        CameraMode, CameraSettings, FlyCam, FlySettings, OrbitCam, OrbitSettings, TopDownCam,
        TopDownSettings,
    },
    kcl_file::*,
};
use bevy::{
    prelude::*,
    render::camera::Viewport,
    tasks::{AsyncComputeTaskPool, Task},
    window::PrimaryWindow,
};
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use futures_lite::future;
use rfd::FileDialog;
use std::path::PathBuf;

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(EguiPlugin)
            .init_resource::<AppState>()
            .add_event::<FileSelected>()
            .add_system(update_ui)
            .add_system(file_dialogue);
    }
}

#[derive(Resource)]
pub struct AppState {
    pub customise_kcl_open: bool,
    pub camera_settings_open: bool,

    pub show_walls: bool,
    pub show_invisible_walls: bool,
    pub show_death_barriers: bool,
    pub show_effects_triggers: bool,

    pub lap_count_buf: String,
    pub speed_mod_buf: String,

    pub look_sensitivity_buf: String,
    pub speed_buf: String,
    pub speed_boost_buf: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            customise_kcl_open: false,
            camera_settings_open: false,

            show_walls: true,
            show_invisible_walls: true,
            show_death_barriers: true,
            show_effects_triggers: true,

            lap_count_buf: String::from("3"),
            speed_mod_buf: String::from("0.0"),

            look_sensitivity_buf: String::from("1.0"),
            speed_buf: String::from("1.0"),
            speed_boost_buf: String::from("3.0"),
        }
    }
}

// this component stores the selected file task - when the select file window is currently open
#[derive(Component)]
pub struct SelectedFileTask(Task<Option<PathBuf>>);
// this event is triggered when a file is chosen
pub struct FileSelected(pub PathBuf);
// this system polls to see if we have picked a file, and if so sends out an event with the path
pub fn file_dialogue(
    mut commands: Commands,
    mut tasks: Query<(Entity, &mut SelectedFileTask)>,
    mut ev_file_selected: EventWriter<FileSelected>,
) {
    for (entity, mut selected_file) in tasks.iter_mut() {
        if let Some(result) = future::block_on(future::poll_once(&mut selected_file.0)) {
            if let Some(path) = result {
                // send a file selected event
                ev_file_selected.send(FileSelected(path));
            }
            // whatever happens, remove the SelectedFileTask - the user may have cancelled the window
            commands.entity(entity).remove::<SelectedFileTask>();
        }
    }
}

fn open_file(mut commands: Commands) {
    // open a file dialogue on a seperate thread
    let task = AsyncComputeTaskPool::get()
        .spawn(async move { FileDialog::new().add_filter("KCL", &["kcl"]).pick_file() });
    commands.spawn(SelectedFileTask(task));
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn update_ui(
    mut contexts: EguiContexts,
    mut kcl: ResMut<Kcl>,
    mut app_state: ResMut<AppState>,
    mut camera_settings: ResMut<CameraSettings>,
    commands: Commands,
    window: Query<&Window, With<PrimaryWindow>>,

    mut fly_cam: Query<
        (&mut Camera, &mut Transform),
        (With<FlyCam>, Without<OrbitCam>, Without<TopDownCam>),
    >,
    mut orbit_cam: Query<
        (&mut Camera, &mut Transform),
        (Without<FlyCam>, With<OrbitCam>, Without<TopDownCam>),
    >,
    mut topdown_cam: Query<
        (&mut Camera, &mut Transform, &mut Projection),
        (Without<FlyCam>, Without<OrbitCam>, With<TopDownCam>),
    >,
) {
    let ctx = contexts.ctx_mut();
    let (mut fly_cam, mut fly_cam_transform) = fly_cam
        .get_single_mut()
        .expect("Could not get single fly cam");
    let (mut orbit_cam, mut orbit_cam_transform) = orbit_cam
        .get_single_mut()
        .expect("Could not get single orbit cam");
    let (mut topdown_cam, mut topdown_cam_transform, mut topdown_cam_projection) = topdown_cam
        .get_single_mut()
        .expect("Could not get single topdown cam");

    let window = window
        .get_single()
        .expect("Primary window not found for update ui");

    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            let sc_btn = if cfg!(target_os = "macos") {
                "Cmd"
            } else {
                "Ctrl"
            };
            ui.menu_button("File", |ui| {
                if ui
                    .add(egui::Button::new("Open").shortcut_text(format!("{sc_btn}+O")))
                    .clicked()
                {
                    open_file(commands);
                }
            });
            ui.menu_button("Edit", |ui| {
                if ui
                    .add(egui::Button::new("Undo").shortcut_text(format!("{sc_btn}+Z")))
                    .clicked()
                {
                    // ...
                }
                if ui
                    .add(egui::Button::new("Redo").shortcut_text(format!("{sc_btn}+Shift+Z")))
                    .clicked()
                {
                    // ...
                }
            });
        });
    });

    let mut customise_kcl_open = app_state.customise_kcl_open;
    egui::Window::new("Customise Collision Model")
        .open(&mut customise_kcl_open)
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
    if customise_kcl_open != app_state.customise_kcl_open {
        app_state.customise_kcl_open = customise_kcl_open;
    }

    let mut camera_settings_open = app_state.camera_settings_open;
    egui::Window::new("Camera Settings")
        .open(&mut camera_settings_open)
        .collapsible(false)
        .min_width(300.)
        .show(ctx, |ui| {
            if ui.button("Reset Positions").clicked() {
                let fly_default = FlySettings::default();
                let orbit_default = OrbitSettings::default();
                let topdown_default = TopDownSettings::default();
                *fly_cam_transform = Transform::from_translation(fly_default.start_pos)
                    .looking_at(Vec3::ZERO, Vec3::Y);
                *orbit_cam_transform = Transform::from_translation(orbit_default.start_pos)
                    .looking_at(Vec3::ZERO, Vec3::Y);
                *topdown_cam_transform = Transform::from_translation(topdown_default.start_pos)
                    .looking_at(Vec3::ZERO, Vec3::Z);
                *topdown_cam_projection = Projection::Orthographic(OrthographicProjection {
                    near: topdown_default.near,
                    far: topdown_default.far,
                    scale: topdown_default.scale,
                    ..default()
                });
            }
            if ui.button("Reset Settings").clicked() {
                *camera_settings = CameraSettings::default();
            }
            ui.collapsing("Fly Camera", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Look Sensitivity")
                        .on_hover_text("How sensitive the camera rotation is to mouse movements");
                    ui.add(
                        egui::DragValue::new(&mut camera_settings.fly.look_sensitivity).speed(0.1),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Speed").on_hover_text("How fast the camera moves");
                    ui.add(egui::DragValue::new(&mut camera_settings.fly.speed).speed(0.1));
                });
                ui.horizontal(|ui| {
                    ui.label("Speed Boost").on_hover_text(
                        "How much faster the camera moves when holding the speed boost button",
                    );
                    ui.add(egui::DragValue::new(&mut camera_settings.fly.speed_boost).speed(0.1));
                });
                ui.checkbox(&mut camera_settings.fly.hold_mouse_to_move, "Hold Mouse To Move")
                    .on_hover_text("Whether or not the mouse button needs to be pressed in order to move the camera");
            });
            ui.collapsing("Orbit Camera", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Rotate Sensitivity")
                        .on_hover_text("How sensitive the camera rotation is to mouse movements");
                    ui.add(
                        egui::DragValue::new(&mut camera_settings.orbit.rotate_sensitivity)
                            .speed(0.1),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Pan Sensitivity:")
                        .on_hover_text("How sensitive the camera panning is to mouse movements");
                    ui.add(
                        egui::DragValue::new(&mut camera_settings.orbit.pan_sensitivity).speed(0.1),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label("Scroll Sensitivity")
                        .on_hover_text("How sensitive the camera zoom is to scrolling");
                    ui.add(
                        egui::DragValue::new(&mut camera_settings.orbit.scroll_sensitivity)
                            .speed(0.1),
                    );
                });
            });
            ui.collapsing("Top Down Camera", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Move Sensitivity")
                        .on_hover_text("How sensitive the camera movement is to mouse movements");
                    ui.add(
                        egui::DragValue::new(&mut camera_settings.top_down.move_sensitivity)
                            .speed(0.1),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Scroll Sensitivity")
                        .on_hover_text("How sensitive the camera zoom is to scrolling");
                    ui.add(
                        egui::DragValue::new(&mut camera_settings.top_down.scroll_sensitivity)
                            .speed(0.1),
                    );
                });
            });
        });
    if camera_settings_open != app_state.camera_settings_open {
        app_state.camera_settings_open = camera_settings_open;
    }

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
                            let mut mode = camera_settings.mode;
                            ui.selectable_value(&mut mode, CameraMode::Fly, "Fly");
                            ui.selectable_value(&mut mode, CameraMode::Orbit, "Orbit");
                            ui.selectable_value(&mut mode, CameraMode::TopDown, "Top Down");
                            if camera_settings.mode != mode {
                                camera_settings.mode = mode;
                            }
                        });
                        if ui.button("Camera Settings...").clicked() {
                            app_state.camera_settings_open = true;
                        }
                    });
                });

            ui.separator();
        });

    // this resizes the viewport to the remaining space after we're done drawing all the UI stuff
    // has to check if the width and height are > 0 otherwise it will crash
    if ctx.available_rect().width() as u32 > 0 && ctx.available_rect().height() as u32 > 0 {
        let viewport = Viewport {
            physical_size: UVec2 {
                x: (ctx.available_rect().width() * window.scale_factor() as f32) as u32,
                y: (ctx.available_rect().height() * window.scale_factor() as f32) as u32,
            },
            physical_position: UVec2 {
                x: (ctx.available_rect().min.x * window.scale_factor() as f32) as u32,
                y: (ctx.available_rect().min.y * window.scale_factor() as f32) as u32,
            },
            ..default()
        };

        (fly_cam.viewport, orbit_cam.viewport, topdown_cam.viewport) = (
            Some(viewport.clone()),
            Some(viewport.clone()),
            Some(viewport),
        );
    }
}
