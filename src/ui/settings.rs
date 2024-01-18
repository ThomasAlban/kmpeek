use crate::viewer::{
    camera::CameraSettings, kcl_model::KclModelSettings, kmp::settings::KmpModelSettings,
};
use bevy::prelude::*;
use bevy_pkv::PkvStore;
use serde::{Deserialize, Serialize};

pub struct AppSettingsPlugin;
impl Plugin for AppSettingsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_app_settings.in_set(SetupAppSettingsSet));
    }
}

#[derive(Serialize, Deserialize, Resource)]
pub struct AppSettings {
    pub camera: CameraSettings,
    pub kcl_model: KclModelSettings,
    pub kmp_model: KmpModelSettings,
    pub open_course_kcl_in_directory: bool,
    pub reset_tree: bool,
    pub increment: u32,
    pub view_multiple: bool,
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
            view_multiple: false,
        }
    }
}

#[derive(SystemSet, Hash, PartialEq, Eq, Clone, Debug)]
pub struct SetupAppSettingsSet;

pub fn setup_app_settings(mut commands: Commands, mut pkv: ResMut<PkvStore>) {
    // get the app settings if it exists, if not, set it to default
    let settings = match pkv.get::<AppSettings>("settings") {
        Ok(settings) => settings,
        Err(_) => {
            pkv.set("settings", &AppSettings::default()).unwrap();
            AppSettings::default()
        }
    };

    commands.insert_resource(settings);
}
