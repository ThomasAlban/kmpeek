use crate::{
    camera::{
        CameraMode, CameraSettings, FlyCam, FlySettings, OrbitCam, OrbitSettings, TopDownCam,
        TopDownSettings,
    },
    file_dialog::*,
    kcl_file::*,
    kcl_model::KclModelSettings,
    kmp_file::Kmp,
    kmp_model::{ItptModel, NormalizeScale},
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
// use bevy_mod_picking::prelude::*;
use egui_dock::{DockArea, NodeIndex, Style, Tree};
use serde::{Deserialize, Serialize};
use std::{fs::File, path::PathBuf};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

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

    pub file_dialog: Option<(FileDialog, String)>,
    pub kmp_file_path: Option<PathBuf>,
    pub mouse_in_viewport: bool,
}

#[derive(Serialize, Deserialize)]
pub struct AppSettings {
    pub camera: CameraSettings,
    pub kcl_model: KclModelSettings,
    pub point_scale: f32,
}
impl Default for AppSettings {
    fn default() -> Self {
        Self {
            camera: CameraSettings::default(),
            kcl_model: KclModelSettings::default(),
            point_scale: 1.,
        }
    }
}

#[derive(Deref, DerefMut, Resource)]
pub struct DockTree(Tree<Tab>);

// stores the image which the camera renders to, so that we can display a viewport inside a tab
#[derive(Deref, Resource)]
pub struct ViewportImage(Handle<Image>);

#[derive(Component)]
pub struct BevyModPickingPointer;

pub fn setup_app_state(
    mut commands: Commands,
    mut egui_user_textures: ResMut<EguiUserTextures>,
    mut images: ResMut<Assets<Image>>,
    mut pkv: ResMut<PkvStore>,
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

    // create the docktree
    let mut tree = Tree::new(vec![Tab::Viewport]);
    tree.split_left(NodeIndex::root(), 0.2, vec![Tab::Edit, Tab::Settings]);
    commands.insert_resource(DockTree(tree));

    // set app settings to defaults if they do not already exist
    if pkv.get::<AppSettings>("settings").is_err() {
        pkv.set("settings", &AppSettings::default())
            .expect("failed to store user settings");
    }

    let app_state = AppState {
        customise_kcl_open: false,
        camera_settings_open: false,

        file_dialog: None,
        kmp_file_path: None,
        mouse_in_viewport: false,
    };

    let camera_settings = CameraSettings::default();

    commands.insert_resource(app_state);
    commands.insert_resource(camera_settings);
}

#[derive(Event)]
pub struct KmpFileSelected(pub PathBuf);

#[derive(Event)]
pub struct KclFileSelected(pub PathBuf);

#[derive(Debug, PartialEq, EnumIter)]
pub enum Tab {
    Viewport,
    Edit,
    Settings,
}

// this tells egui how to render each tab
#[allow(clippy::type_complexity)]
struct TabViewer<'a> {
    // add into here any data that needs to be passed into any tabs
    viewport_image: &'a mut Image,
    viewport_tex_id: TextureId,
    window: &'a Window,

    app_state: &'a mut AppState,
    settings: &'a mut AppSettings,

    itpt: Vec<&'a mut Transform>,
    normalize: Vec<&'a mut NormalizeScale>,
    // pointer: &'a mut PointerLocation,
    fly_cam: (&'a mut Camera, &'a mut Transform),
    orbit_cam: (&'a mut Camera, &'a mut Transform),
    topdown_cam: (&'a mut Camera, &'a mut Transform, &'a mut Projection),
}
impl egui_dock::TabViewer for TabViewer<'_> {
    // each tab will be distinguished by a string - its name
    type Tab = Tab;
    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        // we can do different things inside the tab depending on its name
        match tab {
            Tab::Viewport => {
                let viewport_size = vec2(ui.available_width(), ui.available_height());
                // resize the viewport if needed
                if self.viewport_image.size().as_uvec2() != viewport_size.as_uvec2() {
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
            }
            Tab::Edit => {
                for point in self.itpt.iter_mut() {
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut point.translation.x).speed(20.));
                        ui.add(egui::DragValue::new(&mut point.translation.y).speed(20.));
                        ui.add(egui::DragValue::new(&mut point.translation.z).speed(20.));
                    });
                }
            }
            Tab::Settings => {
                ui.add(
                    egui::Slider::new(&mut self.settings.point_scale, 0.01..=2.)
                        .text("Point Scale"),
                );
                // go through and update the normalize multipliers of everything
                for normalize in self.normalize.iter_mut() {
                    normalize.multiplier = self.settings.point_scale;
                }

                egui::CollapsingHeader::new("Collision Model")
                    .default_open(true)
                    .show(ui, |ui| {
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
                        ui.selectable_value(&mut self.settings.camera.mode, CameraMode::Fly, "Fly");
                        ui.selectable_value(&mut self.settings.camera.mode, CameraMode::Orbit, "Orbit");
                        ui.selectable_value(&mut self.settings.camera.mode, CameraMode::TopDown, "Top Down");
                    });

                    ui.horizontal(|ui| {
                        if ui.button("Reset Positions").clicked() {
                            let fly_default = FlySettings::default();
                            let orbit_default = OrbitSettings::default();
                            let topdown_default = TopDownSettings::default();
                            *self.fly_cam.1 = Transform::from_translation(fly_default.start_pos)
                                .looking_at(Vec3::ZERO, Vec3::Y);
                            *self.orbit_cam.1 = Transform::from_translation(orbit_default.start_pos)
                                .looking_at(Vec3::ZERO, Vec3::Y);
                            *self.topdown_cam.1 = Transform::from_translation(topdown_default.start_pos)
                                .looking_at(Vec3::ZERO, Vec3::Z);
                            *self.topdown_cam.2 = Projection::Orthographic(OrthographicProjection {
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
                            ui.label("Speed Boost").on_hover_text(
                                "How much faster the camera moves when holding the speed boost button",
                            );
                            ui.add(egui::DragValue::new(&mut self.settings.camera.fly.speed_boost).speed(0.1));
                        });
                        ui.checkbox(&mut self.settings.camera.fly.hold_mouse_to_move, "Hold Mouse To Move")
                            .on_hover_text("Whether or not the mouse button needs to be pressed in order to move the camera");
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
                    });
                });

                if ui.button("Reset All Settings").clicked() {
                    *self.settings = AppSettings::default();
                }
            }
        };
    }
    // show the title of the tab - the 'Tab' type already stores its title anyway
    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        format!("{tab:?}").into()
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

    mut normalize: Query<&mut NormalizeScale>,

    mut kmp: Option<ResMut<Kmp>>,

    mut cams: (
        // fly cam
        Query<
            (&mut Camera, &mut Transform),
            (With<FlyCam>, Without<OrbitCam>, Without<TopDownCam>),
        >,
        // orbit cam
        Query<
            (&mut Camera, &mut Transform),
            (Without<FlyCam>, With<OrbitCam>, Without<TopDownCam>),
        >,
        // topdown cam
        Query<
            (&mut Camera, &mut Transform, &mut Projection),
            (Without<FlyCam>, Without<OrbitCam>, With<TopDownCam>),
        >,
    ),

    mut itpt: Query<
        (&mut Transform, &ItptModel),
        (
            With<ItptModel>,
            Without<FlyCam>,
            Without<OrbitCam>,
            Without<TopDownCam>,
        ),
    >,

    mut image_assets: ResMut<Assets<Image>>,
    mut tree: ResMut<DockTree>,
    viewport: ResMut<ViewportImage>,
    mut pkv: ResMut<PkvStore>,
    // mut pointer: Query<&mut PointerLocation, With<BevyModPickingPointer>>,
) {
    // get variables we need in this system from queries/assets
    let mut fly_cam = cams
        .0
        .get_single_mut()
        .expect("Could not get fly cam in update ui");
    let mut orbit_cam = cams
        .1
        .get_single_mut()
        .expect("Could not get orbit cam in update ui");
    let mut topdown_cam = cams
        .2
        .get_single_mut()
        .expect("Could not get topdown cam in update ui");
    let window = window
        .get_single()
        .expect("Could not get primary window in update ui");
    let viewport_image = image_assets
        .get_mut(&viewport)
        .expect("Could not get viewport image in update ui");
    let viewport_tex_id = contexts
        .image_id(&viewport)
        .expect("Could not get viewport texture ID in update ui");
    let mut settings = pkv
        .get::<AppSettings>("settings")
        .expect("could not get user settings");
    // let mut pointer = pointer
    //     .get_single_mut()
    //     .expect("Could not get pointer in update ui");
    let mut itpt: Vec<Mut<Transform>> = itpt.iter_mut().map(|(x, _)| x).collect();
    let itpt: Vec<&mut Transform> = itpt.iter_mut().map(|x| x.as_mut()).collect();
    let mut normalize: Vec<Mut<NormalizeScale>> = normalize.iter_mut().collect();
    let normalize: Vec<&mut NormalizeScale> = normalize.iter_mut().map(|x| x.as_mut()).collect();
    let ctx = contexts.ctx_mut();

    // things which can be called from both the UI and keybinds
    macro_rules! open_file {
        ($type:literal) => {
            let mut dialog = FileDialog::open_file(None)
                .default_size((500., 250.))
                .filter(Box::new(|path| {
                    if let Some(os_str) = path.extension() {
                        if let Some(str) = os_str.to_str() {
                            return str == $type;
                        }
                    }
                    false
                }));
            dialog.open();
            app_state.file_dialog = Some((dialog, $type.to_owned()));
        };
    }
    macro_rules! undo {
        () => {
            // to do
            println!("undo");
        };
    }
    macro_rules! redo {
        () => {
            // to do
            println!("redo");
        };
    }
    macro_rules! save {
        () => {
            if let (Some(kmp_file_path), Some(ref mut kmp)) = (&app_state.kmp_file_path, &mut kmp) {
                let kmp_file = File::create(kmp_file_path).unwrap();
                kmp.write(kmp_file).unwrap();
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
            } else if keys.just_pressed(KeyCode::O) {
                open_file!("kcl");
            }
        // keybinds without shift held
        } else if keys.just_pressed(KeyCode::O) {
            open_file!("kmp");
        } else if keys.just_pressed(KeyCode::S) {
            save!();
        } else if keys.just_pressed(KeyCode::Z) {
            undo!();
        }
    }

    if let Some(dialog) = &mut app_state.file_dialog {
        if dialog.0.show(ctx).selected() {
            if let Some(file) = dialog.0.path() {
                if dialog.1 == "kmp" {
                    app_state.kmp_file_path = Some(file.clone());
                    ev_kmp_file_selected.send(KmpFileSelected(file));
                } else if dialog.1 == "kcl" {
                    ev_kcl_file_selected.send(KclFileSelected(file));
                }
            }
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
                    .add(egui::Button::new("Open KMP").shortcut_text(format!("{sc_btn}+O")))
                    .clicked()
                {
                    open_file!("kmp");
                    ui.close_menu();
                }
                if ui
                    .add(egui::Button::new("Open KCL").shortcut_text(format!("{sc_btn}+Shift+O")))
                    .clicked()
                {
                    open_file!("kcl");
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
                        .selectable_label(tab_in_tree.is_some(), format!("{tab:?}"))
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
                itpt,
                normalize,
                // pointer: pointer.as_mut(),
                fly_cam: (&mut fly_cam.0, &mut fly_cam.1),
                orbit_cam: (&mut orbit_cam.0, &mut orbit_cam.1),
                topdown_cam: (&mut topdown_cam.0, &mut topdown_cam.1, &mut topdown_cam.2),
            },
        );

    pkv.set("settings", &settings)
        .expect("could not set user settings");
}
