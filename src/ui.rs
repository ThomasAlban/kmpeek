use crate::{
    camera::{
        CameraMode, CameraModeChanged, CameraSettings, FlyCam, FlySettings, OrbitCam,
        OrbitSettings, TopDownCam, TopDownSettings,
    },
    kcl_file::*,
    kcl_model::KclModelSettings,
    kmp_file::Kmp,
    kmp_model::NormalizeScale,
    undo::UndoStack,
};
use bevy::{
    math::vec2,
    prelude::*,
    render::render_resource::{
        Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    },
    window::PrimaryWindow,
};
use bevy_egui::{
    egui::{self, TextureId},
    EguiContexts, EguiPlugin, EguiUserTextures,
};
use bevy_pkv::PkvStore;
use egui_dock::{DockArea, NodeIndex, Style, Tree};
use egui_file::*;
use serde::{Deserialize, Serialize};
use std::{
    env,
    fs::{read_to_string, File},
    io::Write,
    path::{Path, PathBuf},
};
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter};

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub struct SetupAppStateSet;

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PkvStore::new("ThomasAlban", "KMPeek"))
            .add_plugins(EguiPlugin)
            .add_event::<KmpFileSelected>()
            .add_event::<KclFileSelected>()
            .add_systems(
                Startup,
                // this makes sure all the 'Commands' are completed before moving onto other startup systems
                // so that other startup systems can make use of the Viewport image handle
                (setup_app_state, apply_deferred)
                    .chain()
                    .in_set(SetupAppStateSet),
            )
            .add_systems(Update, update_ui);
    }
}

#[derive(Resource)]
pub struct AppState {
    pub customise_kcl_open: bool,
    pub camera_settings_open: bool,

    pub file_dialog: Option<(FileDialog, DialogType)>,

    pub kmp_file_path: Option<PathBuf>,
    pub mouse_in_viewport: bool,
    pub viewport_rect: Rect,
}

pub enum DialogType {
    OpenKmpKcl,
    ExportSettings,
    ImportSettings,
}

#[derive(Serialize, Deserialize)]
pub struct AppSettings {
    pub camera: CameraSettings,
    pub kcl_model: KclModelSettings,
    pub point_scale: f32,
    pub open_course_kcl_in_directory: bool,
    pub reset_tree: bool,
}
impl Default for AppSettings {
    fn default() -> Self {
        Self {
            camera: CameraSettings::default(),
            kcl_model: KclModelSettings::default(),
            point_scale: 1.,
            open_course_kcl_in_directory: true,
            reset_tree: false,
        }
    }
}

#[derive(Deref, DerefMut, Resource, Serialize, Deserialize)]
pub struct DockTree(Tree<Tab>);
impl Default for DockTree {
    fn default() -> Self {
        let mut tree = Tree::new(vec![Tab::Viewport]);
        tree.split_left(NodeIndex::root(), 0.2, vec![Tab::Edit, Tab::Settings]);
        Self(tree)
    }
}

// stores the image which the camera renders to, so that we can display a viewport inside a tab
#[derive(Deref, Resource)]
pub struct ViewportImage(Handle<Image>);

pub fn setup_app_state(
    mut commands: Commands,
    mut egui_user_textures: ResMut<EguiUserTextures>,
    mut images: ResMut<Assets<Image>>,
    mut pkv: ResMut<PkvStore>,
    mut ev_kmp_file_selected: EventWriter<KmpFileSelected>,
    mut ev_kcl_file_selected: EventWriter<KclFileSelected>,
    mut ev_camera_mode_changed: EventWriter<CameraModeChanged>,
) {
    // this is the texture that will be rendered to
    let image: Image = Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size: Extent3d {
                width: 0,
                height: 0,
                ..default()
            },
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        ..default()
    };

    // create a handle to the image
    let image_handle = images.add(image);
    egui_user_textures.add_image(image_handle.clone());

    commands.insert_resource(ViewportImage(image_handle));

    // set app settings to defaults if they do not already exist
    if pkv.get::<AppSettings>("settings").is_err() {
        pkv.set("settings", &AppSettings::default())
            .expect("failed to store app settings");
    }

    if pkv.get::<DockTree>("tree").is_err() {
        pkv.set("tree", &DockTree::default())
            .expect("failed to store dock tree");
    }

    let mut app_state = AppState {
        customise_kcl_open: false,
        camera_settings_open: false,

        file_dialog: None,

        kmp_file_path: None,
        mouse_in_viewport: false,
        viewport_rect: Rect::from_corners(Vec2::ZERO, Vec2::ZERO),
    };

    // get the settings for below so we know settings.open_course_kcl_in_directory
    let settings = pkv.get::<AppSettings>("settings").unwrap();

    ev_camera_mode_changed.send(CameraModeChanged(settings.camera.mode));

    // if there is a command line arg of a path to a kmp or kcl, open it
    let args: Vec<String> = env::args().collect();
    let mut kmp_file_path: Option<PathBuf> = None;
    if let Some(arg) = args.get(1) {
        let path = Path::new(arg);
        if path.is_file() {
            if let Some(file_ext) = path.extension() {
                if file_ext == "kmp" {
                    kmp_file_path = Some(path.into());
                    ev_kmp_file_selected.send(KmpFileSelected(path.into()));
                    if settings.open_course_kcl_in_directory {
                        let mut course_kcl_path = path.to_owned();
                        course_kcl_path.set_file_name("course.kcl");
                        if course_kcl_path.exists() {
                            ev_kcl_file_selected.send(KclFileSelected(course_kcl_path));
                        }
                    }
                } else if file_ext == "kcl" {
                    ev_kcl_file_selected.send(KclFileSelected(path.into()));
                }
            }
        }
    }
    app_state.kmp_file_path = kmp_file_path;

    let camera_settings = CameraSettings::default();

    commands.insert_resource(app_state);
    commands.insert_resource(camera_settings);
}

#[derive(Event)]
pub struct KmpFileSelected(pub PathBuf);

#[derive(Event)]
pub struct KclFileSelected(pub PathBuf);

#[derive(Display, PartialEq, EnumIter, Serialize, Deserialize, Clone, Copy)]
pub enum Tab {
    Viewport,
    Edit,
    Settings,
}

// this tells egui how to render each tab
struct TabViewer<'a, 'b> {
    // add into here any data that needs to be passed into any tabs
    viewport_image: &'a mut Image,
    viewport_tex_id: TextureId,
    window: &'a Window,
    kmp: Option<ResMut<'a, Kmp>>,

    app_state: &'a mut AppState,
    settings: &'a mut AppSettings,

    normalize: Vec<&'a mut NormalizeScale>,
    // pointer: &'a mut PointerLocation,
    fly_cam: &'a mut Transform,
    orbit_cam: &'a mut Transform,
    topdown_cam: (&'a mut Transform, &'a mut Projection),
    ev_camera_mode_changed: &'a mut EventWriter<'b, CameraModeChanged>,
}
impl egui_dock::TabViewer for TabViewer<'_, '_> {
    // each tab will be distinguished by a string - its name
    type Tab = Tab;
    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        // we can do different things inside the tab depending on its name
        match tab {
            Tab::Viewport => {
                let viewport_size = vec2(ui.available_width(), ui.available_height());
                // resize the viewport if needed
                if self.viewport_image.size().as_uvec2()
                    != (viewport_size.as_uvec2() * self.window.scale_factor() as u32)
                {
                    let size = Extent3d {
                        width: viewport_size.x as u32 * self.window.scale_factor() as u32,
                        height: viewport_size.y as u32 * self.window.scale_factor() as u32,
                        ..default()
                    };
                    self.viewport_image.resize(size);
                }
                // show the viewport image
                ui.image(self.viewport_tex_id, viewport_size.to_array());
                self.app_state.mouse_in_viewport = ui.ui_contains_pointer();
                let rect = ui.max_rect();
                self.app_state.viewport_rect =
                    Rect::new(rect.min.x, rect.min.y, rect.max.x, rect.max.y);
            }
            Tab::Edit => {
                if let Some(kmp) = &mut self.kmp {
                    for point in kmp.itpt.entries.iter_mut() {
                        ui.horizontal(|ui| {
                            ui.add(egui::DragValue::new(&mut point.position.x).speed(20.));
                            ui.add(egui::DragValue::new(&mut point.position.y).speed(20.));
                            ui.add(egui::DragValue::new(&mut point.position.z).speed(20.));
                        });
                    }
                }
            }
            Tab::Settings => {
                ui.label("These settings will be saved when you close the app.");
                ui.add(
                    egui::Slider::new(&mut self.settings.point_scale, 0.01..=2.)
                        .text("Point Scale"),
                );
                // go through and update the normalize multipliers of everything
                for normalize in self.normalize.iter_mut() {
                    normalize.multiplier = self.settings.point_scale;
                }
                ui.checkbox(
                    &mut self.settings.open_course_kcl_in_directory,
                    "Auto open course.kcl",
                ).on_hover_text("If enabled, when opening a KMP file, if there is a 'course.kcl' file in the same directory, it will also be opened");

                egui::CollapsingHeader::new("Collision Model")
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.checkbox(
                            &mut self.settings.kcl_model.backface_culling,
                            "Backface Culling",
                        )
                        .on_hover_text(
                            "Whether or not the back faces of the collision model are shown",
                        );
                        let visible = &mut self.settings.kcl_model.visible;
                        use KclFlag::*;

                        let mut show_walls = visible[Wall1 as usize]
                            && visible[Wall2 as usize]
                            && visible[WeakWall as usize];
                        let mut show_invisible_walls =
                            visible[InvisibleWall1 as usize] && visible[InvisibleWall2 as usize];
                        let mut show_death_barriers =
                            visible[SolidFall as usize] && visible[FallBoundary as usize];
                        let mut show_effects_triggers = visible[ItemStateModifier as usize]
                            && visible[EffectTrigger as usize]
                            && visible[SoundTrigger as usize]
                            && visible[KclFlag::CannonTrigger as usize];

                        let (
                            show_walls_before,
                            show_invisible_walls_before,
                            show_death_barriers_before,
                            show_effects_triggers_before,
                        ) = (
                            show_walls,
                            show_invisible_walls,
                            show_death_barriers,
                            show_effects_triggers,
                        );

                        ui.checkbox(&mut show_walls, "Show Walls");
                        ui.checkbox(&mut show_invisible_walls, "Show Invisible Walls");
                        ui.checkbox(&mut show_death_barriers, "Show Death Barriers");
                        ui.checkbox(&mut show_effects_triggers, "Show Effects & Triggers");

                        if show_walls != show_walls_before {
                            [
                                visible[Wall1 as usize],
                                visible[Wall2 as usize],
                                visible[WeakWall as usize],
                            ] = [show_walls; 3];
                        }
                        if show_invisible_walls != show_invisible_walls_before {
                            [
                                visible[InvisibleWall1 as usize],
                                visible[InvisibleWall2 as usize],
                            ] = [show_invisible_walls; 2];
                        }
                        if show_death_barriers != show_death_barriers_before {
                            [visible[SolidFall as usize], visible[FallBoundary as usize]] =
                                [show_death_barriers; 2];
                        }
                        if show_effects_triggers != show_effects_triggers_before {
                            [
                                visible[ItemStateModifier as usize],
                                visible[EffectTrigger as usize],
                                visible[SoundTrigger as usize],
                                visible[CannonTrigger as usize],
                            ] = [show_effects_triggers; 4];
                        }

                        ui.collapsing("Customise", |ui| {
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
                                    ui.color_edit_button_rgba_unmultiplied(
                                        &mut self.settings.kcl_model.color[i],
                                    );
                                    ui.checkbox(
                                        &mut self.settings.kcl_model.visible[i],
                                        kcl_flag.to_string(),
                                    );
                                });
                            }
                        });
                    });

                egui::CollapsingHeader::new("Camera").default_open(true).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if ui.selectable_value(&mut self.settings.camera.mode, CameraMode::Fly, "Fly").clicked() {
                            self.ev_camera_mode_changed.send(CameraModeChanged(CameraMode::Fly));
                        }
                        if ui.selectable_value(&mut self.settings.camera.mode, CameraMode::Orbit, "Orbit").clicked() {
                            self.ev_camera_mode_changed.send(CameraModeChanged(CameraMode::Orbit));
                        }
                        if ui.selectable_value(&mut self.settings.camera.mode, CameraMode::TopDown, "Top Down").clicked() {
                            self.ev_camera_mode_changed.send(CameraModeChanged(CameraMode::TopDown));
                        }
                    });

                    ui.horizontal(|ui| {
                        if ui.button("Reset Positions").clicked() {
                            let fly_default = FlySettings::default();
                            let orbit_default = OrbitSettings::default();
                            let topdown_default = TopDownSettings::default();
                            *self.fly_cam = Transform::from_translation(fly_default.start_pos)
                                .looking_at(Vec3::ZERO, Vec3::Y);
                            *self.orbit_cam = Transform::from_translation(orbit_default.start_pos)
                                .looking_at(Vec3::ZERO, Vec3::Y);
                            *self.topdown_cam.0 = Transform::from_translation(topdown_default.start_pos)
                                .looking_at(Vec3::ZERO, Vec3::Z);
                            *self.topdown_cam.1 = Projection::Orthographic(OrthographicProjection {
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
                            ui.add(
                                egui::DragValue::new(&mut self.settings.camera.fly.look_sensitivity).speed(0.1),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label("Speed").on_hover_text("How fast the camera moves");
                            ui.add(egui::DragValue::new(&mut self.settings.camera.fly.speed).speed(0.1));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Speed Multiplier").on_hover_text(
                                "How much faster the camera moves when holding the speed boost button",
                            );
                            ui.add(egui::DragValue::new(&mut self.settings.camera.fly.speed_boost).speed(0.1));
                        });
                        ui.checkbox(&mut self.settings.camera.fly.hold_mouse_to_move, "Hold Mouse To Move")
                            .on_hover_text("Whether or not the mouse button needs to be pressed in order to move the camera");

                        ui.horizontal(|ui| {
                            ui.label("Mouse Button").on_hover_text("The mouse button that needs to be pressed to move the camera");
                            egui::ComboBox::from_id_source("Mouse Button")
                            .selected_text(format!("{:?}", self.settings.camera.fly.key_bindings.mouse_button))
                            .width(60.)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.settings.camera.fly.key_bindings.mouse_button, MouseButton::Left, "Left");
                                ui.selectable_value(&mut self.settings.camera.fly.key_bindings.mouse_button, MouseButton::Middle, "Middle");
                                ui.selectable_value(&mut self.settings.camera.fly.key_bindings.mouse_button, MouseButton::Right, "Right");
                            });
                        });
                    });
                    ui.collapsing("Orbit Camera", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Rotate Sensitivity")
                                .on_hover_text("How sensitive the camera rotation is to mouse movements");
                            ui.add(
                                egui::DragValue::new(&mut self.settings.camera.orbit.rotate_sensitivity)
                                    .speed(0.1),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label("Pan Sensitivity:")
                                .on_hover_text("How sensitive the camera panning is to mouse movements");
                            ui.add(
                                egui::DragValue::new(&mut self.settings.camera.orbit.pan_sensitivity).speed(0.1),
                            );
                        });

                        ui.horizontal(|ui| {
                            ui.label("Scroll Sensitivity")
                                .on_hover_text("How sensitive the camera zoom is to scrolling");
                            ui.add(
                                egui::DragValue::new(&mut self.settings.camera.orbit.scroll_sensitivity)
                                    .speed(0.1),
                            );
                        });

                        ui.horizontal(|ui| {
                            ui.label("Mouse Button").on_hover_text("The mouse button that needs to be pressed to move the camera");
                            egui::ComboBox::from_id_source("Mouse Button")
                            .selected_text(format!("{:?}", self.settings.camera.orbit.key_bindings.mouse_button))
                            .width(60.)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.settings.camera.orbit.key_bindings.mouse_button, MouseButton::Left, "Left");
                                ui.selectable_value(&mut self.settings.camera.orbit.key_bindings.mouse_button, MouseButton::Middle, "Middle");
                                ui.selectable_value(&mut self.settings.camera.orbit.key_bindings.mouse_button, MouseButton::Right, "Right");
                            });
                        });
                    });
                    ui.collapsing("Top Down Camera", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Move Sensitivity")
                                .on_hover_text("How sensitive the camera movement is to mouse movements");
                            ui.add(
                                egui::DragValue::new(&mut self.settings.camera.top_down.move_sensitivity)
                                    .speed(0.1),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label("Scroll Sensitivity")
                                .on_hover_text("How sensitive the camera zoom is to scrolling");
                            ui.add(
                                egui::DragValue::new(&mut self.settings.camera.top_down.scroll_sensitivity)
                                    .speed(0.1),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label("Mouse Button").on_hover_text("The mouse button that needs to be pressed to move the camera");
                            egui::ComboBox::from_id_source("Mouse Button")
                            .selected_text(format!("{:?}", self.settings.camera.top_down.key_bindings.mouse_button))
                            .width(60.)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.settings.camera.top_down.key_bindings.mouse_button, MouseButton::Left, "Left");
                                ui.selectable_value(&mut self.settings.camera.top_down.key_bindings.mouse_button, MouseButton::Middle, "Middle");
                                ui.selectable_value(&mut self.settings.camera.top_down.key_bindings.mouse_button, MouseButton::Right, "Right");
                            });
                        });
                    });
                });

                ui.horizontal(|ui| {
                    if ui.button("Export Settings").clicked() {
                        let mut dialog = FileDialog::save_file(None)
                            .default_size((500., 250.))
                            .default_filename("kmpeek_settings.json");
                        dialog.open();

                        self.app_state.file_dialog = Some((dialog, DialogType::ExportSettings));
                    }

                    if ui.button("Import Settings").clicked() {
                        let mut dialog = FileDialog::open_file(None).default_size((500., 250.));
                        dialog.open();
                        self.app_state.file_dialog = Some((dialog, DialogType::ImportSettings));
                    }
                });
                ui.horizontal(|ui| {
                    if ui.button("Reset Tab Layout").clicked() {
                        self.settings.reset_tree = true;
                    }
                    if ui.button("Reset All Settings").clicked() {
                        *self.settings = AppSettings::default();
                        self.settings.reset_tree = true;
                    }
                });
            }
        };
    }
    // show the title of the tab - the 'Tab' type already stores its title anyway
    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.to_string().into()
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn update_ui(
    keys: Res<Input<KeyCode>>,
    mut contexts: EguiContexts,
    mut app_state: ResMut<AppState>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut ev_kmp_file_selected: EventWriter<KmpFileSelected>,
    mut ev_kcl_file_selected: EventWriter<KclFileSelected>,
    mut ev_camera_mode_changed: EventWriter<CameraModeChanged>,

    mut normalize: Query<&mut NormalizeScale>,

    mut kmp: Option<ResMut<Kmp>>,

    mut cams: (
        // fly cam
        Query<&mut Transform, (With<FlyCam>, Without<OrbitCam>, Without<TopDownCam>)>,
        // orbit cam
        Query<&mut Transform, (Without<FlyCam>, With<OrbitCam>, Without<TopDownCam>)>,
        // topdown cam
        Query<
            (&mut Transform, &mut Projection),
            (Without<FlyCam>, Without<OrbitCam>, With<TopDownCam>),
        >,
    ),

    mut image_assets: ResMut<Assets<Image>>,
    viewport: ResMut<ViewportImage>,
    mut pkv: ResMut<PkvStore>,
    mut undo_stack: ResMut<UndoStack>,
) {
    // get variables we need in this system from queries/assets
    let mut fly_cam = cams.0.get_single_mut().unwrap();
    let mut orbit_cam = cams.1.get_single_mut().unwrap();
    let mut topdown_cam = cams.2.get_single_mut().unwrap();
    let window = window.get_single().unwrap();
    let viewport_image = image_assets.get_mut(&viewport).unwrap();
    let viewport_tex_id = contexts.image_id(&viewport).unwrap();
    let mut settings = pkv.get::<AppSettings>("settings").unwrap();
    let mut tree = pkv.get::<DockTree>("tree").unwrap();
    let mut normalize: Vec<Mut<NormalizeScale>> = normalize.iter_mut().collect();
    let normalize: Vec<&mut NormalizeScale> = normalize.iter_mut().map(|x| x.as_mut()).collect();
    let ctx = contexts.ctx_mut();

    // things which can be called from both the UI and keybinds
    macro_rules! open_file {
        () => {
            let mut dialog = FileDialog::open_file(None)
                .default_size((500., 250.))
                .filter(Box::new(|path| {
                    if let Some(os_str) = path.extension() {
                        if let Some(str) = os_str.to_str() {
                            return str == "kcl" || str == "kmp";
                        }
                    }
                    false
                }));
            dialog.open();
            app_state.file_dialog = Some((dialog, DialogType::OpenKmpKcl));
        };
    }
    macro_rules! undo {
        () => {
            if let Some(ref mut kmp) = kmp {
                undo_stack.undo(kmp);
            }
        };
    }
    macro_rules! redo {
        () => {
            if let Some(ref mut kmp) = kmp {
                undo_stack.redo(kmp);
            }
        };
    }
    macro_rules! save {
        () => {
            if let (Some(kmp_file_path), Some(ref mut kmp)) = (&app_state.kmp_file_path, &mut kmp) {
                let kmp_file = File::create(kmp_file_path).expect("could not create kmp file");
                kmp.write(kmp_file).expect("could not write kmp file");
            }
        };
    }

    // keybinds
    // if the control/command key is pressed
    if (!cfg!(target_os = "macos")
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight)))
        || (cfg!(target_os = "macos")
            && (keys.pressed(KeyCode::SuperLeft) || keys.pressed(KeyCode::SuperRight)))
    {
        if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
            // keybinds with shift held
            if keys.just_pressed(KeyCode::Z) {
                redo!();
            }
        // keybinds without shift held
        } else if keys.just_pressed(KeyCode::O) {
            open_file!();
        } else if keys.just_pressed(KeyCode::S) {
            save!();
        } else if keys.just_pressed(KeyCode::Z) {
            undo!();
        }
    }

    // menu bar
    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            let mut sc_btn = "Ctrl";
            if cfg!(target_os = "macos") {
                sc_btn = "Cmd";
            }
            ui.menu_button("File", |ui| {
                if ui
                    .add(egui::Button::new("Open KCL/KMP").shortcut_text(format!("{sc_btn}+O")))
                    .clicked()
                {
                    open_file!();
                    ui.close_menu();
                }
                if ui
                    .add(egui::Button::new("Save").shortcut_text(format!("{sc_btn}+S")))
                    .clicked()
                {
                    save!();
                    ui.close_menu();
                }
            });
            ui.menu_button("Edit", |ui| {
                if ui
                    .add(egui::Button::new("Undo").shortcut_text(format!("{sc_btn}+Z")))
                    .clicked()
                {
                    undo!();
                }
                if ui
                    .add(egui::Button::new("Redo").shortcut_text(format!("{sc_btn}+Shift+Z")))
                    .clicked()
                {
                    redo!();
                }
            });

            ui.menu_button("Window", |ui| {
                // toggle each tab on or off
                for tab in Tab::iter() {
                    // search for the tab and see if it currently exists
                    let tab_in_tree = tree.find_tab(&tab);
                    if ui
                        .selectable_label(tab_in_tree.is_some(), tab.to_string())
                        .clicked()
                    {
                        // remove if it exists, else create it
                        if let Some(index) = tab_in_tree {
                            tree.remove_tab(index);
                        } else {
                            tree.push_to_focused_leaf(tab);
                        }
                    }
                }
            });
        });
    });

    // show the actual dock area
    DockArea::new(&mut tree)
        .style(Style::from_egui(ctx.style().as_ref()))
        .show(
            ctx,
            &mut TabViewer {
                viewport_image,
                viewport_tex_id,
                window,
                app_state: &mut app_state,
                settings: &mut settings,
                kmp,

                normalize,
                fly_cam: &mut fly_cam,
                orbit_cam: &mut orbit_cam,
                topdown_cam: (&mut topdown_cam.0, &mut topdown_cam.1),

                ev_camera_mode_changed: &mut ev_camera_mode_changed,
            },
        );
    if settings.reset_tree {
        tree = DockTree::default();
        settings.reset_tree = false;
    }

    let mut kmp_file_path: Option<PathBuf> = None;
    if let Some(dialog) = &mut app_state.file_dialog {
        if dialog.0.show(ctx).selected() {
            if let Some(file) = dialog.0.path() {
                match dialog.1 {
                    DialogType::OpenKmpKcl => {
                        if let Some(file_ext) = file.extension() {
                            if file_ext == "kmp" {
                                kmp_file_path = Some(file.into());
                                ev_kmp_file_selected.send(KmpFileSelected(file.into()));
                                if settings.open_course_kcl_in_directory {
                                    let mut course_kcl_path = file.to_owned();
                                    course_kcl_path.set_file_name("course.kcl");
                                    if course_kcl_path.exists() {
                                        ev_kcl_file_selected.send(KclFileSelected(course_kcl_path));
                                    }
                                }
                            } else if file_ext == "kcl" {
                                ev_kcl_file_selected.send(KclFileSelected(file.into()));
                            }
                        }
                    }
                    DialogType::ExportSettings => {
                        let settings_string = serde_json::to_string_pretty(&settings)
                            .expect("could not convert settings to json");
                        let mut file =
                            File::create(file).expect("could not create user settings file");
                        file.write_all(settings_string.as_bytes())
                            .expect("could not write to user settings file");
                    }
                    DialogType::ImportSettings => {
                        let input_settings_string =
                            read_to_string(file).expect("could not read user settings to string");
                        if let Ok(input_settings) = serde_json::from_str(&input_settings_string) {
                            settings = input_settings;
                        }
                    }
                }
            }
        }
    }
    app_state.kmp_file_path = kmp_file_path;

    pkv.set("settings", &settings)
        .expect("could not set user settings");
    pkv.set("tree", &tree).expect("could not set dock tree");
}
