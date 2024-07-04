use std::{
    fs::{read_to_string, File},
    io::Write,
};

use crate::viewer::{camera::CameraSettings, kcl_model::KclModelSettings, kmp::settings::KmpModelSettings};
use bevy::prelude::*;
use bevy_pkv::PkvStore;
use serde::{Deserialize, Serialize};

use super::file_dialog::{DialogType, FileDialogResult};

pub fn app_settings_plugin(app: &mut App) {
    app.add_systems(Startup, setup_app_settings.in_set(SetupAppSettingsSet))
        .add_systems(Update, export_import_app_settings);
}

#[derive(Serialize, Deserialize, Resource)]
pub struct AppSettings {
    pub camera: CameraSettings,
    pub kcl_model: KclModelSettings,
    pub kmp_model: KmpModelSettings,
    pub open_course_kcl_in_dir: bool,
    pub increment: u32,
}
impl Default for AppSettings {
    fn default() -> Self {
        Self {
            camera: CameraSettings::default(),
            kcl_model: KclModelSettings::default(),
            kmp_model: KmpModelSettings::default(),
            open_course_kcl_in_dir: true,
            increment: 1,
        }
    }
}

#[derive(SystemSet, Hash, PartialEq, Eq, Clone, Debug)]
pub struct SetupAppSettingsSet;

pub fn setup_app_settings(mut commands: Commands, mut pkv: ResMut<PkvStore>) {
    // get the app settings if it exists, if not, set it to default
    // THIS LINE SHOULD BE REMOVED TO MAKE THIS ACTUALLY WORK
    pkv.set("settings", &AppSettings::default()).unwrap();
    let settings = match pkv.get::<AppSettings>("settings") {
        Ok(settings) => settings,
        Err(_) => {
            pkv.set("settings", &AppSettings::default()).unwrap();
            AppSettings::default()
        }
    };

    commands.insert_resource(settings);
}

pub fn export_import_app_settings(
    mut ev_file_dialog: EventReader<FileDialogResult>,
    mut settings: ResMut<AppSettings>,
) {
    for FileDialogResult { path, dialog_type } in ev_file_dialog.read() {
        match dialog_type {
            DialogType::ImportSettings => {
                let input_settings_string = read_to_string(path).expect("could not read user settings to string");
                if let Ok(input_settings) = serde_json::from_str::<AppSettings>(&input_settings_string) {
                    *settings = input_settings;
                }
            }
            DialogType::ExportSettings => {
                let settings_string =
                    serde_json::to_string_pretty(settings.as_ref()).expect("could not convert settings to json");
                let mut file = File::create(path).expect("could not create user settings file");
                file.write_all(settings_string.as_bytes())
                    .expect("could not write to user settings file");
            }
            _ => {}
        }
    }
}
