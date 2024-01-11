use super::{
    settings::AppSettings,
    ui_state::{FileDialogRes, KmpFilePath},
    update_ui::{KclFileSelected, KmpFileSelected, UiSection},
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::EguiContexts;
use egui_file::FileDialog;
use std::{
    fs::{read_to_string, File},
    io::Write,
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
    settings: ResMut<'w, AppSettings>,
    file_dialog: ResMut<'w, FileDialogRes>,
    kmp_file_path: ResMut<'w, KmpFilePath>,
}
impl UiSection for ShowFileDialog<'_, '_> {
    fn show(&mut self) {
        let ctx = self.contexts.ctx_mut();

        if let Some(dialog) = &mut self.file_dialog.0 {
            if dialog.0.show(ctx).selected() {
                if let Some(file) = dialog.0.path() {
                    match dialog.1 {
                        DialogType::OpenKmpKcl => {
                            if let Some(file_ext) = file.extension() {
                                if file_ext == "kmp" {
                                    self.kmp_file_path.0 = Some(file.into());
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
    }
}
impl ShowFileDialog<'_, '_> {
    pub fn open_kmp_kcl(file_dialog: &mut FileDialogRes) {
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
        file_dialog.0 = Some((dialog, DialogType::OpenKmpKcl));
    }
    pub fn import_settings(file_dialog: &mut FileDialogRes) {
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
        file_dialog.0 = Some((dialog, DialogType::ImportSettings));
    }
    pub fn export_settings(file_dialog: &mut FileDialogRes) {
        let mut dialog = FileDialog::save_file(None)
            .default_size(FILE_DIALOG_SIZE)
            .default_filename("kmpeek_settings.json");
        dialog.open();

        file_dialog.0 = Some((dialog, DialogType::ExportSettings));
    }
    pub fn close(file_dialog: &mut FileDialogRes) {
        file_dialog.0 = None;
    }
}
