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

pub fn setup_app_settings(mut commands: Commands, pkv: Res<PkvStore>) {
    let settings = match pkv.get::<AppSettings>("settings") {
        Ok(settings) => settings,
        Err(error) => {
            warn!("could not load saved application settings; using defaults for this session: {error}");
            AppSettings::default()
        }
    };

    commands.insert_resource(settings);
}

pub fn export_import_app_settings(
    mut ev_file_dialog: MessageReader<FileDialogResult>,
    mut settings: ResMut<AppSettings>,
) {
    for FileDialogResult { path, dialog_type } in ev_file_dialog.read() {
        match dialog_type {
            DialogType::ImportSettings => match read_to_string(path) {
                Ok(input_settings_string) => match serde_json::from_str::<AppSettings>(&input_settings_string) {
                    Ok(input_settings) => *settings = input_settings,
                    Err(error) => error!("could not parse settings file {}: {error}", path.display()),
                },
                Err(error) => error!("could not read settings file {}: {error}", path.display()),
            },
            DialogType::ExportSettings => match serde_json::to_string_pretty(settings.as_ref()) {
                Ok(settings_string) => match File::create(path) {
                    Ok(mut file) => {
                        if let Err(error) = file.write_all(settings_string.as_bytes()) {
                            error!("could not write settings file {}: {error}", path.display());
                        }
                    }
                    Err(error) => error!("could not create settings file {}: {error}", path.display()),
                },
                Err(error) => error!("could not serialize application settings: {error}"),
            },
            _ => {}
        }
    }
}
