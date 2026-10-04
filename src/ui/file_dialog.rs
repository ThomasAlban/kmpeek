use super::{ui_state::KmpFilePath, util::get_egui_ctx, util::FileResult, util::FileSource};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::Align2;
use egui_file::{FileDialog, State as FileDialogState};

pub fn file_dialog_plugin(app: &mut App) {
    app.init_resource::<FileDialogRes>().add_message::<FileResult>();
}

#[derive(Resource, Default)]
pub struct FileDialogRes(pub Option<(FileDialog, FileSource)>);

const FILE_DIALOG_SIZE: (f32, f32) = (500., 250.);

pub fn show_file_dialog(world: &mut World) {
    let ctx = &get_egui_ctx(world);

    world.resource_scope(|world, mut file_dialog: Mut<FileDialogRes>| {
        let mut result = None;
        let mut close_dialog = false;

        if let Some((dialog, file_source)) = &mut file_dialog.0 {
            dialog.show(ctx);
            match dialog.state() {
                FileDialogState::Selected => {
                    result = dialog.path().map(|path| FileResult {
                        path: path.into(),
                        file_source: *file_source,
                    });
                    close_dialog = true;
                }
                FileDialogState::Cancelled | FileDialogState::Closed => {
                    close_dialog = true;
                }
                FileDialogState::Open => {}
            }
        }

        if close_dialog {
            file_dialog.0 = None;
            // The dialog can close during the first egui pass. Rerun the UI so
            // its transitional closing pass is never presented to the user.
            ctx.request_discard("file dialog closed");
        }
        if let Some(result) = result {
            world.write_message(result);
        }
    });
}

#[derive(SystemParam)]
pub struct FileDialogManager<'w> {
    file_dialog: ResMut<'w, FileDialogRes>,
    // Optional because the dialog manager also handles opening the first course.
    kmp_path: Option<Res<'w, KmpFilePath>>,
}

impl FileDialogManager<'_> {
    pub fn is_open(&self) -> bool {
        self.file_dialog.0.is_some()
    }
    pub fn close(&mut self) {
        self.file_dialog.0 = None;
    }
    pub fn open_kmp_kcl(&mut self) {
        let mut dialog = FileDialog::open_file()
            .default_size(FILE_DIALOG_SIZE)
            .anchor(Align2::CENTER_CENTER, [0., 0.])
            .show_files_filter(Box::new(move |path| {
                if let Some(os_str) = path.extension() {
                    if let Some(str) = os_str.to_str() {
                        return ["kcl", "kmp"].contains(&str);
                    }
                }
                false
            }));
        dialog.open();
        self.file_dialog.0 = Some((dialog, FileSource::OpenKmpKclDialog));
    }
    /// Pick a destination only; the document service selects patch/rebuild mode
    /// and performs the actual atomic save after the dialog result is delivered.
    pub fn save_kmp(&mut self) {
        let mut dialog = FileDialog::save_file()
            .default_size(FILE_DIALOG_SIZE)
            .anchor(Align2::CENTER_CENTER, [0., 0.])
            .default_filename("course.kmp");
        if let Some(path) = self.kmp_path.as_ref().and_then(|path| path.0.parent()) {
            dialog = dialog.initial_path(path);
        }
        dialog.open();
        self.file_dialog.0 = Some((dialog, FileSource::SaveKmpDialog));
    }
    pub fn import_settings(&mut self) {
        let mut dialog = FileDialog::open_file()
            .default_size(FILE_DIALOG_SIZE)
            .anchor(Align2::CENTER_CENTER, [0., 0.])
            .show_files_filter(Box::new(|path| {
                if let Some(os_str) = path.extension() {
                    if let Some(str) = os_str.to_str() {
                        return str == "json";
                    }
                }
                false
            }));
        dialog.open();
        self.file_dialog.0 = Some((dialog, FileSource::ImportSettings));
    }
    pub fn export_settings(&mut self) {
        let mut dialog = FileDialog::save_file()
            .default_size(FILE_DIALOG_SIZE)
            .anchor(Align2::CENTER_CENTER, [0., 0.])
            .default_filename("kmpeek_settings.json");
        dialog.open();

        self.file_dialog.0 = Some((dialog, FileSource::ExportSettings));
    }
    // pub fn export_csv(&mut self, name: impl Into<String>) {
    //     let mut dialog = FileDialog::save_file()
    //         .default_size(FILE_DIALOG_SIZE)
    //         .anchor(Align2::CENTER_CENTER, [0., 0.])
    //         .default_filename(name.into());
    //     dialog.open();

    //     self.file_dialog.0 = Some((dialog, DialogType::ExportCsv));
    // }
    // pub fn import_csv(&mut self) {
    //     let mut dialog = FileDialog::open_file()
    //         .default_size(FILE_DIALOG_SIZE)
    //         .anchor(Align2::CENTER_CENTER, [0., 0.])
    //         .show_files_filter(Box::new(|path| {
    //             if let Some(os_str) = path.extension() {
    //                 if let Some(str) = os_str.to_str() {
    //                     return str == "csv";
    //                 }
    //             }
    //             false
    //         }));
    //     dialog.open();
    //     self.file_dialog.0 = Some((dialog, DialogType::ImportCsv));
    // }
}
