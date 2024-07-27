use super::util::get_egui_ctx;
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::{egui::Align2, EguiContexts};
use egui_file::FileDialog;
use std::path::PathBuf;

pub fn file_dialog_plugin(app: &mut App) {
    app.init_resource::<FileDialogRes>().add_event::<FileDialogResult>();
}

#[derive(Resource, Default)]
pub struct FileDialogRes(pub Option<(FileDialog, DialogType)>);

#[derive(Clone, Copy)]
pub enum DialogType {
    OpenKmpKcl,
    ExportSettings,
    ImportSettings,
    // ExportCsv,
    // ImportCsv,
}

#[derive(Event)]
pub struct FileDialogResult {
    pub path: PathBuf,
    pub dialog_type: DialogType,
}

const FILE_DIALOG_SIZE: (f32, f32) = (500., 250.);

pub fn show_file_dialog(world: &mut World) {
    let ctx = &get_egui_ctx(world);

    world.resource_scope(|world, mut file_dialog: Mut<FileDialogRes>| {
        if let Some((dialog, dialog_type)) = &mut file_dialog.0 {
            let dialog_type = *dialog_type;
            if dialog.show(ctx).selected() {
                if let Some(path) = dialog.path() {
                    world.send_event(FileDialogResult {
                        path: path.into(),
                        dialog_type,
                    });
                }
            }
        }
    });
}

// #[derive(SystemParam)]
// pub struct ShowFileDialog<'w, 's> {
//     contexts: EguiContexts<'w, 's>,
//     file_dialog: ResMut<'w, FileDialogRes>,
//     ev_file_dialog_result: EventWriter<'w, FileDialogResult>,
// }
// impl UiSection for ShowFileDialog<'_, '_> {
//     fn show(&mut self) {
//         let ctx = self.contexts.ctx_mut();
//         if let Some((dialog, dialog_type)) = &mut self.file_dialog.0 {
//             if dialog.show(ctx).selected() {
//                 if let Some(path) = dialog.path() {
//                     self.ev_file_dialog_result.send(FileDialogResult {
//                         path: path.into(),
//                         dialog_type: *dialog_type,
//                     });
//                 }
//             }
//         }
//     }
// }

#[derive(SystemParam)]
pub struct FileDialogManager<'w> {
    file_dialog: ResMut<'w, FileDialogRes>,
}

impl FileDialogManager<'_> {
    pub fn is_open(&self) -> bool {
        self.file_dialog.0.is_some()
    }
    pub fn close(&mut self) {
        self.file_dialog.0 = None;
    }
    pub fn open_kmp_kcl(&mut self) {
        let mut dialog = FileDialog::open_file(None)
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
        self.file_dialog.0 = Some((dialog, DialogType::OpenKmpKcl));
    }
    pub fn import_settings(&mut self) {
        let mut dialog = FileDialog::open_file(None)
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
        self.file_dialog.0 = Some((dialog, DialogType::ImportSettings));
    }
    pub fn export_settings(&mut self) {
        let mut dialog = FileDialog::save_file(None)
            .default_size(FILE_DIALOG_SIZE)
            .anchor(Align2::CENTER_CENTER, [0., 0.])
            .default_filename("kmpeek_settings.json");
        dialog.open();

        self.file_dialog.0 = Some((dialog, DialogType::ExportSettings));
    }
    // pub fn export_csv(&mut self, name: impl Into<String>) {
    //     let mut dialog = FileDialog::save_file(None)
    //         .default_size(FILE_DIALOG_SIZE)
    //         .anchor(Align2::CENTER_CENTER, [0., 0.])
    //         .default_filename(name.into());
    //     dialog.open();

    //     self.file_dialog.0 = Some((dialog, DialogType::ExportCsv));
    // }
    // pub fn import_csv(&mut self) {
    //     let mut dialog = FileDialog::open_file(None)
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
