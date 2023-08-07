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
use egui_dock::{DockArea, NodeIndex, Style, Tree};
use std::{fs::File, path::PathBuf};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub struct SetupAppStateSet;

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin)
            .add_event::<KmpFileSelected>()
            .add_event::<KclFileSelected>()
            .add_systems(
                Startup,
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

    pub show_walls: bool,
    pub show_invisible_walls: bool,
    pub show_death_barriers: bool,
    pub show_effects_triggers: bool,

    pub file_dialog: Option<(FileDialog, String)>,
    pub kmp_file_path: Option<PathBuf>,

    pub point_scale: f32,
}

#[derive(Debug, PartialEq, EnumIter)]
pub enum Tab {
    Viewport,
    Edit,
    Settings,
}

#[derive(Deref, DerefMut, Resource)]
pub struct DockTree(Tree<Tab>);

// stores the image which the camera renders to, so that we can display a viewport inside a tab
#[derive(Deref, Resource)]
pub struct ViewportImage(Handle<Image>);

pub fn setup_app_state(
    mut commands: Commands,
    mut egui_user_textures: ResMut<EguiUserTextures>,
    mut images: ResMut<Assets<Image>>,
) {
    // default size (will be immediately overwritten)
    let size = Extent3d {
        width: 512,
        height: 512,
        ..default()
    };

    // this is the texture that will be rendered to
    let mut image: Image = Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size,
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

    // fill image.data with zeroes
    image.resize(size);

    // create a handle to the image
    let image_handle = images.add(image);
    egui_user_textures.add_image(image_handle.clone());

    commands.insert_resource(ViewportImage(image_handle));

    // create the docktree
    let mut tree = Tree::new(vec![Tab::Viewport]);
    tree.split_left(NodeIndex::root(), 0.2, vec![Tab::Edit, Tab::Settings]);
    commands.insert_resource(DockTree(tree));

    let app_state = AppState {
        customise_kcl_open: false,
        camera_settings_open: false,

        show_walls: true,
        show_invisible_walls: true,
        show_death_barriers: true,
        show_effects_triggers: true,

        file_dialog: None,
        kmp_file_path: None,

        point_scale: 1.,
    };

    commands.insert_resource(app_state);
}

#[derive(Event)]
pub struct KmpFileSelected(pub PathBuf);

#[derive(Event)]
pub struct KclFileSelected(pub PathBuf);

// this tells egui how to render each tab
#[allow(clippy::type_complexity)]
struct TabViewer<'a> {
    // add into here any data that needs to be passed into any tabs
    viewport_image: &'a mut Image,
    viewport_tex_id: TextureId,
    window_scale_factor: f64,

    app_state: &'a mut AppState,
    kcl_model_settings: &'a mut KclModelSettings,
    camera_settings: &'a mut CameraSettings,

    itpt: Vec<&'a mut Transform>,
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
                        width: viewport_size.x as u32 * self.window_scale_factor as u32,
                        height: viewport_size.y as u32 * self.window_scale_factor as u32,
                        ..default()
                    };
                    self.viewport_image.resize(size);
                }
                // show the viewport image
                ui.image(self.viewport_tex_id, viewport_size.to_array());
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
                    egui::Slider::new(&mut self.app_state.point_scale, 0.01..=2.)
                        .text("Point Scale"),
                );

                ui.collapsing("Collision Model", |ui| {
                    let (
                        mut show_walls,
                        mut show_invisible_walls,
                        mut show_death_barriers,
                        mut show_effects_triggers,
                    ) = (
                        self.app_state.show_walls,
                        self.app_state.show_invisible_walls,
                        self.app_state.show_death_barriers,
                        self.app_state.show_effects_triggers,
                    );
                    ui.checkbox(&mut show_walls, "Show Walls");
                    ui.checkbox(&mut show_invisible_walls, "Show Invisible Walls");
                    ui.checkbox(&mut show_death_barriers, "Show Death Barriers");
                    ui.checkbox(&mut show_effects_triggers, "Show Effects & Triggers");
                    if show_walls != self.app_state.show_walls {
                        self.app_state.show_walls = show_walls;
                        self.kcl_model_settings.visible[KclFlag::Wall1 as usize] = show_walls;
                        self.kcl_model_settings.visible[KclFlag::Wall2 as usize] = show_walls;
                        self.kcl_model_settings.visible[KclFlag::WeakWall as usize] = show_walls;
                    }
                    if show_invisible_walls != self.app_state.show_invisible_walls {
                        self.app_state.show_invisible_walls = show_invisible_walls;
                        self.kcl_model_settings.visible[KclFlag::InvisibleWall1 as usize] =
                            show_invisible_walls;
                        self.kcl_model_settings.visible[KclFlag::InvisibleWall2 as usize] =
                            show_invisible_walls;
                    }
                    if show_death_barriers != self.app_state.show_death_barriers {
                        self.app_state.show_death_barriers = show_death_barriers;
                        self.kcl_model_settings.visible[KclFlag::SolidFall as usize] =
                            show_death_barriers;
                        self.kcl_model_settings.visible[KclFlag::FallBoundary as usize] =
                            show_death_barriers;
                    }
                    if show_effects_triggers != self.app_state.show_effects_triggers {
                        self.app_state.show_effects_triggers = show_effects_triggers;
                        self.kcl_model_settings.visible[KclFlag::ItemStateModifier as usize] =
                            show_effects_triggers;
                        self.kcl_model_settings.visible[KclFlag::EffectTrigger as usize] =
                            show_effects_triggers;
                        self.kcl_model_settings.visible[KclFlag::SoundTrigger as usize] =
                            show_effects_triggers;
                        self.kcl_model_settings.visible[KclFlag::CannonTrigger as usize] =
                            show_effects_triggers;
                    }
                    if ui.button("Customise...").clicked() {
                        self.app_state.customise_kcl_open = true;
                    }
                });

                ui.collapsing("Camera", |ui| {
                    ui.horizontal(|ui| {
                        let mut mode = self.camera_settings.mode;
                        ui.selectable_value(&mut mode, CameraMode::Fly, "Fly");
                        ui.selectable_value(&mut mode, CameraMode::Orbit, "Orbit");
                        ui.selectable_value(&mut mode, CameraMode::TopDown, "Top Down");
                        if self.camera_settings.mode != mode {
                            self.camera_settings.mode = mode;
                        }
                    });
                    if ui.button("Camera Settings...").clicked() {
                        self.app_state.camera_settings_open = true;
                    }
                });

                ui.separator();
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
    mut camera_settings: ResMut<CameraSettings>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut ev_kmp_file_selected: EventWriter<KmpFileSelected>,
    mut ev_kcl_file_selected: EventWriter<KclFileSelected>,
    mut kcl_model_settings: ResMut<KclModelSettings>,

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
) {
    // get variables for camera and window
    let mut fly_cam_transform = cams
        .0
        .get_single_mut()
        .expect("Could not get single fly cam in update ui");
    let mut orbit_cam_transform = cams
        .1
        .get_single_mut()
        .expect("Could not get single orbit cam in update ui");
    let (mut topdown_cam_transform, mut topdown_cam_projection) = cams
        .2
        .get_single_mut()
        .expect("Could not get single topdown cam in update ui");
    let window = window
        .get_single()
        .expect("Could not get single primary window in update ui");
    let viewport_image = image_assets
        .get_mut(&viewport)
        .expect("Could not get viewport image in update ui");

    let viewport_tex_id = contexts
        .image_id(&viewport)
        .expect("Could not get viewport texture ID in update ui");

    let ctx = contexts.ctx_mut();

    // things which can be called from both the UI and keybinds (may restructure this later)
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
            println!("undo");
        };
    }
    macro_rules! redo {
        () => {
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
    if (!cfg!(target_os = "macos")
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight)))
        || (cfg!(target_os = "macos")
            && (keys.pressed(KeyCode::SuperLeft) || keys.pressed(KeyCode::SuperRight)))
    {
        // ^ if the control/command key is pressed:
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

    let mut customise_kcl_open = app_state.customise_kcl_open;
    egui::Window::new("Customise Collision Model")
        .open(&mut customise_kcl_open)
        .collapsible(false)
        .min_width(300.)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Check All").clicked() {
                        kcl_model_settings.visible = [true; 32];
                    }
                    if ui.button("Uncheck All").clicked() {
                        kcl_model_settings.visible = [false; 32];
                    }
                    if ui.button("Reset").clicked() {
                        *kcl_model_settings = Default::default();
                    }
                });
                ui.separator();
                // this macro means that the same ui options can be repeated without copy and pasting it 32 times
                macro_rules! kcl_type_options {
                    ($name:expr, $i:expr) => {
                        ui.horizontal(|ui| {
                            ui.color_edit_button_rgba_unmultiplied(
                                &mut kcl_model_settings.color[$i],
                            );
                            ui.checkbox(&mut kcl_model_settings.visible[$i], $name);
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

    let mut itpt: Vec<Mut<Transform>> = itpt.iter_mut().map(|(x, _)| x).collect();
    let itpt: Vec<&mut Transform> = itpt.iter_mut().map(|x| x.as_mut()).collect();

    // show the actual dock area
    DockArea::new(&mut tree)
        .style(Style::from_egui(ctx.style().as_ref()))
        .show(
            ctx,
            &mut TabViewer {
                viewport_image,
                viewport_tex_id,
                window_scale_factor: window.scale_factor(),
                app_state: app_state.as_mut(),
                kcl_model_settings: kcl_model_settings.as_mut(),
                camera_settings: camera_settings.as_mut(),
                itpt,
            },
        );

    for mut normalize in normalize.iter_mut() {
        normalize.multiplier = app_state.point_scale;
    }
}
