use super::{
    app_state::{AppSettings, AppState},
    update_ui::DialogType,
    viewport::render_viewport,
};
use crate::{
    util::kcl_file::*,
    viewer::{
        camera::{
            CameraMode, CameraModeChanged, CameraSettings, FlySettings, OrbitSettings,
            TopDownSettings,
        },
        kmp::KmpVisibilityUpdated,
    },
};
use bevy::prelude::*;
use bevy_egui::egui::{self, TextureId};
use bevy_pkv::PkvStore;
use egui_dock::{DockState, NodeIndex};
use egui_file::*;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter};

pub struct DockTreePlugin;
impl Plugin for DockTreePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_docktree);
    }
}
fn setup_docktree(mut commands: Commands, mut pkv: ResMut<PkvStore>) {
    // get the docktree if it exists, if not, set it to default
    let tree = match pkv.get::<DockTree>("tree") {
        Ok(tree) => tree,
        Err(_) => {
            pkv.set("tree", &DockTree::default()).unwrap();
            DockTree::default()
        }
    };
    commands.insert_resource(tree);
}

#[derive(Deref, DerefMut, Resource, Serialize, Deserialize)]
pub struct DockTree(DockState<Tab>);
impl Default for DockTree {
    fn default() -> Self {
        let mut tree = DockState::new(vec![Tab::Viewport]);
        tree.main_surface_mut()
            .split_left(NodeIndex::root(), 0.2, vec![Tab::Edit, Tab::Settings]);
        Self(tree)
    }
}

#[derive(Display, PartialEq, EnumIter, Serialize, Deserialize, Clone, Copy)]
pub enum Tab {
    Viewport,
    Edit,
    Settings,
}

// this tells egui how to render each tab
pub struct TabViewer<'a, 'b, 'c> {
    // add into here any data that needs to be passed into any tabs
    pub viewport_image: &'a mut Image,
    pub viewport_tex_id: TextureId,
    pub window: &'a Window,

    pub app_state: &'a mut AppState,
    pub settings: &'a mut AppSettings,

    pub fly_cam: &'a mut Transform,
    pub orbit_cam: &'a mut Transform,
    pub topdown_cam: (&'a mut Transform, &'a mut Projection),
    pub ev_camera_mode_changed: &'a mut EventWriter<'b, CameraModeChanged>,
    pub ev_kmp_visibility_updated: &'a mut EventWriter<'c, KmpVisibilityUpdated>,
}
impl egui_dock::TabViewer for TabViewer<'_, '_, '_> {
    // each tab will be distinguished by a string - its name
    type Tab = Tab;
    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        // we can do different things inside the tab depending on its name
        match tab {
            Tab::Viewport => render_viewport(
                ui,
                self.viewport_image,
                self.window,
                self.viewport_tex_id,
                self.app_state,
            ),
            Tab::Edit => {}
            Tab::Settings => {
                ui.label("These settings will be saved when you close the app.");
                ui.add(
                    egui::Slider::new(&mut self.settings.kmp_model.point_scale, 0.01..=2.)
                        .text("Point Scale"),
                );
                ui.checkbox(&mut self.settings.kmp_model.normalize, "Normalize points");

                macro_rules! visibility_checkbox {
                    ($sect:ident, $msg:expr) => {
                        let checkbox =
                            ui.checkbox(&mut self.settings.kmp_model.sections.$sect.visible, $msg);
                        if checkbox.changed() {
                            self.ev_kmp_visibility_updated.send(KmpVisibilityUpdated);
                        }
                    };
                }
                visibility_checkbox!(start_points, "Start points visible");
                visibility_checkbox!(enemy_paths, "Enemy paths visible");
                visibility_checkbox!(item_paths, "Item paths visible");

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
                        let mut show_invis_walls =
                            visible[InvisibleWall1 as usize] && visible[InvisibleWall2 as usize];
                        let mut show_death_barriers =
                            visible[SolidFall as usize] && visible[FallBoundary as usize];
                        let mut show_effects_triggers = visible[ItemStateModifier as usize]
                            && visible[EffectTrigger as usize]
                            && visible[SoundTrigger as usize]
                            && visible[KclFlag::CannonTrigger as usize];

                        let show_walls_checkbox = ui.checkbox(&mut show_walls, "Show Walls");
                        let show_invis_walls_checkbox =
                            ui.checkbox(&mut show_invis_walls, "Show Invisible Walls");
                        let show_death_barriers_checkbox =
                            ui.checkbox(&mut show_death_barriers, "Show Death Barriers");
                        let show_effects_triggers_checkbox =
                            ui.checkbox(&mut show_effects_triggers, "Show Effects & Triggers");

                        if show_walls_checkbox.changed() {
                            [
                                visible[Wall1 as usize],
                                visible[Wall2 as usize],
                                visible[WeakWall as usize],
                            ] = [show_walls; 3];
                        }

                        if show_invis_walls_checkbox.changed() {
                            [
                                visible[InvisibleWall1 as usize],
                                visible[InvisibleWall2 as usize],
                            ] = [show_invis_walls; 2];
                        }
                        if show_death_barriers_checkbox.changed() {
                            [visible[SolidFall as usize], visible[FallBoundary as usize]] =
                                [show_death_barriers; 2];
                        }
                        if show_effects_triggers_checkbox.changed() {
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
