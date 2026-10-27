use bevy::app::{App, Startup, Update};
use bevy::log::{error, warn};
use bevy::prelude::{on_message, Commands, IntoScheduleConfigs, MessageReader, Res, ResMut, Resource};
use bevy_pkv::PkvStore;
use serde::{Deserialize, Serialize};
use crate::ui::util::{FileResult};

pub fn last_opened_plugin(app: &mut App) {
    app.add_systems(Startup, setup_last_opened)
        .add_message::<FileResult>()
        .add_systems(Update, add_last_opened_path.run_if(on_message::<FileResult>));
}

fn setup_last_opened(mut commands: Commands, pkv: Res<PkvStore>) {
    let last_opened = pkv.get::<LastOpened>("last_opened").unwrap_or_else(|error| {
        warn!("could not load recently open files; using defaults for this session: {error}");
        LastOpened::default()
    });
    commands.insert_resource(last_opened);
}

pub const LAST_OPENED_SIZE: usize = 10;

#[derive(Serialize, Deserialize, Resource)]
pub struct LastOpened {
    pub entries: [FileResult; LAST_OPENED_SIZE]
}

impl LastOpened {
    pub fn most_recent(&self) -> Option<&FileResult> {
        self.entries.first()
    }
}

impl Default for LastOpened {
    fn default() -> Self {
        Self {
            entries: Default::default()
        }
    }
}

pub fn add_last_opened_path(mut pkv: ResMut<PkvStore>, mut last_opened: ResMut<LastOpened>, mut reader: MessageReader<FileResult>) {
    let mut new_entry_read = None;
    for msg in reader.read() {
        new_entry_read = Some(msg);
    }

    match new_entry_read {
        None => {
            error!("No FileResult passed to function.")
        }
        Some(file_result) if file_result.path.as_os_str().is_empty() => {
            error!("Passed path is empty, cannot add to recently opened.")
        }
        Some(file_result) => {
            match last_opened.entries.clone().into_iter().position(|entry| entry.path == file_result.path) {
                Some(0) => return,
                Some(index) => {
                    for i in index..LAST_OPENED_SIZE - 1 {
                        last_opened.entries[i] = last_opened.entries[i + 1].clone();
                    }
                },
                None => {}
            }

            last_opened.entries[LAST_OPENED_SIZE - 1] = file_result.clone();
            last_opened.entries.rotate_right(1);

            if let Err(error) = pkv.set("last_opened", last_opened.as_ref()) {
                error!("could not save recently opened files: {error}");
            }
        }
    }
}
