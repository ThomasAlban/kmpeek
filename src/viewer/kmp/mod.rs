pub mod components;
pub mod meshes_materials;
pub mod path;
pub mod point;
pub mod sections;
pub mod settings;

use self::{
    components::*,
    meshes_materials::{setup_kmp_meshes_materials, KmpMeshesMaterials},
    path::{spawn_path_section, traverse_paths, update_node_links, KmpPathNodeLink, RecalculatePaths},
    point::{add_respawn_point_preview, spawn_point_section},
    sections::KmpEditMode,
};
use crate::{
    ui::{
        settings::{AppSettings, SetupAppSettingsSet},
        update_ui::KmpFileSelected,
    },
    util::kmp_file::*,
    viewer::kmp::sections::KmpModelSections,
};
use bevy::{prelude::*, window::RequestRedraw};
use binrw::BinRead;
use std::{ffi::OsStr, fs::File, sync::Arc};

pub struct KmpPlugin;

impl Plugin for KmpPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<KmpVisibilityUpdate>()
            .add_event::<RecalculatePaths>()
            .init_resource::<KmpEditMode>()
            .add_systems(
                Startup,
                (apply_deferred, setup_kmp_meshes_materials)
                    .chain()
                    .after(SetupAppSettingsSet),
            )
            .add_systems(
                Update,
                (
                    spawn_model.run_if(on_event::<KmpFileSelected>()),
                    update_node_links,
                    update_visible.run_if(on_event::<KmpVisibilityUpdate>()),
                    traverse_paths.run_if(on_event::<RecalculatePaths>()),
                ),
            );
    }
}
pub fn spawn_model(
    mut commands: Commands,
    mut ev_kmp_file_selected: EventReader<KmpFileSelected>,
    q_kmp_section: Query<Entity, With<KmpSelectablePoint>>,
    settings: Res<AppSettings>,
    mut ev_kmp_visibility_update: EventWriter<KmpVisibilityUpdate>,
    mut ev_recalculate_paths: EventWriter<RecalculatePaths>,
    kmp_meshes_materials: Res<KmpMeshesMaterials>,
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

    let meshes = &kmp_meshes_materials.meshes;
    let materials = &kmp_meshes_materials.materials;

    // --- TRACK INFO ---

    let stgi = kmp.stgi.entries.first().unwrap();
    commands.spawn(TrackInfo::from_kmp(stgi));

    // --- START POINTS ---

    spawn_point_section::<Ktpt, StartPoint>(
        &mut commands,
        kmp.clone(),
        meshes.clone(),
        materials.start_points.clone(),
        settings.kmp_model.outline.clone(),
    );

    // --- ENEMY PATHS ---

    spawn_path_section::<Enpt, EnemyPathPoint, EnemyPathMarker>(
        &mut commands,
        kmp.clone(),
        meshes.clone(),
        materials.enemy_paths.clone(),
        settings.kmp_model.outline.clone(),
    );

    // --- ITEM PATHS ---

    spawn_path_section::<Itpt, ItemPathPoint, ItemPathMarker>(
        &mut commands,
        kmp.clone(),
        meshes.clone(),
        materials.item_paths.clone(),
        settings.kmp_model.outline.clone(),
    );

    // --- CHECKPOINTS ---

    // --- OBJECTS ---

    spawn_point_section::<Gobj, Object>(
        &mut commands,
        kmp.clone(),
        meshes.clone(),
        materials.objects.clone(),
        settings.kmp_model.outline.clone(),
    );

    // --- ROUTES ---

    // spawn_route_section(
    //     &mut commands,
    //     kmp.clone(),
    //     path_meshes.clone(),
    //     PathMaterials::from_colors(&mut materials, &sections.color.routes),
    // );

    // --- AREAS ---

    spawn_point_section::<Area, AreaPoint>(
        &mut commands,
        kmp.clone(),
        meshes.clone(),
        materials.areas.clone(),
        settings.kmp_model.outline.clone(),
    );

    // --- CAMREAS ---

    spawn_point_section::<Came, KmpCamera>(
        &mut commands,
        kmp.clone(),
        meshes.clone(),
        materials.cameras.clone(),
        settings.kmp_model.outline.clone(),
    );

    // --- RESPAWN POINTS ---

    let respawn_points = spawn_point_section::<Jgpt, RespawnPoint>(
        &mut commands,
        kmp.clone(),
        meshes.clone(),
        materials.respawn_points.clone(),
        settings.kmp_model.outline.clone(),
    );
    respawn_points
        .iter()
        .for_each(|e| add_respawn_point_preview(*e, &mut commands, meshes, &materials.respawn_points));

    // spawn_respawn_point_section(
    //     &mut commands,
    //     kmp.clone(),
    //     meshes.clone(),
    //     materials.respawn_points.clone(),
    //     settings.kmp_model.outline.clone(),
    // );

    // --- CANNON POINTS ---

    // --- FINISH POINTS ---

    // ---

    ev_kmp_visibility_update.send_default();
    ev_recalculate_paths.send_default();
}

#[derive(Event, Default)]
pub struct KmpVisibilityUpdate;

fn update_visible(
    settings: Res<AppSettings>,
    mut ev_request_redraw: EventWriter<RequestRedraw>,
    mut q: ParamSet<(
        ParamSet<(
            Query<&mut Visibility, With<StartPoint>>,
            Query<&mut Visibility, With<EnemyPathMarker>>,
            Query<&mut Visibility, With<ItemPathMarker>>,
            Query<&mut Visibility, With<Object>>,
            Query<&mut Visibility, With<AreaPoint>>,
        )>,
        ParamSet<(
            Query<&mut Visibility, With<KmpCamera>>,
            Query<&mut Visibility, With<RespawnPoint>>,
            Query<&mut Visibility, With<CannonPoint>>,
            Query<&mut Visibility, With<BattleFinishPoint>>,
            Query<(&mut Visibility, Option<&EnemyPathMarker>, Option<&ItemPathMarker>), With<KmpPathNodeLink>>,
        )>,
    )>,
) {
    let sections = &settings.kmp_model.sections;

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
                set_visibility(&mut visibility, sections.visible[$i]);
            }
        };
    }

    set_visibility!(q.p0().p0(), usize::from(KmpModelSections::StartPoints));
    set_visibility!(q.p0().p1(), usize::from(KmpModelSections::EnemyPaths));
    set_visibility!(q.p0().p2(), usize::from(KmpModelSections::ItemPaths));
    set_visibility!(q.p0().p3(), usize::from(KmpModelSections::Objects));
    set_visibility!(q.p0().p4(), usize::from(KmpModelSections::Areas));
    set_visibility!(q.p1().p0(), usize::from(KmpModelSections::Cameras));
    set_visibility!(q.p1().p1(), usize::from(KmpModelSections::RespawnPoints));
    set_visibility!(q.p1().p2(), usize::from(KmpModelSections::CannonPoints));
    set_visibility!(q.p1().p3(), usize::from(KmpModelSections::BattleFinishPoints));

    for (mut visibility, enemy_route, item_route) in q.p1().p4().iter_mut() {
        if enemy_route.is_some() {
            // if it is an enemy path node link
            set_visibility(
                &mut visibility,
                sections.visible[usize::from(KmpModelSections::EnemyPaths)],
            );
        } else if item_route.is_some() {
            // if it is an item path node link
            set_visibility(
                &mut visibility,
                sections.visible[usize::from(KmpModelSections::ItemPaths)],
            );
        }
    }
    ev_request_redraw.send(RequestRedraw);
}
