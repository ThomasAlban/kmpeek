use crate::{
    ui::file_dialog::DialogType,
    ui::update_ui::{KclFileSelected, KmpFileSelected},
    viewer::kcl_model::KclModelSettings,
    viewer::{
        camera::{CameraModeChanged, CameraSettings},
        kmp::settings::KmpModelSettings,
    },
};
use bevy::{
    app::AppExit,
    prelude::*,
    window::{exit_on_all_closed, exit_on_primary_closed},
};
use bevy_pkv::PkvStore;
use egui_file::*;
use serde::{Deserialize, Serialize};
use std::{
    env,
    path::{Path, PathBuf},
};
use strum_macros::{Display, EnumIter, EnumString, IntoStaticStr};

use super::tabs::DockTree;

pub struct AppStatePlugin;
impl Plugin for AppStatePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PkvStore::new("ThomasAlban", "kmpeek"))
            .add_event::<AppModeChanged>()
            .add_systems(Startup, setup_app_state)
            .add_systems(
                PostUpdate,
                on_app_exit
                    .after(exit_on_primary_closed)
                    .after(exit_on_all_closed)
                    .run_if(on_event::<AppExit>()),
            );
    }
}

#[derive(Resource)]
pub struct AppState {
    pub mode: AppMode,
    pub customise_kcl_open: bool,
    pub camera_settings_open: bool,

    pub file_dialog: Option<(FileDialog, DialogType)>,

    pub kmp_file_path: Option<PathBuf>,
    pub mouse_in_viewport: bool,
    pub viewport_rect: Rect,
    pub show_modes_collapsed: Option<f32>,
}
impl Default for AppState {
    fn default() -> Self {
        AppState {
            mode: AppMode::TrackInfo,
            customise_kcl_open: false,
            camera_settings_open: false,
            file_dialog: None,
            kmp_file_path: None,
            mouse_in_viewport: false,
            viewport_rect: Rect::from_corners(Vec2::ZERO, Vec2::ZERO),
            show_modes_collapsed: None,
        }
    }
}

#[derive(Display, EnumString, IntoStaticStr, EnumIter, PartialEq, Clone, Copy)]
pub enum AppMode {
    #[strum(serialize = "Track Info")]
    TrackInfo,
    #[strum(serialize = "Start/Finish Points")]
    StartFinishPoints,
    Paths,
    #[strum(serialize = "Checkpoints & Respawns")]
    CheckpointsRespawns,
    Objects,
    Cameras,
    #[strum(serialize = "Routes & Areas")]
    RoutesAreas,
    #[strum(serialize = "Free Edit")]
    FreeEdit,
}

#[derive(Event, Default)]
pub struct AppModeChanged;

#[derive(Serialize, Deserialize, Resource)]
pub struct AppSettings {
    pub camera: CameraSettings,
    pub kcl_model: KclModelSettings,
    pub kmp_model: KmpModelSettings,
    pub open_course_kcl_in_directory: bool,
    pub reset_tree: bool,
    pub increment: u32,
}
impl Default for AppSettings {
    fn default() -> Self {
        Self {
            camera: CameraSettings::default(),
            kcl_model: KclModelSettings::default(),
            kmp_model: KmpModelSettings::default(),
            open_course_kcl_in_directory: true,
            reset_tree: false,
            increment: 1,
        }
    }
}

pub fn setup_app_state(
    mut commands: Commands,
    mut pkv: ResMut<PkvStore>,
    mut ev_kmp_file_selected: EventWriter<KmpFileSelected>,
    mut ev_kcl_file_selected: EventWriter<KclFileSelected>,
    mut ev_camera_mode_changed: EventWriter<CameraModeChanged>,
) {
    // create the default app state
    let mut app_state = AppState::default();

    // get the app settings if it exists, if not, set it to default
    let settings = match pkv.get::<AppSettings>("settings") {
        Ok(settings) => settings,
        Err(_) => {
            pkv.set("settings", &AppSettings::default()).unwrap();
            AppSettings::default()
        }
    };

    // change the camera mode to whatever the settings say it needs to be
    ev_camera_mode_changed.send(CameraModeChanged(settings.camera.mode));

    // if there is a command line arg of a path to a kmp or kcl, open it
    let args: Vec<String> = env::args().collect();
    let mut kmp_file_path: Option<PathBuf> = None;
    if let Some(arg) = args.get(1) {
        let path = Path::new(arg);
        if path.is_file() {
            if let Some(file_ext) = path.extension() {
                // if the file is a kmp file
                if file_ext == "kmp" {
                    // open it
                    kmp_file_path = Some(path.into());
                    ev_kmp_file_selected.send(KmpFileSelected(path.into()));
                    // if there is a course.kcl in the same directory and the setting to open it is set, open the kcl as well
                    if settings.open_course_kcl_in_directory {
                        let mut course_kcl_path = path.to_owned();
                        course_kcl_path.set_file_name("course.kcl");
                        if course_kcl_path.exists() {
                            ev_kcl_file_selected.send(KclFileSelected(course_kcl_path));
                        }
                    }
                // else if the file is a kcl file, open it
                } else if file_ext == "kcl" {
                    ev_kcl_file_selected.send(KclFileSelected(path.into()));
                }
            }
        }
    }
    app_state.kmp_file_path = kmp_file_path;

    commands.insert_resource(app_state);
    commands.insert_resource(settings);
}

fn on_app_exit(settings: Res<AppSettings>, tree: Res<DockTree>, mut pkv: ResMut<PkvStore>) {
    // save the user settings
    pkv.set("settings", settings.as_ref()).unwrap();
    // save the dock tree
    pkv.set("tree", tree.as_ref()).unwrap();
}
