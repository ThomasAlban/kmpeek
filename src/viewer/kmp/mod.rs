pub mod checkpoints;
pub mod components;
pub mod csv;
pub mod meshes_materials;
pub mod ordering;
pub mod path;
pub mod point;
pub mod routes;
pub mod sections;
pub mod settings;

use self::{
    checkpoints::{checkpoint_plugin, spawn_checkpoint_section, CheckpointHeight},
    components::*,
    meshes_materials::setup_kmp_meshes_materials,
    path::{spawn_enemy_item_path_section, RecalcPaths},
    point::{spawn_point_section, AddRespawnPointPreview},
};
use crate::{
    ui::{
        file_dialog::{DialogType, FileDialogResult},
        settings::{AppSettings, SetupAppSettingsSet},
        ui_state::KmpFilePath,
        update_ui::{KclFileSelected, KmpFileSelected},
    },
    util::{kmp_file::*, try_despawn, BoolToVisibility},
};
use bevy::prelude::*;
use binrw::BinRead;
use derive_new::new;
use ordering::ordering_plugin;
use path::{path_plugin, PathGroups};
use routes::{routes_plugin, spawn_route_section};
use sections::{add_for_all_components, section_plugin, KmpEditMode, KmpEditModeChange};
use std::{ffi::OsStr, fs::File, marker::PhantomData, sync::Arc};

pub fn kmp_plugin(app: &mut App) {
    app.add_plugins((
        checkpoint_plugin,
        path_plugin,
        ordering_plugin,
        section_plugin,
        routes_plugin,
    ))
    .add_event::<RecalcPaths>()
    .add_systems(Startup, setup_kmp_meshes_materials.after(SetupAppSettingsSet))
    .add_systems(
        Update,
        (spawn_model.run_if(on_event::<KmpFileSelected>()), open_kmp_kcl),
    );

    add_for_all_components!(@event app, SetSectionVisibility);
    app.add_event::<SetSectionVisibility<TrackInfo>>();
    add_for_all_components!(@system app, update_visible_on_mode_change);
    add_for_all_components!(@system app, set_section_visibility);
}

pub fn open_kmp_kcl(
    mut ev_file_dialog: EventReader<FileDialogResult>,
    mut ev_kmp_file_selected: EventWriter<KmpFileSelected>,
    mut ev_kcl_file_selected: EventWriter<KclFileSelected>,
    settings: ResMut<AppSettings>,
    mut commands: Commands,
) {
    for FileDialogResult { path, dialog_type } in ev_file_dialog.read() {
        if let DialogType::OpenKmpKcl = dialog_type {
            if let Some(file_ext) = path.extension() {
                if file_ext == "kmp" {
                    commands.insert_resource(KmpFilePath(path.into()));
                    ev_kmp_file_selected.send(KmpFileSelected(path.into()));
                    if settings.open_course_kcl_in_dir {
                        let mut course_kcl_path = path.to_owned();
                        course_kcl_path.set_file_name("course.kcl");
                        if course_kcl_path.exists() {
                            ev_kcl_file_selected.send(KclFileSelected(course_kcl_path));
                        }
                    }
                } else if file_ext == "kcl" {
                    ev_kcl_file_selected.send(KclFileSelected(path.into()));
                }
            }
        }
    }
}

pub fn spawn_model(
    mut commands: Commands,
    mut ev_kmp_file_selected: EventReader<KmpFileSelected>,
    q_kmp_section: Query<Entity, With<KmpSelectablePoint>>,
    mut ev_recalc_paths: EventWriter<RecalcPaths>,
    checkpoint_height: Res<CheckpointHeight>,
) {
    // if there is no kmp file selected event return
    let Some(ev) = ev_kmp_file_selected.read().next() else {
        return;
    };
    // if the file extension is not 'kmp' return
    if ev.0.extension() != Some(OsStr::new("kmp")) {
        return;
    }

    // open the KMP file and read it
    let mut kmp_file = File::open(ev.0.clone()).expect("could not open kmp file");
    let kmp = KmpFile::read(&mut kmp_file).expect("could not read kmp file");
    // allocate the KMP on the heap so that we can access it in commands which execute after this function
    let kmp = Arc::new(kmp);

    // despawn all kmp entities so we have a clean slate
    for entity in q_kmp_section.iter() {
        try_despawn(&mut commands, entity);
    }
    commands.remove_resource::<PathGroups<EnemyPathPoint>>();
    commands.remove_resource::<PathGroups<ItemPathPoint>>();
    commands.remove_resource::<PathGroups<Checkpoint>>();

    let mut kmp_errors = Vec::new();

    // --- TRACK INFO ---

    let stgi = kmp.stgi.entries.first().unwrap();
    commands.insert_resource(TrackInfo::from_kmp(stgi, &mut kmp_errors));

    // --- ROUTES ---
    let route_id_map = spawn_route_section(&mut commands, kmp.clone(), &mut kmp_errors);

    // --- RESPAWN POINTS ---
    let respawn_points_id_map =
        spawn_point_section::<Jgpt, RespawnPoint>(&mut commands, &route_id_map, kmp.clone(), &mut kmp_errors);
    respawn_points_id_map
        .iter()
        .for_each(|(_, e)| commands.add(AddRespawnPointPreview(*e)));

    // --- START POINTS ---
    spawn_point_section::<Ktpt, StartPoint>(&mut commands, &route_id_map, kmp.clone(), &mut kmp_errors);

    // --- ENEMY PATHS ---
    spawn_enemy_item_path_section::<Enpt, EnemyPathPoint>(&mut commands, kmp.clone(), &mut kmp_errors);

    // --- ITEM PATHS ---
    spawn_enemy_item_path_section::<Itpt, ItemPathPoint>(&mut commands, kmp.clone(), &mut kmp_errors);

    // --- CHECKPOINTS ---
    spawn_checkpoint_section(
        &mut commands,
        kmp.clone(),
        &mut kmp_errors,
        checkpoint_height.0,
        respawn_points_id_map,
    );

    // --- OBJECTS ---
    spawn_point_section::<Gobj, Object>(&mut commands, &route_id_map, kmp.clone(), &mut kmp_errors);

    // --- AREAS ---
    spawn_point_section::<Area, AreaPoint>(&mut commands, &route_id_map, kmp.clone(), &mut kmp_errors);

    // --- CAMREAS ---
    spawn_point_section::<Came, KmpCamera>(&mut commands, &route_id_map, kmp.clone(), &mut kmp_errors);

    // --- CANNON POINTS ---
    spawn_point_section::<Cnpt, CannonPoint>(&mut commands, &route_id_map, kmp.clone(), &mut kmp_errors);

    // --- FINISH POINTS ---
    spawn_point_section::<Mspt, BattleFinishPoint>(&mut commands, &route_id_map, kmp.clone(), &mut kmp_errors);

    ev_recalc_paths.send(RecalcPaths::all());
}

#[derive(Event, Deref, new)]
pub struct SetSectionVisibility<T>(#[deref] pub bool, PhantomData<T>);

fn set_section_visibility<T: Component>(
    mut ev_set_sect_visibility: EventReader<SetSectionVisibility<T>>,
    mut q: Query<&mut Visibility, (With<KmpSelectablePoint>, With<T>)>,
) {
    let Some(ev) = ev_set_sect_visibility.read().next() else {
        return;
    };
    let visible = **ev;

    for mut visibility in q.iter_mut() {
        *visibility = visible.to_visibility();
    }
}

fn update_visible_on_mode_change<T: Component>(
    mut mode_change: EventReader<KmpEditModeChange>,
    cur_mode: Option<Res<KmpEditMode<T>>>,
    mut ev_set_sect_visibility: EventWriter<SetSectionVisibility<T>>,
) {
    if mode_change.read().next().is_none() {
        return;
    }
    ev_set_sect_visibility.send(SetSectionVisibility::new(cur_mode.is_some()));
}

/// Utility function for calculating the transform a cylinder should have in order to join 2 points
fn calc_line_transform(l_tr: Vec3, r_tr: Vec3) -> Transform {
    let mut line_transform = Transform::from_translation(l_tr.lerp(r_tr, 0.5)).looking_at(r_tr, Vec3::Y);
    line_transform.rotate_local_x(f32::to_radians(-90.));
    line_transform.scale.y = l_tr.distance(r_tr);
    line_transform
}
/// Utility function for calculating the transform a checkpoint arrow should have
fn calc_cp_arrow_transform(l_tr: Vec3, r_tr: Vec3) -> Transform {
    let mp = l_tr.lerp(r_tr, 0.5);
    let mut trans = Transform::from_translation(mp).looking_at(r_tr, Vec3::Y);
    trans.rotate_local_z(f32::to_radians(90.));
    trans
}
