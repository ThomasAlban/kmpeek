pub mod checkpoints;
pub mod components;
pub mod meshes_materials;
pub mod path;
pub mod point;
pub mod sections;
pub mod settings;

use self::{
    checkpoints::{spawn_checkpoint_section, CheckpointHeight, CheckpointPlugin},
    components::*,
    meshes_materials::setup_kmp_meshes_materials,
    path::{spawn_enemy_item_path_section, traverse_paths, update_node_links, KmpPathNodeLink, RecalcPaths},
    point::{spawn_point_section, AddRespawnPointPreview},
    sections::KmpEditMode,
};
use crate::{
    ui::{settings::SetupAppSettingsSet, ui_state::KmpVisibility, update_ui::KmpFileSelected},
    util::kmp_file::*,
    viewer::kmp::{path::PathType, sections::KmpSections},
};
use bevy::prelude::*;
use binrw::BinRead;
use std::{ffi::OsStr, fs::File, sync::Arc};

pub struct KmpPlugin;

impl Plugin for KmpPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(CheckpointPlugin)
            .add_event::<RecalcPaths>()
            .init_resource::<KmpEditMode>()
            .add_systems(Startup, setup_kmp_meshes_materials.after(SetupAppSettingsSet))
            .add_systems(
                Update,
                (
                    spawn_model.run_if(on_event::<KmpFileSelected>()),
                    update_node_links,
                    update_visible.run_if(resource_changed::<KmpVisibility>),
                    traverse_paths,
                ),
            );
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
        commands.entity(entity).despawn();
    }

    let mut kmp_errors = Vec::new();

    // --- TRACK INFO ---

    let stgi = kmp.stgi.entries.first().unwrap();
    commands.insert_resource(TrackInfo::from_kmp(stgi, &mut kmp_errors, 0));

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

    // --- FINISH POINTS ---

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
            Query<&mut Visibility, With<CheckpointLeft>>,
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
    let visibilities = kmp_visibility.0;

    let set_visibility = |visibility: &mut Visibility, visible: bool| {
        *visibility = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    };
    macro_rules! set_visibility {
        ($query:expr, $i:expr) => {
            for mut visibility in $query.iter_mut() {
                set_visibility(&mut visibility, visibilities[$i]);
            }
        };
    }

    set_visibility!(q.p0().p0(), KmpSections::StartPoints as usize);
    set_visibility!(q.p0().p1(), KmpSections::EnemyPaths as usize);
    set_visibility!(q.p0().p2(), KmpSections::ItemPaths as usize);
    set_visibility!(q.p0().p3(), KmpSections::Checkpoints as usize);
    set_visibility!(q.p0().p4(), KmpSections::Objects as usize);
    set_visibility!(q.p0().p5(), KmpSections::Areas as usize);
    set_visibility!(q.p1().p0(), KmpSections::Cameras as usize);
    set_visibility!(q.p1().p1(), KmpSections::RespawnPoints as usize);
    set_visibility!(q.p1().p2(), KmpSections::CannonPoints as usize);
    set_visibility!(q.p1().p3(), KmpSections::BattleFinishPoints as usize);

    for (mut visibility, node_link) in q.p1().p4().iter_mut() {
        match node_link.kind {
            PathType::Enemy => set_visibility(&mut visibility, visibilities[KmpSections::EnemyPaths as usize]),
            PathType::Item => set_visibility(&mut visibility, visibilities[KmpSections::ItemPaths as usize]),
            PathType::CheckpointLeft | PathType::CheckpointRight => {
                set_visibility(&mut visibility, visibilities[KmpSections::Checkpoints as usize])
            }
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
