use super::{
    app_state::{AppSettings, AppState},
    update_ui::{KclFileSelected, KmpFileSelected, UiSection},
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::EguiContexts;
use egui_file::FileDialog;
use std::{
    fs::{read_to_string, File},
    io::Write,
    path::PathBuf,
};

pub enum DialogType {
    OpenKmpKcl,
    ExportSettings,
    ImportSettings,
}

const FILE_DIALOG_SIZE: (f32, f32) = (500., 250.);

#[derive(SystemParam)]
pub struct ShowFileDialog<'w, 's> {
    contexts: EguiContexts<'w, 's>,
    ev_kmp_file_selected: EventWriter<'w, KmpFileSelected>,
    ev_kcl_file_selected: EventWriter<'w, KclFileSelected>,
    app_state: ResMut<'w, AppState>,
    settings: ResMut<'w, AppSettings>,
}
impl UiSection for ShowFileDialog<'_, '_> {
    fn show(&mut self) {
        let ctx = self.contexts.ctx_mut();

        let mut kmp_file_path: Option<PathBuf> = None;
        if let Some(dialog) = &mut self.app_state.file_dialog {
            if dialog.0.show(ctx).selected() {
                if let Some(file) = dialog.0.path() {
                    match dialog.1 {
                        DialogType::OpenKmpKcl => {
                            if let Some(file_ext) = file.extension() {
                                if file_ext == "kmp" {
                                    kmp_file_path = Some(file.into());
                                    self.ev_kmp_file_selected.send(KmpFileSelected(file.into()));
                                    if self.settings.open_course_kcl_in_directory {
                                        let mut course_kcl_path = file.to_owned();
                                        course_kcl_path.set_file_name("course.kcl");
                                        if course_kcl_path.exists() {
                                            self.ev_kcl_file_selected
                                                .send(KclFileSelected(course_kcl_path));
                                        }
                                    }
                                } else if file_ext == "kcl" {
                                    self.ev_kcl_file_selected.send(KclFileSelected(file.into()));
                                }
                            }
                        }
                        DialogType::ExportSettings => {
                            let settings_string =
                                serde_json::to_string_pretty(self.settings.as_ref())
                                    .expect("could not convert settings to json");
                            let mut file =
                                File::create(file).expect("could not create user settings file");
                            file.write_all(settings_string.as_bytes())
                                .expect("could not write to user settings file");
                        }
                        DialogType::ImportSettings => {
                            let input_settings_string = read_to_string(file)
                                .expect("could not read user settings to string");
                            if let Ok(input_settings) =
                                serde_json::from_str::<AppSettings>(&input_settings_string)
                            {
                                *self.settings = input_settings;
                            }
                        }
                    }
                }
            }
        }

        self.app_state.kmp_file_path = kmp_file_path;
    }
}
impl ShowFileDialog<'_, '_> {
    pub fn open_kmp_kcl(app_state: &mut AppState) {
        let mut dialog = FileDialog::open_file(None)
            .default_size(FILE_DIALOG_SIZE)
            .show_files_filter(Box::new(move |path| {
                if let Some(os_str) = path.extension() {
                    if let Some(str) = os_str.to_str() {
                        return ["kcl", "kmp"].contains(&str);
                    }
                }
                false
            }));
        dialog.open();
        app_state.file_dialog = Some((dialog, DialogType::OpenKmpKcl));
    }
    pub fn import_settings(app_state: &mut AppState) {
        let mut dialog = FileDialog::open_file(None)
            .default_size(FILE_DIALOG_SIZE)
            .show_files_filter(Box::new(|path| {
                if let Some(os_str) = path.extension() {
                    if let Some(str) = os_str.to_str() {
                        return str == "json";
                    }
                }
                false
            }));
        dialog.open();
        app_state.file_dialog = Some((dialog, DialogType::ImportSettings));
    }
    pub fn export_settings(app_state: &mut AppState) {
        let mut dialog = FileDialog::save_file(None)
            .default_size(FILE_DIALOG_SIZE)
            .default_filename("kmpeek_settings.json");
        dialog.open();

        app_state.file_dialog = Some((dialog, DialogType::ExportSettings));
    }
    pub fn close(app_state: &mut AppState) {
        app_state.file_dialog = None;
    }
}
