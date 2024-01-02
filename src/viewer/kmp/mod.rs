mod path;
pub mod settings;

use self::path::{spawn_route_section, update_node_links, KmpPathNodeLink};
use crate::{
    ui::{app_state::AppSettings, update_ui::KmpFileSelected},
    util::kmp_file::*,
    util::Cylinder,
    viewer::normalize::Normalize,
};
use bevy::prelude::*;
use std::{ffi::OsStr, fs::File};
// use bevy_mod_outline::OutlineMeshExt;

pub struct KmpPlugin;

impl Plugin for KmpPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<KmpVisibilityUpdated>()
            .add_systems(Update, (spawn_model, update_node_links, update_visible));
    }
}

#[derive(Component, Default)]
pub struct KmpSection;

trait FromKmpPoint<T: KmpData> {
    fn from(kmp_data: T) -> Self;
}

// components attached to kmp entities, to store data about them:

// --- START POINT COMPONENTS ---

#[derive(Component, Default)]
pub struct StartPoint;

// --- ENEMY PATH COMPONENTS ---

#[derive(Component, Default)]
pub struct EnemyPath;
#[derive(Component, Clone)]
pub struct EnemyPathPoint {
    pub leniency: f32,
    pub setting_1: u16,
    pub setting_2: u8,
    pub setting_3: u8,
}
impl FromKmpPoint<Enpt> for EnemyPathPoint {
    fn from(kmp_data: Enpt) -> Self {
        Self {
            leniency: kmp_data.leniency,
            setting_1: kmp_data.setting_1,
            setting_2: kmp_data.setting_2,
            setting_3: kmp_data.setting_3,
        }
    }
}
// --- ITEM PATH COMPONENTS ---

#[derive(Component, Default)]
pub struct ItemPath;
#[derive(Component)]
pub struct ItemPathPoint {
    pub bullet_bill_control: f32,
    pub setting_1: u16,
    pub setting_2: u16,
}
impl FromKmpPoint<Itpt> for ItemPathPoint {
    fn from(kmp_data: Itpt) -> Self {
        Self {
            bullet_bill_control: kmp_data.bullet_bill_control,
            setting_1: kmp_data.setting_1,
            setting_2: kmp_data.setting_2,
        }
    }
}
#[derive(Component)]
pub struct Object;
#[derive(Component)]
pub struct AreaPoint;
#[derive(Component)]
pub struct Camera;
#[derive(Component)]
pub struct RespawnPoint;
#[derive(Component)]
pub struct CannonPoint;
#[derive(Component)]
pub struct FinishPoint;

pub fn spawn_model(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut ev_kmp_file_selected: EventReader<KmpFileSelected>,
    kmp_section_query: Query<Entity, With<KmpSection>>,
    settings: Res<AppSettings>,
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
    let cone_mesh = meshes.add(Mesh::from(Cylinder {
        height: 100.,
        radius_bottom: 100.,
        radius_top: 50.,
        radial_segments: 32,
        height_segments: 32,
    }));

    // utility function for creating an unlit material of a certain colour
    let mut unlit_material = |color: Color| {
        materials.add(StandardMaterial {
            base_color: color,
            unlit: true,
            ..default()
        })
    };

    let sections = &settings.kmp_model.sections;

    // --- START POINTS ---

    for start_point in kmp.ktpt.entries.iter() {
        commands.spawn((
            PbrBundle {
                mesh: sphere_mesh.clone(),
                material: unlit_material(sections.start_points.color),
                transform: Transform::from_translation(start_point.position),
                ..default()
            },
            StartPoint,
            KmpSection,
            Normalize::new(200., 12., BVec3::TRUE),
        ));
    }

    // --- ENEMY PATHS ---

    spawn_route_section::<Enpt, EnemyPathPoint, EnemyPath>(
        &mut commands,
        &kmp.enph.entries,
        &kmp.enpt.entries,
        sphere_mesh.clone(),
        cylinder_mesh.clone(),
        cone_mesh.clone(),
        unlit_material(sections.enemy_paths.color.point),
        unlit_material(sections.enemy_paths.color.line),
        unlit_material(sections.enemy_paths.color.arrow),
    );

    // --- ITEM POINTS ---

    spawn_route_section::<Itpt, ItemPathPoint, ItemPath>(
        &mut commands,
        &kmp.itph.entries,
        &kmp.itpt.entries,
        sphere_mesh.clone(),
        cylinder_mesh.clone(),
        cone_mesh.clone(),
        unlit_material(sections.item_paths.color.point),
        unlit_material(sections.item_paths.color.line),
        unlit_material(sections.item_paths.color.arrow),
    );

    // --- CHECKPOINTS ---

    // --- OBJECTS ---

    for object in kmp.gobj.entries.iter() {
        commands.spawn((
            PbrBundle {
                mesh: sphere_mesh.clone(),
                material: unlit_material(sections.objects.color),
                transform: Transform::from_translation(object.position),
                ..default()
            },
            StartPoint,
            KmpSection,
            Normalize::new(200., 12., BVec3::TRUE),
        ));
    }

    // --- ROUTES ---

    // --- AREAS ---

    // --- CAMREAS ---

    // --- RESPAWN POINTS ---

    // --- CANNON POINTS ---

    // --- FINISH POINTS ---

    // --- STAGE INFO ---
}

#[derive(Event, Default)]
pub struct KmpVisibilityUpdated;

fn update_visible(
    mut ev_kmp_visibility_updated: EventReader<KmpVisibilityUpdated>,
    settings: Res<AppSettings>,
    mut query: ParamSet<(
        ParamSet<(
            Query<(&mut Visibility, With<StartPoint>)>,
            Query<(&mut Visibility, With<EnemyPath>)>,
            Query<(&mut Visibility, With<ItemPath>)>,
            Query<(&mut Visibility, With<Object>)>,
            Query<(&mut Visibility, With<AreaPoint>)>,
        )>,
        ParamSet<(
            Query<(&mut Visibility, With<Camera>)>,
            Query<(&mut Visibility, With<RespawnPoint>)>,
            Query<(&mut Visibility, With<CannonPoint>)>,
            Query<(&mut Visibility, With<FinishPoint>)>,
            Query<(&mut Visibility, Option<&EnemyPath>, Option<&ItemPath>), With<KmpPathNodeLink>>,
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
    set_visibility!(query.p0().p4(), areas);
    set_visibility!(query.p1().p0(), cameras);
    set_visibility!(query.p1().p1(), respawn_points);
    set_visibility!(query.p1().p2(), cannon_points);
    set_visibility!(query.p1().p3(), finish_points);

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
