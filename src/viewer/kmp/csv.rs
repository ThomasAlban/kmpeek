// use std::{arch::aarch64::vaddvq_u8, fmt::Debug};

// use bevy::{math::vec3, prelude::*};
// use serde::{ser::SerializeMap, Deserialize, Serialize};

// use crate::ui::{
//     file_dialog::{DialogType, FileDialogResult},
//     util::{euler_to_quat, quat_to_euler},
// };

// use super::{
//     ordering::OrderID,
//     path::{is_path, KmpPathNode},
//     point::PointSpawner,
//     sections::{add_for_all_components, KmpEditMode, ToKmpSection},
//     AreaPoint, BattleFinishPoint, CannonPoint, Checkpoint, EnemyPathPoint, EnemyPathSetting1, EnemyPathSetting2,
//     ItemPathPoint, KmpCamera, Object, RespawnPoint, Spawn, Spawner, StartPoint,
// };

// pub fn csv_plugin(app: &mut App) {
//     app.add_systems(
//         Update,
//         (csv_export_import::<StartPoint>, csv_export_import::<EnemyPathPoint>),
//     );
// }

// pub fn csv_export_import<T: Component + Clone + Debug + ToFromCsvFormat + Spawn>(
//     mut ev_file_dialog_result: EventReader<FileDialogResult>,
//     mode: Option<Res<KmpEditMode<T>>>,
//     q: Query<(Entity, &OrderID, &T, &Transform)>,
//     mut commands: Commands,
// ) {
//     if mode.is_none() {
//         return;
//     }
//     for FileDialogResult { path, dialog_type } in ev_file_dialog_result.read() {
//         match dialog_type {
//             DialogType::ExportCsv => {
//                 let mut items: Vec<_> = q.iter().collect();
//                 items.sort_by(|x, y| x.1.cmp(y.1));
//                 let items: Vec<_> = items.iter().map(|x| (x.0, x.2.clone(), *x.3)).collect();
//                 let path = path.clone();

//                 commands.add(move |world: &mut World| {
//                     let Ok(mut wtr) = csv::Writer::from_path(path) else {
//                         // send error
//                         return;
//                     };
//                     for (e, t, transform) in items {
//                         let csv_format = t.clone().to_csv_format(transform, e, world);
//                         let res = wtr.serialize(csv_format);
//                         if res.is_err() {
//                             dbg!(res);
//                             // send error
//                             continue;
//                         }
//                     }
//                     if wtr.flush().is_err() {
//                         // send error
//                     }
//                 });
//             }
//             DialogType::ImportCsv => {
//                 let Ok(mut rdr) = csv::Reader::from_path(path) else {
//                     // send error
//                     return;
//                 };
//                 for (entity, _, _, _) in q.iter() {
//                     commands.entity(entity).despawn_recursive();
//                 }
//                 commands.add(move |world: &mut World| {
//                     let mut records = Vec::new();
//                     for (i, result) in rdr.deserialize::<T::CsvFormat>().enumerate() {
//                         let Ok(record) = result else {
//                             // send error
//                             continue;
//                         };
//                         let (component, transform) = T::from_csv_format(record.clone(), world);

//                         let e = Spawner::<T>::new(component)
//                             .transform(transform)
//                             .order_id(i as u32)
//                             .spawn(world);

//                         records.push((record, e));
//                     }
//                     if is_path::<T>() {
//                         for (cur_record, cur_e) in records.iter() {
//                             for i in T::next_points(cur_record).unwrap() {
//                                 let next_e = records[i as usize].1;
//                                 KmpPathNode::link_nodes(*cur_e, next_e, world);
//                             }
//                         }
//                     }
//                 });
//             }
//             _ => {}
//         }
//     }
// }

// pub trait ToFromCsvFormat
// where
//     Self: Sized,
//     Self::CsvFormat: Serialize + Clone,
//     for<'de> Self::CsvFormat: Deserialize<'de>,
// {
//     type CsvFormat;
//     fn to_csv_format(self, transform: Transform, entity: Entity, world: &mut World) -> Self::CsvFormat;
//     fn from_csv_format(value: Self::CsvFormat, world: &mut World) -> (Self, Transform);
//     fn next_points(csv: &Self::CsvFormat) -> Option<Vec<u32>> {
//         None
//     }
// }

// #[derive(Serialize, Deserialize, Clone)]
// pub struct StartPointCsvRow {
//     position_x: f32,
//     position_y: f32,
//     position_z: f32,
//     rotation_x: f32,
//     rotation_y: f32,
//     rotation_z: f32,
//     player_index: i16,
// }
// impl ToFromCsvFormat for StartPoint {
//     type CsvFormat = StartPointCsvRow;
//     fn to_csv_format(self, transform: Transform, _: Entity, _: &mut World) -> Self::CsvFormat {
//         let rotation = quat_to_euler(&transform);
//         StartPointCsvRow {
//             position_x: transform.translation.x,
//             position_y: transform.translation.y,
//             position_z: transform.translation.z,
//             rotation_x: rotation.x,
//             rotation_y: rotation.y,
//             rotation_z: rotation.z,
//             player_index: self.player_index,
//         }
//     }
//     fn from_csv_format(value: Self::CsvFormat, _: &mut World) -> (Self, Transform) {
//         (
//             StartPoint {
//                 player_index: value.player_index,
//             },
//             Transform::from_xyz(value.position_x, value.position_y, value.position_z).with_rotation(Quat::from_euler(
//                 EulerRot::XYZ,
//                 value.rotation_x,
//                 value.rotation_y,
//                 value.rotation_z,
//             )),
//         )
//     }
// }

// #[derive(Serialize, Deserialize, Clone)]
// pub struct EnemyPathCsvRow {
//     position_x: f32,
//     position_y: f32,
//     position_z: f32,
//     leniency: f32,
//     setting_1: EnemyPathSetting1,
//     setting_2: EnemyPathSetting2,
//     setting_3: u8,
//     next_points: Vec<u32>,
// }
// impl ToFromCsvFormat for EnemyPathPoint {
//     type CsvFormat = EnemyPathCsvRow;
//     fn to_csv_format(self, transform: Transform, entity: Entity, world: &mut World) -> Self::CsvFormat {
//         let next_entities = world.entity(entity).get::<KmpPathNode>().unwrap().next_nodes.clone();

//         let mut q = world.query::<&OrderID>();
//         let next_points = q.iter_many(world, next_entities).map(|x| x.0).collect();

//         EnemyPathCsvRow {
//             position_x: transform.translation.x,
//             position_y: transform.translation.y,
//             position_z: transform.translation.z,
//             leniency: self.leniency,
//             setting_1: self.setting_1,
//             setting_2: self.setting_2,
//             setting_3: self.setting_3,
//             next_points,
//         }
//     }
//     fn from_csv_format(value: Self::CsvFormat, _: &mut World) -> (Self, Transform) {
//         (
//             EnemyPathPoint {
//                 leniency: value.leniency,
//                 setting_1: value.setting_1,
//                 setting_2: value.setting_2,
//                 setting_3: value.setting_3,
//             },
//             Transform::from_xyz(value.position_x, value.position_y, value.position_z),
//         )
//     }
//     fn next_points(csv: &Self::CsvFormat) -> Option<Vec<u32>> {
//         Some(csv.next_points.clone())
//     }
// }
