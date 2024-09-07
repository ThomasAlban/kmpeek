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
    checkpoints::{checkpoint_plugin, spawn_checkpoint_section},
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
    util::kmp_file::*,
};
use anyhow::{bail, Context};
use bevy::{
    ecs::{entity::EntityHashMap, system::SystemState, world::Command},
    prelude::*,
    utils::HashMap,
};
use derive_new::new;
use ordering::{ordering_plugin, RefreshOrdering};
use path::{path_plugin, save_path_section, EntityPathGroups};
use point::save_point_section;
use routes::{routes_plugin, spawn_route_section};
use sections::{add_for_all_components, section_plugin, KmpEditMode};
use std::{ffi::OsStr, fs::File, marker::PhantomData};

pub fn kmp_plugin(app: &mut App) {
    app.add_plugins((
        checkpoint_plugin,
        path_plugin,
        ordering_plugin,
        section_plugin,
        routes_plugin,
    ))
    .add_event::<SaveFile>()
    .add_systems(Startup, setup_kmp_meshes_materials.after(SetupAppSettingsSet))
    .add_systems(
        Update,
        (save_kmp.pipe(handle_save_kmp_errors)).run_if(on_event::<SaveFile>()),
    )
    .add_systems(
        Update,
        (
            open_kmp
                .pipe(handle_open_kmp_errors)
                .run_if(on_event::<KmpFileSelected>()),
            open_kmp_kcl,
        ),
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
) {
    for FileDialogResult { path, dialog_type } in ev_file_dialog.read() {
        if let DialogType::OpenKmpKcl = dialog_type {
            if let Some(file_ext) = path.extension() {
                if file_ext == "kmp" {
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

#[derive(Resource, Deref, DerefMut, Clone, Default)]
pub struct KmpErrors(pub Vec<KmpError>);
impl KmpErrors {
    pub fn add(&mut self, msg: impl Into<String>) {
        self.push(KmpError::new(msg.into()));
    }
}
#[derive(Clone, new)]
pub struct KmpError {
    #[allow(unused)]
    message: String,
}
#[derive(Resource, Deref, DerefMut, Clone, Default, new)]
pub struct KmpSectionIdEntityMap<T: Component>(#[deref] pub HashMap<u32, Entity>, PhantomData<T>);

pub fn open_kmp(world: &mut World) -> anyhow::Result<()> {
    let mut ss = SystemState::<EventReader<KmpFileSelected>>::new(world);
    let mut ev_kmp_file_selected = ss.get(world);
    let Some(ev) = ev_kmp_file_selected.read().next() else {
        return Ok(());
    };
    // if the file extension is not 'kmp' return
    if ev.extension() != Some(OsStr::new("kmp")) {
        bail!("file extension was not .kmp")
    }

    // open the KMP file and read it
    let mut kmp_file = File::open(ev.0.clone()).context("could not open kmp file")?;
    let kmp = KmpFile::read(&mut kmp_file).context("could not read kmp file")?;

    world.insert_resource(KmpFilePath(ev.0.clone()));

    // get rid of all kmp points we may currently have in the world
    let entities: Vec<_> = world
        .query_filtered::<Entity, With<KmpSelectablePoint>>()
        .iter(world)
        .collect();
    for e in entities {
        world.entity_mut(e).despawn_recursive();
    }
    world.remove_resource::<EntityPathGroups<EnemyPathPoint>>();
    world.remove_resource::<EntityPathGroups<ItemPathPoint>>();
    world.remove_resource::<EntityPathGroups<Checkpoint>>();

    world.init_resource::<KmpErrors>();

    let stgi = kmp.stgi.first().unwrap();
    let track_info = TrackInfo::from_kmp(stgi, world);
    world.insert_resource(track_info);

    // --- ROUTES ---
    let route_id_map = spawn_route_section(world, &kmp);
    world.insert_resource(route_id_map);

    // --- RESPAWN POINTS ---
    let respawn_pts_id_map = spawn_point_section::<RespawnPoint>(world, &kmp);
    respawn_pts_id_map
        .iter()
        .for_each(|(_, e)| AddRespawnPointPreview(*e).apply(world));
    world.insert_resource(respawn_pts_id_map);

    // --- START POINTS ---
    spawn_point_section::<StartPoint>(world, &kmp);

    // --- ENEMY PATHS ---
    spawn_enemy_item_path_section::<EnemyPathPoint>(world, &kmp);

    // --- ITEM PATHS ---
    spawn_enemy_item_path_section::<ItemPathPoint>(world, &kmp);

    // --- CHECKPOINTS ---
    spawn_checkpoint_section(world, &kmp);

    // --- OBJECTS ---
    spawn_point_section::<Object>(world, &kmp);

    // --- AREAS ---
    spawn_point_section::<AreaPoint>(world, &kmp);

    // --- CAMREAS ---
    let camera_id_map = spawn_point_section::<KmpCamera>(world, &kmp);

    // the intro start index is the first byte of the additional value
    let intro_start = kmp.came.section_header.additional_value >> 8;
    dbg!(&intro_start);
    if let Some(e) = camera_id_map.get(&(intro_start as u32)) {
        world.entity_mut(*e).insert(KmpCameraIntroStart);
    }

    // --- CANNON POINTS ---
    spawn_point_section::<CannonPoint>(world, &kmp);

    // --- FINISH POINTS ---
    spawn_point_section::<BattleFinishPoint>(world, &kmp);

    world.send_event(RecalcPaths::all());

    world.remove_resource::<KmpErrors>();
    world.remove_resource::<KmpSectionIdEntityMap<RoutePoint>>();
    world.remove_resource::<KmpSectionIdEntityMap<RespawnPoint>>();

    world.send_event(RefreshOrdering);

    Ok(())
}

fn handle_open_kmp_errors(In(result): In<anyhow::Result<()>>) {
    if let Err(err) = result {
        dbg!(err);
    }
}

#[derive(Resource, Deref, DerefMut, Clone, Default, new)]
pub struct KmpSectionEntityIdMap<T: Component>(#[deref] pub EntityHashMap<u8>, PhantomData<T>);

#[derive(Event)]
pub struct SaveFile;

pub fn save_kmp(world: &mut World) -> anyhow::Result<()> {
    let mut kmp = KmpFile::default();
    let (mut poti, route_id_map) = save_point_section::<RouteSettings>(world);
    // additional value of poti section header must be set to the total number of points in all routes
    poti.section_header.additional_value = poti.iter().flat_map(|x| x.iter()).count() as u16;
    kmp.poti = poti;
    world.insert_resource(route_id_map);
    let (jgpt, respawn_id_map) = save_point_section::<RespawnPoint>(world);
    kmp.jgpt = jgpt;
    world.insert_resource(respawn_id_map);

    let (ktpt, _) = save_point_section::<StartPoint>(world);
    kmp.ktpt = ktpt;
    let (enpt, enph) = save_path_section::<EnemyPathPoint>(world);
    kmp.enpt = enpt;
    kmp.enph = enph;
    let (itpt, itph) = save_path_section::<ItemPathPoint>(world);
    kmp.itpt = itpt;
    kmp.itph = itph;
    let (ckpt, ckph) = save_path_section::<Checkpoint>(world);
    kmp.ckpt = ckpt;
    kmp.ckph = ckph;
    let (gobj, _) = save_point_section::<Object>(world);
    kmp.gobj = gobj;
    let (area, _) = save_point_section::<AreaPoint>(world);
    kmp.area = area;
    let (mut came, camera_id_map) = save_point_section::<KmpCamera>(world);
    // additional value of came section is the intro cam start
    let intro_start_e = world
        .query_filtered::<Entity, With<KmpCameraIntroStart>>()
        .iter(world)
        .next();
    came.section_header.additional_value = if let Some(e) = intro_start_e {
        let id = *camera_id_map.get(&e).unwrap() as u16;
        id << 8
    } else {
        0
    };
    kmp.came = came;
    let (cnpt, _) = save_point_section::<CannonPoint>(world);
    kmp.cnpt = cnpt;
    let (mspt, _) = save_point_section::<BattleFinishPoint>(world);
    kmp.mspt = mspt;

    kmp.stgi = Section::new(vec![world.resource::<TrackInfo>().clone().to_kmp(
        Transform::default(),
        world,
        Entity::PLACEHOLDER,
    )]);

    let kmp_file_path = world.resource::<KmpFilePath>().clone().0;
    let mut kmp_file = File::create(kmp_file_path)?;

    kmp.write(&mut kmp_file).context("could not write kmp file")?;

    Ok(())
}

fn handle_save_kmp_errors(In(result): In<anyhow::Result<()>>) {
    if let Err(err) = result {
        dbg!(err);
    }
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
    let visib = if **ev { Visibility::Visible } else { Visibility::Hidden };

    for mut visibility in q.iter_mut() {
        *visibility = visib;
    }
}

fn update_visible_on_mode_change<T: Component>(
    mode: Res<KmpEditMode>,
    mut ev_set_sect_visibility: EventWriter<SetSectionVisibility<T>>,
) {
    if !mode.is_changed() {
        return;
    }
    ev_set_sect_visibility.send(SetSectionVisibility::new(mode.in_mode::<T>()));
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
