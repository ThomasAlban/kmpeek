use std::io;

use bevy::prelude::*;
use serde::Serialize;

use super::{
    ordering::OrderID,
    sections::{add_for_all_components, KmpEditMode, ToKmpSection},
    AreaPoint, BattleFinishPoint, CannonPoint, Checkpoint, EnemyPathPoint, ItemPathPoint, KmpCamera, Object,
    RespawnPoint, StartPoint,
};

pub fn csv_plugin(app: &mut App) {
    app.add_event::<CsvExport>()
        .add_systems(Update, add_for_all_components!(csv_export));
}

#[derive(Event)]
pub struct CsvExport;

pub fn csv_export<T: Component + ToKmpSection + Serialize>(
    mut ev_csv_export: EventReader<CsvExport>,
    mode: Option<Res<KmpEditMode<T>>>,
    q: Query<(&OrderID, &T)>,
) {
    if mode.is_none() || ev_csv_export.is_empty() {
        return;
    }
    ev_csv_export.clear();

    let mut wtr = csv::Writer::from_writer(io::stdout());

    let mut items: Vec<_> = q.iter().collect();
    items.sort_by(|x, y| x.0.cmp(y.0));
    let items: Vec<_> = items.iter().map(|x| x.1).collect();

    for item in items.iter() {
        let res = wtr.serialize(item);
        if res.is_err() {
            break;
        }
    }
    wtr.flush().unwrap();
}
