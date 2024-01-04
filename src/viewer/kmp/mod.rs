pub mod components;
pub mod path;
mod point;
pub mod sections;
pub mod settings;

use self::{
    components::*,
    path::{spawn_path_section, spawn_route_section, update_node_links, KmpPathNodeLink},
    point::spawn_point_section,
};
use crate::{
    ui::{app_state::AppSettings, update_ui::KmpFileSelected},
    util::kmp_file::*,
    util::shapes::{Cone, Cylinder},
};
use bevy::prelude::*;
use std::{ffi::OsStr, fs::File};

use super::normalize::UpdateNormalizeSet;
// use bevy_mod_outline::OutlineMeshExt;

pub struct KmpPlugin;

impl Plugin for KmpPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<KmpVisibilityUpdated>().add_systems(
            Update,
            (
                spawn_model,
                // run update node links before update normalize so that the updated positions are normalized
                update_node_links.before(UpdateNormalizeSet),
                update_visible,
            ),
        );
    }
}

pub fn spawn_model(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut ev_kmp_file_selected: EventReader<KmpFileSelected>,
    kmp_section_query: Query<Entity, With<KmpSection>>,
    settings: Res<AppSettings>,
    mut ev_kmp_visibility_updated: EventWriter<KmpVisibilityUpdated>,
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
    let kmp_file = File::open(ev.0.clone()).expect("could not open kmp file");
    let kmp = Kmp::read(kmp_file).expect("could not read kmp file");

    // despawn all kmp entities so we have a clean slate
    for entity in kmp_section_query.iter() {
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

    // utility function for creating an unlit material of a certain colour
    let mut unlit_material = |color: Color| {
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
    };

    let sections = &settings.kmp_model.sections;

    // --- START POINTS ---

    spawn_point_section::<Ktpt, StartPoint>(
        &mut commands,
        &kmp.ktpt.entries,
        sphere_mesh.clone(),
        cylinder_mesh.clone(),
        cone_mesh.clone(),
        unlit_material(sections.start_points.color.point),
        unlit_material(sections.start_points.color.line),
        unlit_material(sections.start_points.color.arrow),
        unlit_material(sections.start_points.color.up_arrow),
    );

    // --- ENEMY PATHS ---

    spawn_path_section::<Enpt, EnemyPathPoint, EnemyPathMarker>(
        &mut commands,
        &kmp.enph.entries,
        &kmp.enpt.entries,
        sphere_mesh.clone(),
        cylinder_mesh.clone(),
        frustrum_mesh.clone(),
        unlit_material(sections.enemy_paths.color.point),
        unlit_material(sections.enemy_paths.color.line),
        unlit_material(sections.enemy_paths.color.arrow),
    );

    // --- ITEM POINTS ---

    spawn_path_section::<Itpt, ItemPathPoint, ItemPathMarker>(
        &mut commands,
        &kmp.itph.entries,
        &kmp.itpt.entries,
        sphere_mesh.clone(),
        cylinder_mesh.clone(),
        frustrum_mesh.clone(),
        unlit_material(sections.item_paths.color.point),
        unlit_material(sections.item_paths.color.line),
        unlit_material(sections.item_paths.color.arrow),
    );

    // --- CHECKPOINTS ---

    // --- OBJECTS ---

    spawn_point_section::<Gobj, Object>(
        &mut commands,
        &kmp.gobj.entries,
        sphere_mesh.clone(),
        cylinder_mesh.clone(),
        cone_mesh.clone(),
        unlit_material(sections.objects.color.point),
        unlit_material(sections.objects.color.line),
        unlit_material(sections.objects.color.arrow),
        unlit_material(sections.objects.color.up_arrow),
    );

    // --- ROUTES ---

    spawn_route_section(
        &mut commands,
        kmp.poti.entries,
        sphere_mesh.clone(),
        cylinder_mesh.clone(),
        frustrum_mesh.clone(),
        unlit_material(sections.routes.color.point),
        unlit_material(sections.routes.color.line),
        unlit_material(sections.routes.color.arrow),
    );

    // --- AREAS ---

    spawn_point_section::<Area, AreaPoint>(
        &mut commands,
        &kmp.area.entries,
        sphere_mesh.clone(),
        cylinder_mesh.clone(),
        cone_mesh.clone(),
        unlit_material(sections.areas.color.point),
        unlit_material(sections.areas.color.line),
        unlit_material(sections.areas.color.arrow),
        unlit_material(sections.areas.color.up_arrow),
    );

    // --- CAMREAS ---

    spawn_point_section::<Came, KmpCamera>(
        &mut commands,
        &kmp.came.entries,
        sphere_mesh.clone(),
        cylinder_mesh.clone(),
        cone_mesh.clone(),
        unlit_material(sections.cameras.color.point),
        unlit_material(sections.cameras.color.line),
        unlit_material(sections.cameras.color.arrow),
        unlit_material(sections.cameras.color.up_arrow),
    );

    // --- RESPAWN POINTS ---

    // --- CANNON POINTS ---

    // --- FINISH POINTS ---

    // --- STAGE INFO ---

    ev_kmp_visibility_updated.send_default();
    ev_kmp_file_selected.clear();
}

#[derive(Event, Default)]
pub struct KmpVisibilityUpdated;

fn update_visible(
    mut ev_kmp_visibility_updated: EventReader<KmpVisibilityUpdated>,
    settings: Res<AppSettings>,
    mut query: ParamSet<(
        ParamSet<(
            Query<(&mut Visibility, With<StartPoint>)>,
            Query<(&mut Visibility, With<EnemyPathMarker>)>,
            Query<(&mut Visibility, With<ItemPathMarker>)>,
            Query<(&mut Visibility, With<Object>)>,
            Query<(&mut Visibility, With<RouteMarker>)>,
            Query<(&mut Visibility, With<AreaPoint>)>,
        )>,
        ParamSet<(
            Query<(&mut Visibility, With<components::KmpCamera>)>,
            Query<(&mut Visibility, With<RespawnPoint>)>,
            Query<(&mut Visibility, With<CannonPoint>)>,
            Query<(&mut Visibility, With<FinishPoint>)>,
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
    // only run this function if the KmpVisibilityUpdated event is triggered
    if ev_kmp_visibility_updated.is_empty() {
        return;
    } else {
        ev_kmp_visibility_updated.clear();
    }

    let sections = &settings.kmp_model.sections;

    macro_rules! set_visibility {
        ($query:expr, $sect:ident) => {
            for (mut visibility, _) in $query.iter_mut() {
                *visibility = if sections.$sect.visible {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                };
            }
        };
    }

    set_visibility!(query.p0().p0(), start_points);
    set_visibility!(query.p0().p1(), enemy_paths);
    set_visibility!(query.p0().p2(), item_paths);
    set_visibility!(query.p0().p3(), objects);
    set_visibility!(query.p0().p4(), routes);
    set_visibility!(query.p0().p5(), areas);
    set_visibility!(query.p1().p0(), cameras);
    set_visibility!(query.p1().p1(), respawn_points);
    set_visibility!(query.p1().p2(), cannon_points);
    set_visibility!(query.p1().p3(), battle_finish_points);

    for (mut visibility, enemy_route, item_route) in query.p1().p4().iter_mut() {
        // if it is an enemy path node link
        if enemy_route.is_some() {
            *visibility = if sections.enemy_paths.visible {
                Visibility::Visible
            } else {
                Visibility::Hidden
            }
            // if it is an item path node link
        } else if item_route.is_some() {
            *visibility = if sections.item_paths.visible {
                Visibility::Visible
            } else {
                Visibility::Hidden
            }
        }
    }
}
