use super::{
    settings::{AppSettings, SetupAppSettingsSet},
    tabs::DockTree,
};
use crate::ui::update_ui::{KclFileSelected, KmpFileSelected};
use bevy::prelude::*;
use bevy_pkv::PkvStore;
use serde::{Deserialize, Serialize};
use std::{
    env,
    path::{Path, PathBuf},
};

pub fn ui_state_plugin(app: &mut App) {
    app.insert_resource(PkvStore::new("ThomasAlban", "kmpeek"))
        // .init_resource::<CustomiseKclOpen>()
        // .init_resource::<CameraSettingsOpen>()
        .init_resource::<KmpFilePath>()
        // .init_resource::<ShowModesCollapsed>()
        .init_resource::<KmpVisibility>()
        .add_event::<SaveDockTree>()
        .add_systems(Update, save_docktree.run_if(on_event::<SaveDockTree>()))
        .add_systems(Update, reset_docktree.run_if(on_event::<ResetDockTree>()))
        .add_event::<ResetDockTree>()
        .add_systems(Startup, check_cmd_args.after(SetupAppSettingsSet));
}

#[derive(Event, Default)]
pub struct SaveDockTree;
pub fn save_docktree(mut pkv: ResMut<PkvStore>, tree: Res<DockTree>) {
    pkv.set("tree", tree.as_ref()).unwrap();
}
#[derive(Event, Default)]
pub struct ResetDockTree;
pub fn reset_docktree(mut pkv: ResMut<PkvStore>, mut tree: ResMut<DockTree>) {
    *tree = DockTree::default();
    pkv.set("tree", tree.as_ref()).unwrap();
}

// #[derive(Resource, Default)]
// pub struct CustomiseKclOpen(pub bool);
// #[derive(Resource, Default)]
// pub struct CameraSettingsOpen(pub bool);

#[derive(Resource, Default)]
pub struct KmpFilePath(pub Option<PathBuf>);
// #[derive(Resource, Default)]
// pub struct ShowModesCollapsed(pub Option<f32>);
#[derive(Resource, Default, Clone, PartialEq)]
pub struct KmpVisibility(pub [bool; 10]);

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
                    if settings.open_course_kcl_in_dir {
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
