use std::{
    fs::{read_to_string, File},
    io::Write,
};

use crate::viewer::{camera::CameraSettings, kcl_model::KclModelSettings, kmp::settings::KmpModelSettings};
use bevy::prelude::*;
use bevy_pkv::PkvStore;
use serde::{Deserialize, Serialize};

use super::{
    file_dialog::{DialogType, FileDialogResult},
    keybinds::EditorKeyBindings,
};

pub fn app_settings_plugin(app: &mut App) {
    app.add_systems(Startup, setup_app_settings.in_set(SetupAppSettingsSet))
        .add_systems(Update, export_import_app_settings);
}

#[derive(Serialize, Deserialize, Resource)]
pub struct AppSettings {
    pub camera: CameraSettings,
    pub kcl_model: KclModelSettings,
    pub kmp_model: KmpModelSettings,
    #[serde(default)]
    pub editor_key_bindings: EditorKeyBindings,
    pub open_course_kcl_in_dir: bool,
    #[serde(default)]
    pub preserve_visibility_on_section_select: bool,
    /// Patch mode is an opt-in preservation tool; normal saves rebuild editor data.
    #[serde(default)]
    pub patch_saving: bool,
    pub increment: u32,
}
impl Default for AppSettings {
    fn default() -> Self {
        Self {
            camera: CameraSettings::default(),
            kcl_model: KclModelSettings::default(),
            kmp_model: KmpModelSettings::default(),
            editor_key_bindings: EditorKeyBindings::default(),
            open_course_kcl_in_dir: true,
            preserve_visibility_on_section_select: false,
            patch_saving: false,
            increment: 1,
        }
    }
}

/// Keep the rebuild warning available where the user chooses the save mode.
pub const REBUILD_SAVING_WARNING: &str =
    "Data-loss warning: Patch saving is OFF. Saving rebuilds the KMP, discards unused/unsupported data, and renumbers indices. Cannon reordering also requires updating external KCL triggers. Use Save As to keep a separate copy.";

#[cfg(test)]
mod tests {
    use super::AppSettings;

    /// New installations use canonical rebuild saving; patching is opt-in.
    #[test]
    fn patch_saving_defaults_off() {
        assert!(!AppSettings::default().patch_saving);
    }

    /// Older settings omit this field and therefore use the current default.
    #[test]
    fn missing_patch_saving_defaults_off() {
        let settings = AppSettings {
            increment: 7,
            open_course_kcl_in_dir: false,
            ..Default::default()
        };
        let mut json = serde_json::to_value(settings).unwrap();
        json.as_object_mut().unwrap().remove("patch_saving");
        let restored: AppSettings = serde_json::from_value(json).unwrap();
        assert!(!restored.patch_saving);
        assert_eq!(restored.increment, 7);
        assert!(!restored.open_course_kcl_in_dir);
    }

    #[test]
    fn preserved_section_visibility_defaults_off_for_existing_settings() {
        let mut json = serde_json::to_value(AppSettings::default()).unwrap();
        json.as_object_mut()
            .unwrap()
            .remove("preserve_visibility_on_section_select");

        let restored: AppSettings = serde_json::from_value(json).unwrap();

        assert!(!restored.preserve_visibility_on_section_select);
    }

    #[test]
    fn missing_editor_key_bindings_use_defaults() {
        let mut json = serde_json::to_value(AppSettings::default()).unwrap();
        json.as_object_mut().unwrap().remove("editor_key_bindings");

        let restored: AppSettings = serde_json::from_value(json).unwrap();

        assert!(!restored.editor_key_bindings.transform.is_empty());
        assert!(!restored.editor_key_bindings.scale.is_empty());

        let mut json = serde_json::to_value(AppSettings::default()).unwrap();
        json["editor_key_bindings"]
            .as_object_mut()
            .unwrap()
            .remove("default_mode");
        let restored: AppSettings = serde_json::from_value(json).unwrap();
        assert!(!restored.editor_key_bindings.default_mode.is_empty());
    }

    /// Export/import must retain an explicit opt-out as well as the enabled value.
    #[test]
    fn patch_saving_round_trips_both_values() {
        for patch_saving in [false, true] {
            let settings = AppSettings {
                patch_saving,
                ..Default::default()
            };
            let json = serde_json::to_string_pretty(&settings).unwrap();
            let restored: AppSettings = serde_json::from_str(&json).unwrap();
            assert_eq!(restored.patch_saving, patch_saving);
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
