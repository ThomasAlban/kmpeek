use super::tabs::DockTree;
use crate::viewer::{
    camera::CameraSettings, kcl_model::KclModelSettings, kmp::settings::KmpModelSettings,
};
use bevy::{
    app::AppExit,
    prelude::*,
    window::{exit_on_all_closed, exit_on_primary_closed},
};
use bevy_pkv::PkvStore;
use serde::{Deserialize, Serialize};

pub struct AppSettingsPlugin;
impl Plugin for AppSettingsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_app_settings.in_set(SetupAppSettingsSet))
            .add_systems(
                PostUpdate,
                on_app_exit
                    .after(exit_on_primary_closed)
                    .after(exit_on_all_closed)
                    .run_if(on_event::<AppExit>()),
            );
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

fn on_app_exit(settings: Res<AppSettings>, tree: Res<DockTree>, mut pkv: ResMut<PkvStore>) {
    // save the user settings
    pkv.set("settings", settings.as_ref()).unwrap();
    // save the dock tree
    pkv.set("tree", tree.as_ref()).unwrap();
}
