use super::settings::{AppSettings, SetupAppSettingsSet};
use crate::{
    ui::file_dialog::DialogType,
    ui::update_ui::{KclFileSelected, KmpFileSelected},
};
use bevy::prelude::*;
use bevy_pkv::PkvStore;
use egui_file::*;
use serde::{Deserialize, Serialize};
use std::{
    env,
    path::{Path, PathBuf},
};
use strum_macros::{Display, EnumIter, EnumString, IntoStaticStr};

pub struct UiStatePlugin;
impl Plugin for UiStatePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PkvStore::new("ThomasAlban", "kmpeek"))
            .insert_resource(CustomiseKclOpen(false))
            .insert_resource(CameraSettingsOpen(false))
            .insert_resource(FileDialogRes(None))
            .insert_resource(KmpFilePath(None))
            .insert_resource(MouseInViewport(false))
            .insert_resource(ViewportRect(Rect::from_corners(Vec2::ZERO, Vec2::ZERO)))
            .insert_resource(ShowModesCollapsed(None))
            .insert_resource(AppMode::TrackInfo)
            .add_event::<AppModeChanged>()
            .add_systems(
                Startup,
                (apply_deferred, check_cmd_args)
                    .chain()
                    .after(SetupAppSettingsSet),
            );
    }
}

#[derive(Resource)]
pub struct CustomiseKclOpen(pub bool);
#[derive(Resource)]
pub struct CameraSettingsOpen(pub bool);
#[derive(Resource)]
pub struct FileDialogRes(pub Option<(FileDialog, DialogType)>);
#[derive(Resource)]
pub struct KmpFilePath(pub Option<PathBuf>);
#[derive(Resource)]
pub struct MouseInViewport(pub bool);
#[derive(Resource)]
pub struct ViewportRect(pub Rect);
#[derive(Resource)]
pub struct ShowModesCollapsed(pub Option<f32>);

#[derive(Display, EnumString, IntoStaticStr, EnumIter, PartialEq, Clone, Copy, Resource)]
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

#[derive(Serialize, Deserialize, Resource, Deref, DerefMut)]
pub struct Increment(pub u32);

pub fn check_cmd_args(
    mut ev_kmp_file_selected: EventWriter<KmpFileSelected>,
    mut ev_kcl_file_selected: EventWriter<KclFileSelected>,
    mut kmp_file_path: ResMut<KmpFilePath>,
    settings: Res<AppSettings>,
) {
    // if there is a command line arg of a path to a kmp or kcl, open it
    let args: Vec<String> = env::args().collect();
    if let Some(arg) = args.get(1) {
        let path = Path::new(arg);
        if path.is_file() {
            if let Some(file_ext) = path.extension() {
                // if the file is a kmp file
                if file_ext == "kmp" {
                    // open it
                    kmp_file_path.0 = Some(path.into());
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
}
