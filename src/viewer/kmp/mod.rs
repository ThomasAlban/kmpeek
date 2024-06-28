pub mod checkpoints;
pub mod components;
pub mod csv;
pub mod meshes_materials;
pub mod ordering;
pub mod path;
pub mod point;
pub mod sections;
pub mod settings;

use self::{
    checkpoints::{checkpoint_plugin, spawn_checkpoint_section, CheckpointHeight},
    components::*,
    meshes_materials::setup_kmp_meshes_materials,
    path::{spawn_enemy_item_path_section, KmpPathNodeLink, RecalcPaths},
    point::{spawn_point_section, AddRespawnPointPreview},
};
use crate::{
    ui::{settings::SetupAppSettingsSet, ui_state::KmpVisibility, update_ui::KmpFileSelected},
    util::{kmp_file::*, BoolToVisibility},
    viewer::kmp::path::PathType,
};
use bevy::prelude::*;
use binrw::BinRead;
use csv::csv_plugin;
use ordering::ordering_plugin;
use path::path_plugin;
use sections::{section_plugin, KmpSection};
use std::{ffi::OsStr, fs::File, sync::Arc};

pub fn kmp_plugin(app: &mut App) {
    app.add_plugins((
        checkpoint_plugin,
        path_plugin,
        ordering_plugin,
        csv_plugin,
        section_plugin,
    ))
    .add_event::<RecalcPaths>()
    .add_systems(Startup, setup_kmp_meshes_materials.after(SetupAppSettingsSet))
    .add_systems(
        Update,
        (
            spawn_model.run_if(on_event::<KmpFileSelected>()),
            update_visible.run_if(resource_changed::<KmpVisibility>),
        ),
    );
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
        commands.entity(entity).despawn();
    }

    let mut kmp_errors = Vec::new();

    // --- TRACK INFO ---

    let stgi = kmp.stgi.entries.first().unwrap();
    commands.insert_resource(TrackInfo::from_kmp(stgi, &mut kmp_errors));

    // --- START POINTS ---

    spawn_point_section::<Ktpt, StartPoint>(&mut commands, kmp.clone(), &mut kmp_errors);

    // --- ENEMY PATHS ---

    spawn_enemy_item_path_section::<Enpt, EnemyPathPoint>(&mut commands, kmp.clone(), &mut kmp_errors);

    // --- ITEM PATHS ---

    spawn_enemy_item_path_section::<Itpt, ItemPathPoint>(&mut commands, kmp.clone(), &mut kmp_errors);

    // --- CHECKPOINTS ---
    //
    spawn_checkpoint_section(&mut commands, kmp.clone(), &mut kmp_errors, checkpoint_height.0);

    // --- OBJECTS ---

    spawn_point_section::<Gobj, Object>(&mut commands, kmp.clone(), &mut kmp_errors);

    // --- ROUTES ---

    // --- AREAS ---

    spawn_point_section::<Area, AreaPoint>(&mut commands, kmp.clone(), &mut kmp_errors);

    // --- CAMREAS ---

    spawn_point_section::<Came, KmpCamera>(&mut commands, kmp.clone(), &mut kmp_errors);

    // --- RESPAWN POINTS ---

    let respawn_points = spawn_point_section::<Jgpt, RespawnPoint>(&mut commands, kmp.clone(), &mut kmp_errors);
    respawn_points
        .iter()
        .for_each(|e| commands.add(AddRespawnPointPreview(*e)));

    // --- CANNON POINTS ---

    spawn_point_section::<Cnpt, CannonPoint>(&mut commands, kmp.clone(), &mut kmp_errors);

    // --- FINISH POINTS ---

    spawn_point_section::<Mspt, BattleFinishPoint>(&mut commands, kmp.clone(), &mut kmp_errors);

    // ---

    ev_recalc_paths.send(RecalcPaths::all());
}

fn update_visible(
    kmp_visibility: Res<KmpVisibility>,
    mut q: ParamSet<(
        ParamSet<(
            Query<&mut Visibility, With<StartPoint>>,
            Query<&mut Visibility, With<EnemyPathPoint>>,
            Query<&mut Visibility, With<ItemPathPoint>>,
            Query<&mut Visibility, With<Checkpoint>>,
            Query<&mut Visibility, With<Object>>,
            Query<&mut Visibility, With<AreaPoint>>,
        )>,
        ParamSet<(
            Query<&mut Visibility, With<KmpCamera>>,
            Query<&mut Visibility, With<RespawnPoint>>,
            Query<&mut Visibility, With<CannonPoint>>,
            Query<&mut Visibility, With<BattleFinishPoint>>,
            Query<(&mut Visibility, &KmpPathNodeLink)>,
        )>,
    )>,
) {
    use KmpSection::*;

    let visibilities = kmp_visibility.0;
    let set_visibility = |mut v: Mut<Visibility>, i: KmpSection| {
        *v = visibilities[i as usize].to_visibility();
    };
    macro_rules! set_visibility_iter {
        ($q:expr, $i:ident) => {
            for v in $q.iter_mut() {
                set_visibility(v, $i);
            }
        };
    }

    set_visibility_iter!(q.p0().p0(), StartPoints);
    set_visibility_iter!(q.p0().p1(), EnemyPaths);
    set_visibility_iter!(q.p0().p2(), ItemPaths);
    set_visibility_iter!(q.p0().p3(), Checkpoints);
    set_visibility_iter!(q.p0().p4(), Objects);
    set_visibility_iter!(q.p0().p5(), Areas);
    set_visibility_iter!(q.p1().p0(), Cameras);
    set_visibility_iter!(q.p1().p1(), RespawnPoints);
    set_visibility_iter!(q.p1().p2(), CannonPoints);
    set_visibility_iter!(q.p1().p3(), BattleFinishPoints);

    for (v, node_link) in q.p1().p4().iter_mut() {
        match node_link.kind {
            PathType::Enemy => set_visibility(v, EnemyPaths),
            PathType::Item => set_visibility(v, ItemPaths),
            PathType::Checkpoint { .. } => set_visibility(v, Checkpoints),
        }
    }
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
