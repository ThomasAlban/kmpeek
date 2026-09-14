//! Loaded-document identity snapshots and atomic patch/rebuild saving.
use super::checkpoints::{CheckpointLeft, CheckpointRespawnLink, CheckpointRight};
use super::ordering::{refresh_order, NextOrderID, OrderId};
use super::path::KmpPathNode;
use super::routes::RouteLink;
use super::*;
use anyhow::{ensure, Result};
use bevy::ecs::system::RunSystemOnce;
use std::{
    fs::{self, OpenOptions, Permissions},
    io::Write,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

/// The loaded source and the editor's initial interpretation of it are separate
/// snapshots: importing can normalize rotations, flags, and ordering. Comparing
/// later editor values with `baseline`, rather than `original`, prevents those
/// import conversions from becoming unintended edits to the source bytes.
/// Patch saves keep these snapshots fixed; a successful rebuild reloads its
/// committed output to establish new source identities and a fresh baseline.
#[derive(Resource, Clone)]
pub struct LoadedKmp {
    /// Physical source layout, including bytes the parser does not model.
    pub original_bytes: Vec<u8>,
    /// Parsed source values, before editor normalization.
    pub original: KmpFile,
    /// Initial export of the normalized editor, used to detect user changes.
    pub baseline: KmpFile,
    signature: StructuralSignature,
    // Reverse import maps retain source IDs, not dense editor row indices.
    // Empty routes can leave holes in these IDs without creating entities.
    route_ids: EntityHashMap<u32>,
    respawn_ids: EntityHashMap<u32>,
    route_links: EntityHashMap<Entity>,
    respawn_links: EntityHashMap<Entity>,
    // Track every successfully written destination, including the opened file.
    disk_versions: Vec<(PathBuf, Vec<u8>)>,
    permissions: Permissions,
}

/// Entity identity, export order, and connectivity that field-only patches must
/// not change. Rebuilding these would require reallocating records and updating
/// references and metadata, which cannot be done by patching scalar fields.
#[derive(Clone, Debug, PartialEq, Eq)]
struct StructuralSignature {
    sections: Vec<(&'static str, Vec<Entity>)>,
    graph: Vec<(Entity, Vec<Entity>, Vec<Entity>, bool)>,
    checkpoint_pairs: Vec<(Entity, Entity)>,
}

/// Sort by editor order, using entity identity to break ties deterministically.
/// Missing order components remain visible here so snapshot validation can fail.
fn ordered<T: Component>(world: &mut World) -> Vec<Entity> {
    let mut entries: Vec<_> = world
        .query_filtered::<(Entity, Option<&OrderId>), With<T>>()
        .iter(world)
        .map(|(e, order)| (order.map(|o| o.0), e))
        .collect();
    entries.sort_unstable();
    entries.into_iter().map(|(_, e)| e).collect()
}

impl StructuralSignature {
    /// Record structure without editable values. Sort adjacency sets and query
    /// results so ECS iteration order cannot masquerade as a topology change.
    fn capture(world: &mut World) -> Self {
        let mut sections = Vec::new();
        macro_rules! section { ($($ty:ty),*) => { $(sections.push((stringify!($ty), ordered::<$ty>(world)));)* }; }
        section!(
            StartPoint,
            EnemyPathPoint,
            ItemPathPoint,
            Checkpoint,
            CheckpointRight,
            Object,
            RoutePoint,
            RouteSettings,
            AreaPoint,
            KmpCamera,
            RespawnPoint,
            CannonPoint,
            BattleFinishPoint
        );
        let mut graph: Vec<_> = world
            .query::<(Entity, Option<&KmpPathNode>, Has<PathOverallStart>)>()
            .iter(world)
            .filter(|(_, node, start)| node.is_some() || *start)
            .map(|(e, node, start)| {
                let mut prev: Vec<_> = node.into_iter().flat_map(|n| n.prev_nodes.iter().copied()).collect();
                let mut next: Vec<_> = node.into_iter().flat_map(|n| n.next_nodes.iter().copied()).collect();
                prev.sort_unstable();
                next.sort_unstable();
                (e, prev, next, start)
            })
            .collect();
        graph.sort_unstable();
        let mut checkpoint_pairs: Vec<_> = world
            .query::<(Entity, &CheckpointLeft)>()
            .iter(world)
            .map(|(e, pair)| (e, pair.right))
            .collect();
        checkpoint_pairs.sort_unstable();
        Self {
            sections,
            graph,
            checkpoint_pairs,
        }
    }

    /// Refuse structural changes before any export can misassociate source rows.
    fn validate(&self, current: &Self) -> Result<()> {
        for ((name, before), (_, after)) in self.sections.iter().zip(&current.sections) {
            ensure!(before == after, "Structural saving is not supported in patch mode: {name} points were added, removed, or reordered. No file was written.");
        }
        ensure!(self.graph == current.graph && self.checkpoint_pairs == current.checkpoint_pairs,
            "Structural saving is not supported in patch mode: path topology, checkpoint pairing, or overall start changed. No file was written.");
        Ok(())
    }
}

/// Enumerate source indices in importer spawn order, not physical point order.
/// Unreferenced records are absent and overlapping groups repeat indices; later
/// snapshot assembly preserves the former and checks duplicate edits for agreement.
fn source_indices<T: Default>(groups: &[PathGroup<T>], count: usize) -> Vec<usize> {
    groups
        .iter()
        .flat_map(|group| {
            let start = usize::from(group.start);
            let end = start + usize::from(group.group_length);
            // Match the importer's handling of invalid ranges.
            if end <= count {
                start..end
            } else {
                0..0
            }
        })
        .collect()
}

/// Restore source-relative ordering for grouped points, then assign the editor's
/// dense order IDs synchronously before capturing the baseline. Checkpoint halves
/// share a source index. Dense IDs are display/export order, not file references.
pub fn normalize_ordering(world: &mut World, kmp: &KmpFile) -> Result<()> {
    macro_rules! source_order {
        ($ty:ty, $points:ident, $groups:ident) => {{
            let entities = ordered::<$ty>(world);
            let indices = source_indices(&kmp.$groups, kmp.$points.len());
            for (e, index) in entities.into_iter().zip(indices) {
                world.entity_mut(e).insert(OrderId(index as u32));
                if let Some(right) = world.get::<CheckpointLeft>(e).map(|p| p.right) {
                    world.entity_mut(right).insert(OrderId(index as u32));
                }
            }
        }};
    }
    source_order!(EnemyPathPoint, enpt, enph);
    source_order!(ItemPathPoint, itpt, itph);
    source_order!(Checkpoint, ckpt, ckph);
    macro_rules! normalize { ($($ty:ty),*) => { $(
        world.init_resource::<NextOrderID<$ty>>();
        world.run_system_once(refresh_order::<$ty>).map_err(|e| anyhow::anyhow!("Could not normalize ordering: {e:?}"))?;
    )* }; }
    normalize!(
        StartPoint,
        EnemyPathPoint,
        ItemPathPoint,
        Checkpoint,
        RoutePoint,
        Object,
        AreaPoint,
        KmpCamera,
        RespawnPoint,
        CannonPoint,
        BattleFinishPoint
    );
    Ok(())
}

impl LoadedKmp {
    /// Find the source row behind an imported entity. Editor OrderIds can change
    /// after deletions, so reference remapping must use the frozen load-time list.
    /// Grouped sections may have holes or aliases; their raw indices are not the
    /// positions in that list.
    pub(crate) fn original_index(&self, section: &str, entity: Entity) -> Option<usize> {
        let entities = &self.signature.sections.iter().find(|(name, _)| *name == section)?.1;
        let position = entities.iter().position(|&candidate| candidate == entity)?;
        let mut indices = match section {
            "EnemyPathPoint" => source_indices(&self.original.enph, self.original.enpt.len()),
            "ItemPathPoint" => source_indices(&self.original.itph, self.original.itpt.len()),
            "Checkpoint" => source_indices(&self.original.ckph, self.original.ckpt.len()),
            _ => return Some(position),
        };
        indices.sort_unstable();
        indices.get(position).copied()
    }

    /// Resolve an old numeric reference to the same entity even if it has moved
    /// to a different editor row. An unrepresented source point returns None so
    /// rebuild validation can report an unresolved reference, not retarget it.
    pub(crate) fn source_entity(&self, section: &str, index: usize) -> Option<Entity> {
        self.signature
            .sections
            .iter()
            .find(|(name, _)| *name == section)?
            .1
            .iter()
            .copied()
            .find(|&entity| self.original_index(section, entity) == Some(index))
    }

    /// Capture after spawning and order normalization, while import ID maps still
    /// identify source records. Save initial links too: an unresolved raw reference
    /// must not be mistaken for a user clearing or changing a link.
    pub fn capture(world: &mut World, path: PathBuf, original_bytes: Vec<u8>, original: KmpFile) -> Result<Self> {
        let route_ids = world
            .resource::<KmpSectionIdEntityMap<RoutePoint>>()
            .iter()
            .map(|(&id, &e)| (e, id))
            .collect();
        let respawn_ids = world
            .resource::<KmpSectionIdEntityMap<RespawnPoint>>()
            .iter()
            .map(|(&id, &e)| (e, id))
            .collect();
        let route_links = world
            .query::<(Entity, &RouteLink)>()
            .iter(world)
            .map(|(e, l)| (e, l.0))
            .collect();
        let respawn_links = world
            .query::<(Entity, &CheckpointRespawnLink)>()
            .iter(world)
            .map(|(e, l)| (e, l.0))
            .collect();
        let mut document = Self {
            permissions: fs::metadata(&path)?.permissions(),
            disk_versions: vec![(fs::canonicalize(&path)?, original_bytes.clone())],
            original_bytes,
            original,
            baseline: KmpFile::default(),
            signature: StructuralSignature::capture(world),
            route_ids,
            respawn_ids,
            route_links,
            respawn_links,
        };
        document.baseline = document.snapshot(world)?;
        Ok(document)
    }

    /// Validate changed references before exporters narrow their numeric IDs.
    /// Objects use 16-bit route IDs; other route references use 8 bits, with the
    /// all-ones value reserved for no route. Respawn IDs allow the full byte range.
    /// Unchanged unsupported references are preserved, but changing one requires
    /// both its old and new targets to be representable by this save path.
    fn validate_links(&self, world: &mut World) -> Result<()> {
        for (&e, &target) in &self.route_links {
            if world.get::<RouteLink>(e).map(|l| l.0) != Some(target) {
                ensure!(
                    self.route_ids
                        .get(&target)
                        .is_some_and(|id| *id < if world.get::<Object>(e).is_some() { 65535 } else { 255 }),
                    "Cannot change a route link whose original ID is outside this section's supported range"
                );
            }
        }
        for (&e, &target) in &self.respawn_links {
            ensure!(
                world.get::<CheckpointRespawnLink>(e).is_some(),
                "Removing a checkpoint respawn link is not supported by field-only saving"
            );
            if world.get::<CheckpointRespawnLink>(e).map(|l| l.0) != Some(target) {
                ensure!(
                    self.respawn_ids.get(&target).is_some_and(|id| *id <= 255),
                    "Cannot change a respawn link whose original ID is outside the supported 8-bit range"
                );
            }
        }
        for (e, link) in world.query::<(Entity, &RouteLink)>().iter(world) {
            if self.route_links.get(&e) != Some(&link.0) {
                let id = self
                    .route_ids
                    .get(&link.0)
                    .context("Changed route link targets an unknown route")?;
                ensure!(
                    *id < if world.get::<Object>(e).is_some() { 65535 } else { 255 },
                    "Changed route link targets route {id}, outside this section's supported range"
                );
            }
        }
        for (e, link) in world.query::<(Entity, &CheckpointRespawnLink)>().iter(world) {
            if self.respawn_links.get(&e) != Some(&link.0) {
                let id = self
                    .respawn_ids
                    .get(&link.0)
                    .context("Changed respawn link targets an unknown point")?;
                ensure!(
                    *id <= 255,
                    "Changed respawn link targets point {id}, outside the supported 8-bit range"
                );
            }
        }
        Ok(())
    }

    /// Export editable values into a source-shaped snapshot, not a regenerated
    /// graph file. The original supplies unrepresented records and metadata;
    /// preservation::patch later decides which exported values actually changed.
    /// This also installs source-ID lookup resources needed by point exporters.
    pub fn snapshot(&self, world: &mut World) -> Result<KmpFile> {
        self.signature.validate(&StructuralSignature::capture(world))?;
        for (section, entities) in &self.signature.sections {
            for &entity in entities {
                ensure!(
                    world.get::<Transform>(entity).is_some() && world.get::<OrderId>(entity).is_some(),
                    "Cannot save {section}: a point is missing its transform or ordering component"
                );
            }
        }
        for &(entity, _, _, _) in &self.signature.graph {
            ensure!(
                world.get::<KmpPathNode>(entity).is_some(),
                "Structural saving is not supported in patch mode: a path node component was removed"
            );
        }
        self.validate_links(world)?;
        // Keep original IDs, including holes from empty routes. Never use dense
        // save_point_section route IDs for references. Unchanged unrepresentable
        // values are retained by the original/baseline merge.
        world.insert_resource(KmpSectionEntityIdMap::<RouteSettings>::new(
            self.route_ids
                .iter()
                .filter_map(|(&e, &id)| u16::try_from(id).ok().map(|id| (e, id)))
                .collect(),
        ));
        world.insert_resource(KmpSectionEntityIdMap::<RespawnPoint>::new(
            self.respawn_ids
                .iter()
                .filter_map(|(&e, &id)| u16::try_from(id).ok().map(|id| (e, id)))
                .collect(),
        ));
        let mut kmp = self.original.clone();
        macro_rules! points { ($($field:ident: $ty:ty),*) => { $(kmp.$field = save_point_section::<$ty>(world).0;)* }; }
        points!(ktpt: StartPoint, gobj: Object, area: AreaPoint, came: KmpCamera,
            jgpt: RespawnPoint, cnpt: CannonPoint, mspt: BattleFinishPoint);
        // Unreferenced points are not spawned by the importer. Leave their raw
        // records in place while mapping represented points back to source IDs.
        macro_rules! path_points {
            ($ty:ty, $points:ident, $groups:ident) => {{
                let (section, _) = save_point_section::<$ty>(world);
                let mut indices = source_indices(&self.original.$groups, self.original.$points.len());
                indices.sort_unstable();
                ensure!(
                    section.len() == indices.len(),
                    "{} snapshot point count changed",
                    stringify!($points)
                );
                let entities = ordered::<$ty>(world);
                let mut represented = HashMap::<usize, String>::default();
                for ((mut point, index), entity) in section.entries.into_iter().zip(indices).zip(entities) {
                    // Checkpoint adjacency is structural, not an editable scalar.
                    // Preserve unresolved respawn bytes unless the entity link changed;
                    // otherwise exporter defaults could silently replace them with zero.
                    if let Some(cp) = (&mut point as &mut dyn std::any::Any).downcast_mut::<Ckpt>() {
                        cp.prev_cp = self.original.ckpt[index].prev_cp;
                        cp.next_cp = self.original.ckpt[index].next_cp;
                        if world.get::<CheckpointRespawnLink>(entity).map(|link| link.0)
                            == self.respawn_links.get(&entity).copied()
                        {
                            cp.respawn_pos = self.original.ckpt[index].respawn_pos;
                        }
                    }
                    // Overlapping groups can expose a source record more than
                    // once. Preserve them on load/no-op, but never silently pick
                    // one of two conflicting edits to the same record.
                    let value = serde_json::to_string(&point)?;
                    if let Some(previous) = represented.insert(index, value.clone()) {
                        ensure!(
                            previous == value,
                            "Conflicting edits to shared {} point {index}; no file was written",
                            stringify!($points)
                        );
                    }
                    kmp.$points[index] = point;
                }
            }};
        }
        path_points!(EnemyPathPoint, enpt, enph);
        path_points!(ItemPathPoint, itpt, itph);
        path_points!(Checkpoint, ckpt, ckph);
        // Replace only represented routes at their original IDs; empty routes
        // have no editor entities but must retain their slots and metadata.
        for (&e, &id) in &self.route_ids {
            let settings = world.get::<RouteSettings>(e).context("Route settings missing")?.clone();
            let transform = *world.get::<Transform>(e).context("Route transform missing")?;
            kmp.poti[id as usize] = settings.to_kmp(transform, world, e);
        }
        let cameras = ordered::<KmpCamera>(world);
        let intros: Vec<_> = world
            .query_filtered::<Entity, With<KmpCameraIntroStart>>()
            .iter(world)
            .collect();
        ensure!(intros.len() <= 1, "Multiple intro camera starts cannot be saved");
        let intro = if let Some(e) = intros.first() {
            let id = cameras
                .iter()
                .position(|camera| camera == e)
                .context("Intro start is not a camera")?;
            ensure!(id < 255, "Intro camera index is outside the supported range");
            id as u16
        } else {
            255
        };
        // CAME packs the intro index into the high byte (255 means none).
        // Its low byte is unrelated metadata and is never regenerated here.
        kmp.came.section_header.additional_value =
            (intro << 8) | (self.original.came.section_header.additional_value & 0xff);
        // The UI represents only the first track-info record; retain any extras.
        kmp.stgi[0] = world
            .get_resource::<TrackInfo>()
            .context("Track information is missing")?
            .clone()
            .to_kmp(Transform::default(), world, Entity::PLACEHOLDER);
        Ok(kmp)
    }
}

/// Report whether the live editor would produce different persisted KMP bytes.
/// This deliberately projects through patch mode regardless of the selected save
/// mode: merely toggling a preference must not make an untouched document dirty.
/// Structural or invalid editor state returns an error so callers conservatively
/// prompt instead of allowing data loss.
pub fn has_unsaved_changes(world: &mut World) -> Result<bool> {
    let Some(document) = world.get_resource::<LoadedKmp>().cloned() else {
        return Ok(false);
    };
    let current = document.snapshot(world)?;
    let projected = preservation::patch(
        &document.original_bytes,
        &document.original,
        &document.baseline,
        &current,
    )?;
    let path = world
        .get_resource::<KmpFilePath>()
        .context("Loaded KMP has no active file path")?;
    let active = resolved_destination(&path.0)?;
    let saved = document
        .disk_versions
        .iter()
        .find(|(path, _)| path == &active)
        .map(|(_, bytes)| bytes)
        .context("Active KMP has no saved-byte baseline")?;
    Ok(&projected != saved)
}

/// Save or Save As through the selected encoder and a shared atomic commit.
/// Patch saves keep their original/baseline pair so reverted edits restore original
/// bytes. Rebuild saves refresh the editor only after the new file is committed.
pub fn save(world: &mut World, destination: Option<PathBuf>) -> Result<PathBuf> {
    let path = destination
        .or_else(|| world.get_resource::<KmpFilePath>().map(|p| p.0.clone()))
        .context("No KMP is loaded; open a file before saving")?;
    ensure!(
        path.extension().is_some_and(|extension| extension == "kmp"),
        "Save destination must have the .kmp extension"
    );
    let document = world
        .get_resource::<LoadedKmp>()
        .context("No lossless document snapshot is available; reopen the KMP")?
        .clone();
    // If settings are unavailable (for example in a headless integration), use
    // the normal rebuild behavior rather than silently enabling patch mode.
    let patch_saving = world
        .get_resource::<AppSettings>()
        .is_some_and(|settings| settings.patch_saving);
    let bytes = if patch_saving {
        let current = document.snapshot(world).map_err(|error| {
            anyhow::anyhow!("{error:#}. Turn off Patch saving in Settings → General to rebuild the KMP")
        })?;
        preservation::patch(
            &document.original_bytes,
            &document.original,
            &document.baseline,
            &current,
        )?
    } else {
        // Rebuild creates a complete checked model before any disk I/O. The
        // canonical encoder packs records and updates all header lengths/counts.
        let rebuilt = rebuild::rebuild(world, &document)?;
        let mut encoded = Cursor::new(Vec::new());
        rebuilt.write(&mut encoded)?;
        let bytes = encoded.into_inner();
        KmpFile::read(&mut Cursor::new(&bytes)).context("Rebuilt KMP failed binary validation")?;
        bytes
    };
    let target = resolved_destination(&path)?;
    let expected = document
        .disk_versions
        .iter()
        .find(|(p, _)| p == &target)
        .map(|(_, b)| b.as_slice());
    atomic_replace(&target, &bytes, expected, &document.permissions)?;
    let mut document = world.resource_mut::<LoadedKmp>();
    if let Some((_, version)) = document.disk_versions.iter_mut().find(|(p, _)| p == &target) {
        *version = bytes;
    } else {
        document.disk_versions.push((target, bytes));
    }
    world.insert_resource(KmpFilePath(path.clone()));
    if !patch_saving {
        // A structural rebuild changes the meaning of numeric row IDs. Reload
        // the committed bytes so controls, references, and the next patch baseline
        // all use those new IDs. This deliberately clears the old selection.
        // Disk-version history survives the reload to retain overwrite protection
        // when switching back to a destination previously saved in this session.
        let versions = world.resource::<LoadedKmp>().disk_versions.clone();
        world.write_message(KmpFileSelected(path.clone()));
        open_kmp(world).context("KMP was saved, but refreshing the editor failed; reopen the saved file")?;
        world.resource_mut::<LoadedKmp>().disk_versions = versions;
    }
    Ok(path)
}

/// Resolve existing symlinks and relative paths for destination tracking. For a
/// new file only the parent exists, so canonicalize it and retain the filename.
/// This is path normalization, not a stable file identity or filesystem lock.
fn resolved_destination(path: &Path) -> Result<PathBuf> {
    if path.exists() {
        return Ok(fs::canonicalize(path)?);
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    Ok(fs::canonicalize(parent)?.join(path.file_name().context("Save destination has no filename")?))
}

// Process-local name suffix; create_new, not the counter, guarantees exclusivity.
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Never truncate the destination. Write a same-directory temporary file, apply
/// permissions, sync and close it, then rename; pre-rename failures clean it up.
/// Existing destination permissions take precedence over loaded-file permissions
/// for Save As. Other filesystem metadata (such as ACLs/xattrs) is not copied.
///
/// Byte checks protect known destinations against observed external changes,
/// not concurrent writers: there is still a check-to-rename race, and an unknown
/// Save As destination has no expected version. File syncing is not directory
/// syncing, so this does not promise rename durability across a power failure.
fn atomic_replace(
    path: &Path,
    bytes: &[u8],
    expected: Option<&[u8]>,
    fallback_permissions: &Permissions,
) -> Result<()> {
    let check_conflict = || -> Result<()> {
        if let Some(expected) = expected {
            ensure!(fs::read(path).context("Cannot verify loaded file on disk")? == expected,
                "The KMP changed on disk since it was opened or last saved. Reopen it or Save As to another file; no file was written.");
        }
        Ok(())
    };
    check_conflict()?;
    let permissions = match fs::metadata(path) {
        Ok(metadata) => metadata.permissions(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => fallback_permissions.clone(),
        Err(error) => return Err(error.into()),
    };
    let parent = path.parent().context("Save destination has no parent directory")?;
    let (temp, mut file) = loop {
        let id = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let temp = parent.join(format!(".kmpeek-{}-{id}.tmp", std::process::id()));
        match OpenOptions::new().write(true).create_new(true).open(&temp) {
            Ok(file) => break (temp, file),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error).context("Could not create temporary save file"),
        }
    };
    let result = (|| -> Result<()> {
        file.write_all(bytes).context("Could not write temporary save file")?;
        file.set_permissions(permissions)?;
        file.sync_all().context("Could not sync temporary save file")?;
        drop(file);
        // Recheck after potentially slow I/O, narrowing (not eliminating) the
        // window in which an external edit could be overwritten by the rename.
        check_conflict()?;
        fs::rename(&temp, path).context("Could not atomically replace KMP; original file was not truncated")?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Isolate disk tests from course data; remove temporary artifacts on drop.
    struct TestDir(PathBuf);
    impl TestDir {
        /// Use the process ID and shared counter to separate concurrent tests.
        fn new() -> Self {
            let id = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!("kmpeek-document-test-{}-{id}", std::process::id()));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// Supply importer/editor resources without creating a window or renderer.
    fn headless_app() -> App {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<AppSettings>()
            .add_message::<KmpFileSelected>()
            .add_message::<SaveFile>()
            .add_plugins((checkpoint_plugin, path_plugin, ordering_plugin, routes_plugin));
        app.world_mut().resource_mut::<AppSettings>().patch_saving = true;
        app.world_mut().run_system_once(setup_kmp_meshes_materials).unwrap();
        app
    }

    /// Exercise source-order restoration, an empty route ID slot, extra STGI
    /// records, and checkpoint/group metadata the editor must not regenerate.
    fn fixture() -> KmpFile {
        let mut kmp = KmpFile::default();
        kmp.stgi = Section::new(vec![
            Stgi::default(),
            Stgi {
                lap_count: 7,
                ..default()
            },
        ]);
        kmp.ktpt = Section::new(vec![Ktpt::default()]);
        kmp.jgpt = Section::new(vec![Jgpt::default()]);
        kmp.ckpt = Section::new(vec![Ckpt {
            prev_cp: 17,
            next_cp: 23,
            ..default()
        }]);
        kmp.ckph = Section::new(vec![PathGroup::new(0, 1, [255; 6], [255; 6], 789)]);
        kmp.enpt = Section::new(vec![
            Enpt {
                leniency: 10.,
                ..default()
            },
            Enpt {
                leniency: 20.,
                ..default()
            },
        ]);
        // Deliberately not in source-point order.
        kmp.enph = Section::new(vec![
            PathGroup::new(1, 1, [255; 6], [255; 6], 123),
            PathGroup::new(0, 1, [255; 6], [255; 6], 456),
        ]);
        kmp.poti = Section::new(vec![
            Poti::default(),
            Poti {
                num_points: 1,
                points: vec![PotiPoint::default()],
                ..default()
            },
        ]);
        kmp.gobj = Section::new(vec![Gobj { route: 1, ..default() }]);
        kmp
    }

    /// Encode a disposable fixture and use the real open path, returning the
    /// exact disk bytes for subsequent preservation assertions.
    fn load(app: &mut App, path: &Path, kmp: &KmpFile) -> Vec<u8> {
        let mut cursor = Cursor::new(Vec::new());
        kmp.clone().write(&mut cursor).unwrap();
        let bytes = cursor.into_inner();
        fs::write(path, &bytes).unwrap();
        app.world_mut().write_message(KmpFileSelected(path.into()));
        open_kmp(app.world_mut()).unwrap();
        bytes
    }

    /// Cover every section through real import and Save As, including normalized
    /// rotations and opaque padding; a later camera edit may change only its field.
    /// Exercise the UI mode switch through the real save service, not only the
    /// planner: rebuild renumbers references, reloads the committed document, and
    /// leaves a fresh baseline suitable for subsequent lossless patch saves.
    #[test]
    fn rebuild_mode_saves_structural_edits_and_rebases_patch_mode() {
        let dir = TestDir::new();
        let path = dir.0.join("course.kmp");
        let mut app = headless_app();
        let bytes = load(&mut app, &path, &fixture());
        let enemies = ordered::<EnemyPathPoint>(app.world_mut());
        let added = app
            .world_mut()
            .spawn((
                EnemyPathPoint {
                    leniency: 30.,
                    ..default()
                },
                Transform::IDENTITY,
                OrderId(2),
                KmpPathNode::default(),
                KmpSelectablePoint,
            ))
            .id();
        app.world_mut().entity_mut(enemies[1]).remove::<PathOverallStart>();
        app.world_mut().entity_mut(enemies[0]).insert(PathOverallStart);
        for (from, to) in [(enemies[0], added), (added, enemies[1])] {
            app.world_mut()
                .get_mut::<KmpPathNode>(from)
                .unwrap()
                .next_nodes
                .insert(to);
            app.world_mut()
                .get_mut::<KmpPathNode>(to)
                .unwrap()
                .prev_nodes
                .insert(from);
        }
        // Keep the same number of KTPT records, but replace the old identity:
        // patch mode must not confuse this with editing the old row in place.
        let old_start = ordered::<StartPoint>(app.world_mut())[0];
        app.world_mut().despawn(old_start);
        app.world_mut().spawn((
            StartPoint::default(),
            Transform::from_xyz(7., 8., 9.),
            OrderId(0),
            KmpSelectablePoint,
        ));
        assert!(save(app.world_mut(), None)
            .unwrap_err()
            .to_string()
            .contains("Turn off Patch saving"));
        assert_eq!(fs::read(&path).unwrap(), bytes);
        app.world_mut().resource_mut::<AppSettings>().patch_saving = false;
        save(app.world_mut(), None).unwrap();
        let rebuilt = fs::read(&path).unwrap();
        let parsed = KmpFile::read(&mut Cursor::new(&rebuilt)).unwrap();
        assert_eq!(parsed.ktpt[0].position, [7., 8., 9.]);
        assert_eq!(
            parsed.enpt.iter().map(|p| p.leniency).collect::<Vec<_>>(),
            vec![10., 30., 20.]
        );
        assert_eq!(parsed.enph.len(), 1);
        assert_eq!(parsed.enph[0].group_length, 3);
        assert_eq!(parsed.enph[0].group_link, 0);
        assert_eq!(parsed.poti.len(), 1); // unused empty route removed
        assert_eq!(parsed.gobj[0].route, 0); // reference follows the surviving route
        assert_eq!(parsed.stgi.len(), 1); // only the represented track record remains
        app.world_mut().resource_mut::<AppSettings>().patch_saving = true;
        save(app.world_mut(), None).unwrap();
        assert_eq!(fs::read(&path).unwrap(), rebuilt);
        let start = ordered::<StartPoint>(app.world_mut())[0];
        app.world_mut().get_mut::<Transform>(start).unwrap().translation.x = 50.;
        save(app.world_mut(), None).unwrap();
        let parsed = KmpFile::read(&mut Cursor::new(fs::read(&path).unwrap())).unwrap();
        assert_eq!(parsed.ktpt[0].position, [50., 8., 9.]);
        assert_eq!(parsed.enph[0].group_length, 3);
    }

    /// Dirty state compares projected persisted bytes with the active destination,
    /// so editor-only state is ignored and a successful patch save becomes clean.
    #[test]
    fn unsaved_changes_track_persisted_output_not_editor_only_state() {
        let dir = TestDir::new();
        let path = dir.0.join("course.kmp");
        let mut app = headless_app();
        load(&mut app, &path, &fixture());
        assert!(!has_unsaved_changes(app.world_mut()).unwrap());
        app.world_mut().resource_mut::<TrackInfo>().track_type = TrackType::Battle;
        assert!(!has_unsaved_changes(app.world_mut()).unwrap());
        app.world_mut().resource_mut::<TrackInfo>().lap_count = 7;
        assert!(has_unsaved_changes(app.world_mut()).unwrap());
        save(app.world_mut(), None).unwrap();
        assert!(!has_unsaved_changes(app.world_mut()).unwrap());
        app.world_mut().resource_mut::<TrackInfo>().lap_count = 0;
        assert!(has_unsaved_changes(app.world_mut()).unwrap());
    }

    /// Validation failures in rebuild mode must have the same disk safety as
    /// refused patch edits, including invalid numbers in newly created records.
    #[test]
    fn invalid_rebuild_never_overwrites_the_source() {
        let dir = TestDir::new();
        let path = dir.0.join("course.kmp");
        let mut app = headless_app();
        let bytes = load(&mut app, &path, &fixture());
        app.world_mut().resource_mut::<AppSettings>().patch_saving = false;
        let camera = app
            .world_mut()
            .spawn((
                KmpCamera {
                    next_index: 255,
                    duration: f32::NAN,
                    ..default()
                },
                Transform::IDENTITY,
                OrderId(0),
                KmpSelectablePoint,
            ))
            .id();
        assert!(save(app.world_mut(), None).unwrap_err().to_string().contains("NaN"));
        assert_eq!(fs::read(&path).unwrap(), bytes);
        {
            let mut camera = app.world_mut().get_mut::<KmpCamera>(camera).unwrap();
            camera.duration = 1.;
            camera.next_index = 99;
        }
        assert!(save(app.world_mut(), None)
            .unwrap_err()
            .to_string()
            .contains("does not exist"));
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert_eq!(app.world().resource::<KmpFilePath>().0, path);
    }

    /// Unrepresented source indices are holes, not references to the following
    /// visible point. This regression prevents AREA links silently changing target.
    #[test]
    fn source_identity_mapping_preserves_orphan_holes() {
        let dir = TestDir::new();
        let path = dir.0.join("course.kmp");
        let mut app = headless_app();
        let mut kmp = fixture();
        kmp.enph = Section::new(vec![PathGroup::new(1, 1, [255; 6], [255; 6], 0)]);
        load(&mut app, &path, &kmp);
        let entity = ordered::<EnemyPathPoint>(app.world_mut())[0];
        let source = app.world().resource::<LoadedKmp>();
        assert_eq!(source.source_entity("EnemyPathPoint", 0), None);
        assert_eq!(source.source_entity("EnemyPathPoint", 1), Some(entity));
        assert_eq!(source.original_index("EnemyPathPoint", entity), Some(1));
    }

    /// All represented sections survive ordinary editor update frames and Save As.
    #[test]
    fn all_sections_roundtrip_exactly_through_editor_and_save_as() {
        let dir = TestDir::new();
        let source = dir.0.join("course.kmp");
        let destination = dir.0.join("copy.kmp");
        let mut kmp = fixture();
        kmp.itpt = Section::new(vec![Itpt {
            setting_1: 0,
            setting_2: 0x45,
            ..default()
        }]);
        kmp.itph = Section::new(vec![PathGroup::new(
            0,
            1,
            [0, 255, 255, 255, 255, 255],
            [0, 255, 255, 255, 255, 255],
            0xabcd,
        )]);
        kmp.ckpt = Section::new(vec![Ckpt {
            cp_right: [100., 0.],
            respawn_pos: 0,
            cp_type: 0,
            prev_cp: 255,
            next_cp: 255,
            ..default()
        }]);
        kmp.ckph = Section::new(vec![PathGroup::new(0, 1, [255; 6], [255; 6], 0x1234)]);
        kmp.area = Section::new(vec![Area {
            kind: 9,
            setting_1: 8,
            setting_2: 0xbeef,
            ..default()
        }]);
        kmp.came = Section::new(vec![Came {
            next_index: 255,
            route: 1,
            zoom_start: 45.,
            zoom_end: 60.,
            ..default()
        }]);
        kmp.came.section_header.additional_value = 0x00ff;
        kmp.jgpt = Section::new(vec![Jgpt {
            respawn_id: 17,
            extra_data: 150,
            ..default()
        }]);
        kmp.cnpt = Section::new(vec![Cnpt {
            shoot_effect: -1,
            ..default()
        }]);
        kmp.mspt = Section::new(vec![Mspt {
            unknown: 0xabcd,
            ..default()
        }]);
        kmp.ktpt[0].rotation = [370., -20., 720.];
        kmp.gobj[0].padding = 0x1234;
        kmp.stgi[0].flare_color = [12, 34, 56, 78];
        kmp.stgi[0].padding_1 = 0x1122;
        kmp.stgi[0].padding_2 = 0x3344;
        let mut encoded = Cursor::new(Vec::new());
        kmp.write(&mut encoded).unwrap();
        let mut bytes = encoded.into_inner();
        // Fields the raw model deliberately does not interpret must survive too.
        for (section, offset) in [(0, 26), (9, 46), (12, 24), (13, 24)] {
            let table = 16 + section * 4;
            let start = 76 + u32::from_be_bytes(bytes[table..table + 4].try_into().unwrap()) as usize;
            bytes[start + 8 + offset..start + 10 + offset].copy_from_slice(&[0x5a, 0xa5]);
        }
        fs::write(&source, &bytes).unwrap();
        let mut app = headless_app();
        app.world_mut().write_message(KmpFileSelected(source.clone()));
        open_kmp(app.world_mut()).unwrap();
        for _ in 0..3 {
            app.update();
        }
        save(app.world_mut(), Some(destination.clone())).unwrap();
        assert_eq!(fs::read(&destination).unwrap(), bytes);
        assert_eq!(fs::read(&source).unwrap(), bytes);
        assert_eq!(app.world().resource::<KmpFilePath>().0, destination);
        let camera = ordered::<KmpCamera>(app.world_mut())[0];
        app.world_mut().get_mut::<KmpCamera>(camera).unwrap().zoom_start = 50.;
        save(app.world_mut(), None).unwrap();
        let saved = fs::read(&destination).unwrap();
        let parsed = KmpFile::read(&mut Cursor::new(&saved)).unwrap();
        assert_eq!(parsed.came[0].zoom_start, 50.);
        assert_eq!(parsed.came.section_header.additional_value, 0x00ff);
        let table = 16 + 10 * 4;
        let came_start = 76 + u32::from_be_bytes(bytes[table..table + 4].try_into().unwrap()) as usize;
        let mut expected = bytes;
        expected[came_start + 8 + 36..came_start + 8 + 40].copy_from_slice(&50_f32.to_be_bytes());
        assert_eq!(saved, expected);
    }

    /// A route beyond the byte range must survive import and unrelated object
    /// edits: GOBJ references are 16-bit even though other sections use bytes.
    #[test]
    fn object_route_ids_are_not_narrowed_to_eight_bits() {
        let dir = TestDir::new();
        let path = dir.0.join("course.kmp");
        let mut kmp = fixture();
        let route = kmp.poti[1].clone();
        kmp.poti = Section::new(vec![Poti::default(); 257]);
        kmp.poti[256] = route;
        kmp.gobj[0].route = 256;
        let mut app = headless_app();
        let bytes = load(&mut app, &path, &kmp);
        let object = ordered::<Object>(app.world_mut())[0];
        let route = ordered::<RoutePoint>(app.world_mut())[0];
        assert_eq!(app.world().get::<RouteLink>(object).unwrap().0, route);
        save(app.world_mut(), None).unwrap();
        assert_eq!(fs::read(&path).unwrap(), bytes);
        app.world_mut().get_mut::<Object>(object).unwrap().settings[0] = 17;
        save(app.world_mut(), None).unwrap();
        let parsed = KmpFile::read(&mut Cursor::new(fs::read(&path).unwrap())).unwrap();
        assert_eq!(parsed.gobj[0].route, 256);
        assert_eq!(parsed.gobj[0].settings[0], 17);
    }

    /// Multiple entities may represent one raw point; no-op saves are valid,
    /// but disagreement between their edits must not arbitrarily choose a winner.
    #[test]
    fn overlapping_groups_roundtrip_but_conflicting_edits_are_refused() {
        let dir = TestDir::new();
        let path = dir.0.join("course.kmp");
        let mut kmp = fixture();
        kmp.enph = Section::new(vec![
            PathGroup::new(0, 2, [255; 6], [255; 6], 0),
            PathGroup::new(0, 1, [255; 6], [255; 6], 0),
        ]);
        let mut app = headless_app();
        let bytes = load(&mut app, &path, &kmp);
        save(app.world_mut(), None).unwrap();
        assert_eq!(fs::read(&path).unwrap(), bytes);
        let enemy = ordered::<EnemyPathPoint>(app.world_mut())[0];
        app.world_mut().get_mut::<EnemyPathPoint>(enemy).unwrap().leniency = 25.;
        assert!(save(app.world_mut(), None)
            .unwrap_err()
            .to_string()
            .contains("Conflicting edits"));
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }

    /// Exercise message-driven saves, source-index mapping, repeated saves, and
    /// external-change protection without replaying already-consumed requests.
    #[test]
    fn headless_load_edit_save_preserves_layout_and_consumes_requests() {
        let dir = TestDir::new();
        let path = dir.0.join("course.kmp");
        let mut app = headless_app();
        let original = load(&mut app, &path, &fixture());
        app.world_mut().write_message(SaveFile(None));
        save_kmp(app.world_mut());
        assert!(app.world().resource::<SaveStatus>().0.starts_with("Saved"));
        assert_eq!(fs::read(&path).unwrap(), original);
        // Exercise the same synchronous normalization as the pending refresh.
        app.world_mut().run_system_once(refresh_order::<RoutePoint>).unwrap();
        let enemy = ordered::<EnemyPathPoint>(app.world_mut())[0];
        app.world_mut().get_mut::<EnemyPathPoint>(enemy).unwrap().leniency = 99.;
        let route = ordered::<RoutePoint>(app.world_mut())[0];
        app.world_mut().get_mut::<RoutePoint>(route).unwrap().settings = 42;
        app.world_mut().write_message(SaveFile(None));
        save_kmp(app.world_mut());
        assert!(
            app.world().resource::<SaveStatus>().0.starts_with("Saved"),
            "{}",
            app.world().resource::<SaveStatus>().0
        );
        let saved = fs::read(&path).unwrap();
        let parsed = KmpFile::read(&mut Cursor::new(&saved)).unwrap();
        assert_eq!(parsed.enpt[0].leniency, 99.);
        assert_eq!(parsed.enpt[1].leniency, 20.);
        assert_eq!(parsed.enph[0].start, 1);
        assert_eq!(parsed.enph[0].group_link, 123);
        assert!(parsed.poti[0].points.is_empty());
        assert_eq!(parsed.poti[1].points[0].setting_1, 42);
        assert_eq!(parsed.gobj[0].route, 1);
        assert_eq!(parsed.stgi[1].lap_count, 7);
        assert_eq!(parsed.ckpt[0].prev_cp, 17);
        assert_eq!(parsed.ckpt[0].next_cp, 23);
        assert_eq!(parsed.ckph[0].group_link, 789);
        // A second intentional save succeeds against last-saved bytes.
        app.world_mut().write_message(SaveFile(None));
        save_kmp(app.world_mut());
        assert!(app.world().resource::<SaveStatus>().0.starts_with("Saved"));
        // No request means no write, even if disk changes externally.
        fs::write(&path, b"external edit").unwrap();
        save_kmp(app.world_mut());
        assert_eq!(fs::read(&path).unwrap(), b"external edit");
        app.world_mut().write_message(SaveFile(None));
        save_kmp(app.world_mut());
        assert!(app.world().resource::<SaveStatus>().0.contains("changed on disk"));
        assert_eq!(fs::read(&path).unwrap(), b"external edit");
    }

    /// Refusal must protect disk bytes, and a failed Save As must not switch the
    /// active path. Cover both entity-count and overall-start topology changes.
    #[test]
    fn structural_changes_refuse_before_writing_and_failed_save_as_keeps_path() {
        let dir = TestDir::new();
        let path = dir.0.join("course.kmp");
        let mut app = headless_app();
        let bytes = load(&mut app, &path, &fixture());
        let added = app
            .world_mut()
            .spawn((StartPoint::default(), Transform::default(), OrderId(1)))
            .id();
        let error = save(app.world_mut(), None).unwrap_err();
        assert!(error.to_string().contains("Structural saving"));
        assert_eq!(fs::read(&path).unwrap(), bytes);
        app.world_mut().despawn(added);
        let missing = dir.0.join("missing").join("new.kmp");
        assert!(save(app.world_mut(), Some(missing)).is_err());
        assert_eq!(app.world().resource::<KmpFilePath>().0, path);
        let enemy = ordered::<EnemyPathPoint>(app.world_mut())[0];
        app.world_mut().entity_mut(enemy).insert(PathOverallStart);
        assert!(save(app.world_mut(), None)
            .unwrap_err()
            .to_string()
            .contains("topology"));
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }

    /// Reject an uneditable incoming document before destroying the valid open
    /// document or its lossless saving snapshot.
    #[test]
    fn missing_stgi_does_not_despawn_existing_document() {
        let dir = TestDir::new();
        let mut app = headless_app();
        let path = dir.0.join("course.kmp");
        load(&mut app, &path, &fixture());
        let before = ordered::<StartPoint>(app.world_mut());
        app.world_mut().resource_mut::<Messages<KmpFileSelected>>().clear();
        let invalid_path = dir.0.join("invalid.kmp");
        let mut cursor = Cursor::new(Vec::new());
        KmpFile::default().write(&mut cursor).unwrap();
        fs::write(&invalid_path, cursor.into_inner()).unwrap();
        app.world_mut().write_message(KmpFileSelected(invalid_path));
        assert!(open_kmp(app.world_mut()).unwrap_err().to_string().contains("STGI"));
        assert_eq!(ordered::<StartPoint>(app.world_mut()), before);
        assert_eq!(app.world().resource::<KmpFilePath>().0, path);
        assert!(app.world().contains_resource::<LoadedKmp>());
    }

    /// Unspawned points remain source data, while explicitly linking an unresolved
    /// respawn to point zero must count as an edit rather than an exporter default.
    #[test]
    fn snapshots_preserve_unrepresented_points_and_detect_link_to_respawn_zero() {
        let dir = TestDir::new();
        let mut app = headless_app();
        let mut kmp = fixture();
        kmp.enpt.entries.push(Enpt {
            leniency: 1234.,
            ..default()
        });
        kmp.ckpt[0].respawn_pos = 200;
        load(&mut app, &dir.0.join("course.kmp"), &kmp);
        let document = app.world().resource::<LoadedKmp>().clone();
        assert_eq!(document.baseline.enpt[2].leniency, 1234.);
        assert_eq!(document.baseline.ckpt[0].respawn_pos, 200);
        let checkpoint = ordered::<Checkpoint>(app.world_mut())[0];
        let respawn = ordered::<RespawnPoint>(app.world_mut())[0];
        app.world_mut()
            .entity_mut(checkpoint)
            .insert(CheckpointRespawnLink(respawn));
        let snapshot = document.snapshot(app.world_mut()).unwrap();
        assert_eq!(snapshot.ckpt[0].respawn_pos, 0);
        assert_eq!(snapshot.enpt[2].leniency, 1234.);
    }

    /// Successful replacement retains destination permissions and leaves no
    /// temporary sibling behind.
    #[test]
    fn atomic_replacement_preserves_permissions() {
        let dir = TestDir::new();
        let path = dir.0.join("course.kmp");
        fs::write(&path, b"original").unwrap();
        let permissions = fs::metadata(&path).unwrap().permissions();
        atomic_replace(&path, b"saved", Some(b"original"), &permissions).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"saved");
        assert_eq!(fs::metadata(&path).unwrap().permissions(), permissions);
        assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 1);
    }

    /// A directory target forces failure at rename, after temporary-file creation,
    /// proving cleanup does not remove or truncate the destination.
    #[test]
    fn failed_atomic_rename_cleans_temp_and_preserves_destination() {
        let dir = TestDir::new();
        let destination = dir.0.join("directory.kmp");
        fs::create_dir(&destination).unwrap();
        let permissions = fs::metadata(&destination).unwrap().permissions();
        assert!(atomic_replace(&destination, b"data", None, &permissions).is_err());
        assert!(destination.is_dir());
        assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 1);
    }
}
