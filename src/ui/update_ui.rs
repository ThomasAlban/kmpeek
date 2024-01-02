use super::{
    app_state::{AppSettings, AppState},
    dock_tree::DockTree,
    viewport::ViewportImage,
};
use crate::{
    ui::dock_tree::{Tab, TabViewer},
    viewer::{
        camera::{CameraModeChanged, FlyCam, OrbitCam, TopDownCam},
        kmp::KmpVisibilityUpdated,
    },
};
use bevy::{prelude::*, window::PrimaryWindow};
use bevy_egui::{
    egui::{self},
    EguiContexts,
};
use egui_dock::{DockArea, Style};
use egui_file::*;
use std::{
    fs::{read_to_string, File},
    io::Write,
    path::PathBuf,
};
use strum::IntoEnumIterator;

pub struct UpdateUIPlugin;
impl Plugin for UpdateUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<KmpFileSelected>()
            .add_event::<KclFileSelected>()
            .add_systems(Update, update_ui);
    }
}

pub enum DialogType {
    OpenKmpKcl,
    ExportSettings,
    ImportSettings,
}

#[derive(Event)]
pub struct KmpFileSelected(pub PathBuf);

#[derive(Event)]
pub struct KclFileSelected(pub PathBuf);

fn update_ui(
    keys: Res<Input<KeyCode>>,
    mut contexts: EguiContexts,
    mut app_state: ResMut<AppState>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut ev_kmp_file_selected: EventWriter<KmpFileSelected>,
    mut ev_kcl_file_selected: EventWriter<KclFileSelected>,
    mut ev_camera_mode_changed: EventWriter<CameraModeChanged>,
    mut ev_kmp_visibility_updated: EventWriter<KmpVisibilityUpdated>,

    // mut normalize: Query<&mut NormalizeScale>,
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
    mut settings: ResMut<AppSettings>,
    mut tree: ResMut<DockTree>,
) {
    // get variables we need in this system from queries/assets
    let mut fly_cam = cams.0.get_single_mut().unwrap();
    let mut orbit_cam = cams.1.get_single_mut().unwrap();
    let mut topdown_cam = cams.2.get_single_mut().unwrap();
    let window = window.get_single().unwrap();
    let viewport_image = image_assets.get_mut(viewport.id()).unwrap();
    let viewport_tex_id = contexts.image_id(&viewport).unwrap();

    let settings = settings.as_mut();
    let tree = tree.as_mut();
    let ctx = contexts.ctx_mut();

    // things which can be called from both the UI and keybinds
    macro_rules! open_file {
        () => {
            let mut dialog = FileDialog::open_file(None)
                .default_size((500., 250.))
                .show_files_filter(Box::new(|path| {
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
            // if let Some(ref mut kmp) = kmp {
            //     undo_stack.undo(kmp);
            // }
        };
    }
    macro_rules! redo {
        () => {
            // if let Some(ref mut kmp) = kmp {
            //     undo_stack.redo(kmp);
            // }
        };
    }
    macro_rules! save {
        () => {
            // if let (Some(kmp_file_path), Some(ref mut kmp)) = (&app_state.kmp_file_path, &mut kmp) {
            //     let kmp_file = File::create(kmp_file_path).expect("could not create kmp file");
            //     kmp.write(kmp_file).expect("could not write kmp file");
            // }
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
    DockArea::new(tree)
        .style(Style::from_egui(ctx.style().as_ref()))
        .show(
            ctx,
            &mut TabViewer {
                viewport_image,
                viewport_tex_id,
                window,
                app_state: &mut app_state,
                settings,

                // normalize,
                fly_cam: &mut fly_cam,
                orbit_cam: &mut orbit_cam,
                topdown_cam: (&mut topdown_cam.0, &mut topdown_cam.1),

                ev_camera_mode_changed: &mut ev_camera_mode_changed,
                ev_kmp_visibility_updated: &mut ev_kmp_visibility_updated,
            },
        );
    if settings.reset_tree {
        *tree = DockTree::default();
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
                        let settings_string = serde_json::to_string_pretty(settings)
                            .expect("could not convert settings to json");
                        let mut file =
                            File::create(file).expect("could not create user settings file");
                        file.write_all(settings_string.as_bytes())
                            .expect("could not write to user settings file");
                    }
                    DialogType::ImportSettings => {
                        let input_settings_string =
                            read_to_string(file).expect("could not read user settings to string");
                        if let Ok(input_settings) =
                            serde_json::from_str::<AppSettings>(&input_settings_string)
                        {
                            *settings = input_settings;
                        }
                    }
                }
            }
        }
    }

    app_state.kmp_file_path = kmp_file_path;
}
