pub mod checkpoints;
pub mod components;
pub mod csv;
pub mod document;
pub mod meshes_materials;
pub mod ordering;
pub mod path;
pub mod point;
pub mod preservation;
mod rebuild;
pub mod routes;
pub mod sections;
pub mod settings;

use self::{
    checkpoints::{checkpoint_plugin, spawn_checkpoint_section},
    components::Spawn,
    components::*,
    meshes_materials::{setup_kmp_meshes_materials, update_checkpoint_plane_culling},
    path::{spawn_enemy_item_path_section, RecalcPaths},
    point::{spawn_point_section, AddRespawnPointPreview},
};
use crate::{
    ui::{
        file_dialog::{DialogType, FileDialogResult},
        settings::{AppSettings, SetupAppSettingsSet},
        ui_state::KmpFilePath,
        update_ui::{FileLoadSet, KclFileSelected, KmpFileSelected},
    },
    util::kmp_file::*,
};
use anyhow::{bail, Context};
use bevy::{ecs::entity::EntityHashMap, platform::collections::HashMap, prelude::*};
use derive_new::new;
use ordering::{ordering_plugin, RefreshOrdering};
use path::{path_plugin, EntityPathGroups};
use point::save_point_section;
use routes::{routes_plugin, spawn_route_section};
use sections::{add_for_all_components, section_plugin, KmpEditMode};
use std::{ffi::OsStr, io::Cursor, marker::PhantomData, path::PathBuf};

pub fn kmp_plugin(app: &mut App) {
    app.add_plugins((
        checkpoint_plugin,
        path_plugin,
        ordering_plugin,
        section_plugin,
        routes_plugin,
    ))
    .add_message::<SaveFile>()
    .add_message::<OpenKmpRequest>()
    .add_message::<ResetSectionVisibilities>()
    .init_resource::<SaveStatus>()
    .add_systems(Startup, setup_kmp_meshes_materials.after(SetupAppSettingsSet))
    // Save after UI commands, deletions, and route repair have all been applied;
    // otherwise a same-frame Save could serialize a half-finished structural edit.
    .add_systems(Last, save_kmp)
    .add_systems(
        Update,
        (
            open_kmp
                .pipe(handle_open_kmp_errors)
                .run_if(on_message::<KmpFileSelected>)
                .in_set(FileLoadSet::Load),
            open_kmp_kcl.in_set(FileLoadSet::Select),
            guard_open_kmp.after(open_kmp_kcl).in_set(FileLoadSet::Select),
            update_checkpoint_plane_culling,
        ),
    );

    add_for_all_components!(@event app, SetSectionVisibility);
    app.add_message::<SetSectionVisibility<TrackInfo>>();
    add_for_all_components!(@system app, update_visible_on_mode_change);
    add_for_all_components!(@system app, set_section_visibility);
}

#[derive(Message)]
struct OpenKmpRequest {
    path: PathBuf,
    companion_kcl: Option<PathBuf>,
}

/// Convert file-dialog results into ordinary saves, KCL-only loads, or guarded
/// KMP replacement requests. MessageReader leaves settings dialog results visible
/// to their own consumer.
fn open_kmp_kcl(
    mut results: MessageReader<FileDialogResult>,
    mut saves: MessageWriter<SaveFile>,
    mut kcl_files: MessageWriter<KclFileSelected>,
    mut kmp_requests: MessageWriter<OpenKmpRequest>,
    settings: Res<AppSettings>,
) {
    for FileDialogResult { path, dialog_type } in results.read() {
        match dialog_type {
            DialogType::SaveKmp => {
                saves.write(SaveFile(Some(path.clone())));
            }
            DialogType::OpenKmpKcl => match path.extension().and_then(OsStr::to_str) {
                Some("kmp") => {
                    let companion_kcl = settings
                        .open_course_kcl_in_dir
                        .then(|| {
                            let mut kcl = path.clone();
                            kcl.set_file_name("course.kcl");
                            kcl
                        })
                        .filter(|kcl| kcl.exists());
                    kmp_requests.write(OpenKmpRequest {
                        path: path.clone(),
                        companion_kcl,
                    });
                }
                Some("kcl") => {
                    kcl_files.write(KclFileSelected(path.clone()));
                }
                _ => {}
            },
            DialogType::ExportSettings | DialogType::ImportSettings => {}
        }
    }
}

/// Dirty-state inspection needs exclusive world access, so guard the lightweight
/// request message in a second system after dialog routing.
fn guard_open_kmp(world: &mut World) {
    let requests: Vec<_> = world.resource_mut::<Messages<OpenKmpRequest>>().drain().collect();
    for request in requests {
        crate::ui::unsaved_changes::request(
            world,
            crate::ui::unsaved_changes::DocumentAction::OpenKmp {
                path: request.path,
                companion_kcl: request.companion_kcl,
            },
        );
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

fn despawn_kmp_points(world: &mut World) {
    let entities: Vec<_> = world
        .query_filtered::<Entity, With<KmpSelectablePoint>>()
        .iter(world)
        .collect();
    for entity in entities {
        // Despawning one checkpoint half also despawns its partner, which may
        // still be present in this snapshot.
        if let Ok(entity) = world.get_entity_mut(entity) {
            entity.despawn();
        }
    }
}

/// Parse before clearing the current course, then build the editable entities and
/// their baseline together. This same path refreshes indices after a rebuild save.
pub fn open_kmp(world: &mut World) -> anyhow::Result<()> {
    let Some(ev) = world.resource_mut::<Messages<KmpFileSelected>>().drain().last() else {
        return Ok(());
    };
    // if the file extension is not 'kmp' return
    if ev.extension() != Some(OsStr::new("kmp")) {
        bail!("file extension was not .kmp")
    }

    // open the KMP file and read it
    let loaded_path = ev.0.clone();
    let original_bytes = std::fs::read(&loaded_path).context("could not open kmp file")?;
    let kmp = KmpFile::read(&mut Cursor::new(&original_bytes)).context("could not read kmp file")?;
    let stgi = kmp.stgi.first().context("KMP has no required STGI track information")?;

    // get rid of all kmp points we may currently have in the world
    despawn_kmp_points(world);
    world.remove_resource::<EntityPathGroups<EnemyPathPoint>>();
    world.remove_resource::<EntityPathGroups<ItemPathPoint>>();
    world.remove_resource::<EntityPathGroups<Checkpoint>>();

    world.init_resource::<KmpErrors>();

    world.remove_resource::<document::LoadedKmp>();
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

    if let Some(e) = camera_id_map.get(&(intro_start as u32)) {
        world.entity_mut(*e).insert(KmpCameraIntroStart);
    }

    // --- CANNON POINTS ---
    spawn_point_section::<CannonPoint>(world, &kmp);

    // --- FINISH POINTS ---
    spawn_point_section::<BattleFinishPoint>(world, &kmp);

    world.write_message(RecalcPaths::all());

    // Normalize now rather than waiting for a later frame: the first save must
    // compare against the settled imported model, not transient spawn ordering.
    document::normalize_ordering(world, &kmp)?;
    let document = document::LoadedKmp::capture(world, loaded_path.clone(), original_bytes, kmp)?;
    world.insert_resource(document);
    world.insert_resource(KmpFilePath(loaded_path));
    // Loading itself is not a status that needs a permanent banner. Save failures
    // remain visible, while save-mode guidance lives next to its Settings toggle.
    world.insert_resource(SaveStatus::default());

    world.remove_resource::<KmpErrors>();
    world.remove_resource::<KmpSectionIdEntityMap<RoutePoint>>();
    world.remove_resource::<KmpSectionIdEntityMap<RespawnPoint>>();

    world.write_message(RefreshOrdering);

    Ok(())
}

fn handle_open_kmp_errors(In(result): In<anyhow::Result<()>>, mut status: ResMut<SaveStatus>) {
    if let Err(err) = result {
        status.0 = format!("Open failed: {err:#}");
        warn!("{}", status.0);
    }
}

#[derive(Resource, Deref, DerefMut, Clone, Default, new)]
pub struct KmpSectionEntityIdMap<T: Component>(#[deref] pub EntityHashMap<u16>, PhantomData<T>);

/// A save request captures an explicit Save As destination, or uses the current
/// file for ordinary Save. The shared service applies the user's save-mode setting.
#[derive(Message)]
pub struct SaveFile(pub Option<PathBuf>);

/// Last load/save result shown in the menu area, including actionable failures.
#[derive(Resource, Default)]
pub struct SaveStatus(pub String);

/// Drain requests explicitly: a newly-created MessageReader would replay saves.
pub fn save_kmp(world: &mut World) {
    let requests: Vec<_> = world.resource_mut::<Messages<SaveFile>>().drain().collect();
    for SaveFile(path) in requests {
        // Capture the mode before a rebuild refreshes the loaded editor document.
        let patch = world
            .get_resource::<AppSettings>()
            .is_some_and(|settings| settings.patch_saving);
        let result = document::save(world, path);
        let completion = result
            .as_ref()
            .map(|_| ())
            .map_err(|error| format!("Save failed: {error:#}"));
        let status = match &result {
            Ok(path) => format!(
                "Saved {} ({}).",
                path.display(),
                if patch {
                    "patch; original layout preserved"
                } else {
                    "rebuilt; indices refreshed, unused source data discarded"
                }
            ),
            Err(error) => format!("Save failed: {error:#}"),
        };
        world.insert_resource(SaveStatus(status));
        // This is a no-op for ordinary saves. A modal-triggered save either
        // continues the queued open/exit action or returns the error to the modal.
        crate::ui::unsaved_changes::complete_save(world, completion);
    }
}

#[derive(Message, Deref, new)]
pub struct SetSectionVisibility<T>(#[deref] pub bool, PhantomData<T>);

fn set_section_visibility<T: Component>(
    mut ev_set_sect_visibility: MessageReader<SetSectionVisibility<T>>,
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

#[derive(Message, Default)]
pub struct ResetSectionVisibilities;

fn update_visible_on_mode_change<T: Component>(
    mode: Res<KmpEditMode>,
    settings: Res<AppSettings>,
    mut reset: MessageReader<ResetSectionVisibilities>,
    mut ev_set_sect_visibility: MessageWriter<SetSectionVisibility<T>>,
) {
    let reset_requested = reset.read().next().is_some();
    if !reset_requested && (!mode.is_changed() || settings.preserve_visibility_on_section_select) {
        return;
    }
    ev_set_sect_visibility.write(SetSectionVisibility::new(mode.in_mode::<T>()));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::viewer::kmp::checkpoints::{CheckpointLeft, CheckpointRight};

    #[test]
    fn despawning_kmp_points_handles_checkpoint_partner_cleanup() {
        let mut app = App::new();
        app.add_plugins(checkpoint_plugin);

        let line = app.world_mut().spawn_empty().id();
        let plane = app.world_mut().spawn_empty().id();
        let arrow = app.world_mut().spawn_empty().id();
        let left = app.world_mut().spawn(KmpSelectablePoint).id();
        let right = app.world_mut().spawn(KmpSelectablePoint).id();

        app.world_mut().entity_mut(left).insert(CheckpointLeft {
            right,
            line,
            plane,
            arrow,
        });
        app.world_mut()
            .entity_mut(right)
            .insert(CheckpointRight { left, line, plane });

        despawn_kmp_points(app.world_mut());

        for entity in [left, right, line, plane, arrow] {
            assert!(app.world().get_entity(entity).is_err());
        }
    }
}
