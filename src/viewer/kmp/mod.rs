pub mod components;
pub mod path;
mod point;
pub mod sections;
pub mod settings;

use self::{
    components::*,
    path::{
        spawn_path_section, spawn_route_section, update_node_links, KmpPathNodeLink, PathMaterials,
        PathMeshes,
    },
    point::{spawn_point_section, spawn_respawn_point_section, PointMaterials, PointMeshes},
    sections::KmpEditMode,
};
use super::normalize::UpdateNormalizeSet;
use crate::{
    ui::{settings::AppSettings, update_ui::KmpFileSelected},
    util::kmp_file::*,
    util::shapes::{Cone, Cylinder},
    viewer::kmp::sections::KmpModelSections,
};
use bevy::{prelude::*, window::RequestRedraw};
use binrw::BinRead;
use std::{ffi::OsStr, fs::File, sync::Arc};

pub struct KmpPlugin;

impl Plugin for KmpPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<KmpVisibilityUpdate>()
            .init_resource::<KmpEditMode>()
            .add_systems(
                Update,
                (
                    spawn_model.run_if(on_event::<KmpFileSelected>()),
                    // run update node links before update normalize so that the updated positions are normalized
                    update_node_links.before(UpdateNormalizeSet),
                    update_visible.run_if(on_event::<KmpVisibilityUpdate>()),
                ),
            );
    }
}

pub fn unlit_material(
    materials: &mut Assets<StandardMaterial>,
    color: Color,
) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: color,
        alpha_mode: if color.a() < 1. {
            AlphaMode::Blend
        } else {
            AlphaMode::Opaque
        },
        unlit: true,
        ..default()
    })
}

pub fn spawn_model(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut ev_kmp_file_selected: EventReader<KmpFileSelected>,
    q_kmp_section: Query<Entity, With<KmpSection>>,
    settings: Res<AppSettings>,
    mut ev_kmp_visibility_update: EventWriter<KmpVisibilityUpdate>,
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
    let kmp = Kmp::read(&mut kmp_file).expect("could not read kmp file");
    // allocate the KMP on the heap so that we can access it in commands which execute after this function
    let kmp = Arc::new(kmp);

    // despawn all kmp entities so we have a clean slate
    for entity in q_kmp_section.iter() {
        commands.entity(entity).despawn();
    }

    // meshes for the kmp model
    let sphere_mesh: Mesh = shape::UVSphere {
        radius: 100.,
        ..default()
    }
    .into();
    // sphere_mesh.generate_outline_normals().unwrap();
    let sphere_mesh = meshes.add(sphere_mesh);
    let cylinder_mesh = meshes.add(Mesh::from(Cylinder {
        height: 1.,
        radius_bottom: 50.,
        radius_top: 50.,
        radial_segments: 32,
        height_segments: 32,
    }));
    let frustrum_mesh = meshes.add(Mesh::from(Cylinder {
        height: 100.,
        radius_bottom: 100.,
        radius_top: 50.,
        radial_segments: 32,
        height_segments: 32,
    }));
    let cone_mesh = meshes.add(Mesh::from(Cone {
        height: 200.,
        radius: 100.,
        segments: 32,
    }));

    let point_meshes = PointMeshes::new(
        sphere_mesh.clone(),
        cylinder_mesh.clone(),
        cone_mesh.clone(),
    );
    let path_meshes = PathMeshes::new(
        sphere_mesh.clone(),
        cylinder_mesh.clone(),
        frustrum_mesh.clone(),
    );

    let sections = &settings.kmp_model.sections;

    // --- TRACK INFO ---

    let stgi = kmp.stgi.entries.first().unwrap();
    commands.spawn(TrackInfo::from_kmp(stgi));

    // --- START POINTS ---

    dbg!(&kmp.ktpt.entries[0].rotation);

    spawn_point_section::<Ktpt, StartPoint>(
        &mut commands,
        kmp.clone(),
        point_meshes.clone(),
        PointMaterials::from_colors(&mut materials, &sections.color.start_points),
        settings.kmp_model.outline.clone(),
    );

    // --- ENEMY PATHS ---

    spawn_path_section::<Enpt, EnemyPathPoint, EnemyPathMarker>(
        &mut commands,
        kmp.clone(),
        path_meshes.clone(),
        PathMaterials::from_colors(&mut materials, &sections.color.enemy_paths),
        settings.kmp_model.outline.clone(),
    );

    // --- ITEM POINTS ---

    spawn_path_section::<Itpt, ItemPathPoint, ItemPathMarker>(
        &mut commands,
        kmp.clone(),
        path_meshes.clone(),
        PathMaterials::from_colors(&mut materials, &sections.color.item_paths),
        settings.kmp_model.outline.clone(),
    );

    // --- CHECKPOINTS ---

    // --- OBJECTS ---

    spawn_point_section::<Gobj, Object>(
        &mut commands,
        kmp.clone(),
        point_meshes.clone(),
        PointMaterials::from_colors(&mut materials, &sections.color.objects),
        settings.kmp_model.outline.clone(),
    );

    // --- ROUTES ---

    spawn_route_section(
        &mut commands,
        kmp.clone(),
        path_meshes.clone(),
        PathMaterials::from_colors(&mut materials, &sections.color.routes),
    );

    // --- AREAS ---

    spawn_point_section::<Area, AreaPoint>(
        &mut commands,
        kmp.clone(),
        point_meshes.clone(),
        PointMaterials::from_colors(&mut materials, &sections.color.areas),
        settings.kmp_model.outline.clone(),
    );

    // --- CAMREAS ---

    spawn_point_section::<Came, KmpCamera>(
        &mut commands,
        kmp.clone(),
        point_meshes.clone(),
        PointMaterials::from_colors(&mut materials, &sections.color.cameras),
        settings.kmp_model.outline.clone(),
    );

    // --- RESPAWN POINTS ---

    spawn_respawn_point_section(
        &mut commands,
        kmp.clone(),
        point_meshes.clone(),
        PointMaterials::from_colors(&mut materials, &sections.color.respawn_points),
        settings.kmp_model.outline.clone(),
    );

    // --- CANNON POINTS ---

    // --- FINISH POINTS ---

    ev_kmp_visibility_update.send_default();
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
            Query<&mut Visibility, With<RouteMarker>>,
            Query<&mut Visibility, With<AreaPoint>>,
        )>,
        ParamSet<(
            Query<&mut Visibility, With<KmpCamera>>,
            Query<&mut Visibility, With<RespawnPoint>>,
            Query<&mut Visibility, With<CannonPoint>>,
            Query<&mut Visibility, With<FinishPoint>>,
            Query<
                (
                    &mut Visibility,
                    Option<&EnemyPathMarker>,
                    Option<&ItemPathMarker>,
                ),
                With<KmpPathNodeLink>,
            >,
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
    set_visibility!(q.p0().p4(), usize::from(KmpModelSections::Routes));
    set_visibility!(q.p0().p5(), usize::from(KmpModelSections::Area));
    set_visibility!(q.p1().p0(), usize::from(KmpModelSections::Cameras));
    set_visibility!(q.p1().p1(), usize::from(KmpModelSections::RespawnPoints));
    set_visibility!(q.p1().p2(), usize::from(KmpModelSections::CannonPoints));
    set_visibility!(
        q.p1().p3(),
        usize::from(KmpModelSections::BattleFinishPoints)
    );

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
