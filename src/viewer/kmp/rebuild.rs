//! Canonical, intentionally lossy reconstruction of the current editor document.
use super::{
    checkpoints::{CheckpointLeft, CheckpointRespawnLink, CheckpointRight},
    components::*,
    document::LoadedKmp,
    ordering::OrderId,
    path::KmpPathNode,
    routes::RouteLink,
    KmpSectionEntityIdMap,
};
use crate::util::kmp_file::*;
use anyhow::{ensure, Context, Result};
use bevy::{ecs::resource::IsResource, prelude::*};
use std::collections::{HashMap, HashSet};

// The plan contains only checked entity identities and final serialized indices.
#[derive(Debug)]
struct GraphPlan {
    groups: Vec<Vec<Entity>>,
    previous: Vec<Vec<usize>>,
    next: Vec<Vec<usize>>,
    indices: HashMap<Entity, usize>,
}

// Collect ordinary points without silently excluding malformed points lacking a transform.
fn ordered<T: Component>(world: &mut World) -> Result<Vec<Entity>> {
    let mut entries = Vec::new();
    for (e, order, transform) in world
        .query_filtered::<(Entity, Option<&OrderId>, Option<&Transform>), (With<T>, Without<IsResource>)>()
        .iter(world)
    {
        let order = order.context("A point is missing OrderId; repair its editor ordering before rebuilding")?;
        let transform = transform.context("A point is missing its transform")?;
        ensure!(
            transform.translation.is_finite() && transform.rotation.is_finite() && transform.scale.is_finite(),
            "Point {e:?} has a non-finite transform"
        );
        ensure!(
            transform.rotation.is_normalized(),
            "Point {e:?} has an invalid rotation quaternion"
        );
        entries.push((order.0, e));
    }
    ensure!(
        entries.len() <= usize::from(u16::MAX),
        "Too many points for a KMP section"
    );
    entries.sort_unstable();
    Ok(entries.into_iter().map(|(_, e)| e).collect())
}

// Validate reciprocal edges before traversal, so stale or cross-section edges cannot disappear.
fn graph(world: &World, entities: &[Entity], routes: bool) -> Result<GraphPlan> {
    let members: HashSet<_> = entities.iter().copied().collect();
    let mut nodes = HashMap::new();
    for &e in entities {
        let node = world
            .get::<KmpPathNode>(e)
            .context("A path point is missing its graph component")?;
        let max = if routes { 1 } else { 6 };
        ensure!(
            node.prev_nodes.len() <= max && node.next_nodes.len() <= max,
            "Path point {e:?} has too many links (maximum {max}); routes cannot branch"
        );
        for (&target, forward) in node
            .next_nodes
            .iter()
            .map(|e| (e, true))
            .chain(node.prev_nodes.iter().map(|e| (e, false)))
        {
            ensure!(
                members.contains(&target),
                "Stale or cross-section path edge at {e:?}; disconnect or relink it"
            );
            let other = world
                .get::<KmpPathNode>(target)
                .context("Linked point has no graph component")?;
            ensure!(
                if forward {
                    other.prev_nodes.contains(&e)
                } else {
                    other.next_nodes.contains(&e)
                },
                "Asymmetric path edge at {e:?}; disconnect and relink it"
            );
        }
        nodes.insert(e, node);
    }
    let starts: Vec<_> = entities
        .iter()
        .copied()
        .filter(|e| world.get::<PathOverallStart>(*e).is_some())
        .collect();
    ensure!(
        routes || starts.len() <= 1,
        "Multiple overall starts in one path section"
    );
    let mut seeds = entities.to_vec();
    if !routes {
        if let Some(start) = starts.first() {
            seeds.retain(|e| e != start);
            seeds.insert(0, *start);
        }
    }
    let mut visited = HashSet::new();
    let mut groups = Vec::new();
    for seed in seeds {
        if visited.contains(&seed) {
            continue;
        }
        let mut start = seed;
        let mut backwards = HashSet::from([seed]);
        // Walk back to a chain boundary. An unanchored cycle begins at its lowest ordered seed.
        while routes || world.get::<PathOverallStart>(start).is_none() {
            let node = nodes[&start];
            if node.prev_nodes.len() != 1 {
                break;
            }
            let Some(&previous) = node.prev_nodes.iter().next() else {
                break;
            };
            if nodes[&previous].next_nodes.len() != 1 || visited.contains(&previous) {
                break;
            }
            if !backwards.insert(previous) {
                ensure!(
                    !routes,
                    "Route cycle detected; disconnect the closing edge and use route loop settings"
                );
                start = seed;
                break;
            }
            start = previous;
        }
        let mut chain = Vec::new();
        let mut current = start;
        loop {
            if !visited.insert(current) {
                break;
            }
            chain.push(current);
            let node = nodes[&current];
            if node.next_nodes.len() != 1 {
                break;
            }
            let Some(&next) = node.next_nodes.iter().next() else {
                break;
            };
            if nodes[&next].prev_nodes.len() != 1
                || visited.contains(&next)
                || (!routes && world.get::<PathOverallStart>(next).is_some())
            {
                break;
            }
            current = next;
        }
        // Splitting a long chain creates ordinary group links, never truncated length bytes.
        let chunk_size = if routes { usize::from(u16::MAX) } else { 255 };
        for chunk in chain.chunks(chunk_size) {
            groups.push(chunk.to_vec());
        }
    }
    ensure!(
        visited.len() == entities.len(),
        "Path planning did not cover every point"
    );
    ensure!(groups.len() <= if routes { 65535 } else { 255 }, "Too many path groups");
    let mut group_of = HashMap::new();
    let mut indices = HashMap::new();
    let mut offset = 0;
    for (id, group) in groups.iter().enumerate() {
        ensure!(routes || offset <= 255, "Path group start exceeds the 8-bit range");
        for &e in group {
            group_of.insert(e, id);
            indices.insert(e, offset);
            offset += 1;
        }
    }
    let mut previous = vec![Vec::new(); groups.len()];
    let mut next = vec![Vec::new(); groups.len()];
    for (id, group) in groups.iter().enumerate() {
        if let Some(last) = group.last() {
            for target in &nodes[last].next_nodes {
                let target_id = group_of[target];
                ensure!(
                    groups[target_id].first() == Some(target),
                    "Path edge targets the middle of a serialized group"
                );
                next[id].push(target_id);
                previous[target_id].push(id);
            }
        }
    }
    for links in previous.iter_mut().chain(next.iter_mut()) {
        links.sort_unstable();
        links.dedup();
        ensure!(routes || links.len() <= 6, "Path group has more than six links");
    }
    Ok(GraphPlan {
        groups,
        previous,
        next,
        indices,
    })
}

// Assign dense IDs before any converter can look up a route or respawn reference.
fn index_map(entities: &[Entity]) -> HashMap<Entity, usize> {
    entities.iter().enumerate().map(|(i, &e)| (e, i)).collect()
}

// Resolve unchanged raw references through source identity, but edited numbers through editor order.
fn raw_target(
    value: u8,
    baseline: Option<u8>,
    original: Option<u8>,
    source: &LoadedKmp,
    target_section: &str,
    current: &[Entity],
    final_indices: &HashMap<Entity, usize>,
    optional: bool,
) -> Result<u8> {
    if value == 255 {
        return Ok(255);
    }
    // An explicitly entered index must resolve. Only an unchanged reference
    // whose old target was deleted may be cleared to an optional sentinel.
    ensure!(
        baseline == Some(value) || usize::from(value) < current.len(),
        "Edited {target_section} index {value} does not exist; choose a valid row or 255 for no target"
    );
    let target = if baseline == Some(value) {
        original.and_then(|index| source.source_entity(target_section, usize::from(index)))
    } else {
        current.get(usize::from(value)).copied()
    };
    let index = target.and_then(|e| final_indices.get(&e)).copied();
    match index {
        Some(index) => {
            ensure!(
                index < 255,
                "{target_section} reference exceeds the 8-bit index range; relink it"
            );
            Ok(u8::try_from(index)?)
        }
        None if optional => Ok(255),
        None => anyhow::bail!(
            "Referenced {target_section} point was deleted or is unavailable; relink the reference before rebuilding"
        ),
    }
}

// Convert a validated component without assuming a query can never fail.
fn convert<T: KmpComponent>(world: &mut World, e: Entity) -> Result<T::KmpFormat> {
    let component = world
        .get::<T>(e)
        .context("Planned point component disappeared")?
        .clone();
    let transform = *world.get::<Transform>(e).context("Planned transform disappeared")?;
    Ok(component.to_kmp(transform, world, e))
}

// Convert ordinary sections in their checked editor order.
fn points<T: KmpComponent>(world: &mut World, entities: &[Entity]) -> Result<Section<T::KmpFormat>> {
    Section::try_new(
        entities
            .iter()
            .map(|&entity| convert::<T>(world, entity))
            .collect::<Result<_>>()?,
    )
}

// Serialize graph groups and override CKPT adjacency using final group-local order.
fn paths<T: KmpComponent>(
    world: &mut World,
    plan: &GraphPlan,
) -> Result<(Section<T::KmpFormat>, Section<PathGroup<T::KmpFormat>>)>
where
    PathGroup<T::KmpFormat>: KmpSectionName,
{
    let mut records = Vec::new();
    let mut groups = Vec::new();
    for (id, entities) in plan.groups.iter().enumerate() {
        let start = records.len();
        for (offset, &e) in entities.iter().enumerate() {
            let mut record = convert::<T>(world, e)?;
            if let Some(cp) = (&mut record as &mut dyn std::any::Any).downcast_mut::<Ckpt>() {
                cp.prev_cp = if offset == 0 {
                    255
                } else {
                    u8::try_from(start + offset - 1)?
                };
                cp.next_cp = if offset + 1 == entities.len() {
                    255
                } else {
                    u8::try_from(start + offset + 1)?
                };
            }
            records.push(record);
        }
        let mut previous = [255; 6];
        let mut next = [255; 6];
        for (slot, &index) in previous.iter_mut().zip(&plan.previous[id]) {
            *slot = u8::try_from(index)?;
        }
        for (slot, &index) in next.iter_mut().zip(&plan.next[id]) {
            *slot = u8::try_from(index)?;
        }
        groups.push(PathGroup::new(
            u8::try_from(start)?,
            u8::try_from(entities.len())?,
            previous,
            next,
            0,
        ));
    }
    Ok((Section::try_new(records)?, Section::try_new(groups)?))
}

/// Raw KMP records contain no optional JSON fields. Serde represents NaN and
/// infinity as null, so this final check rejects non-finite scalar settings that
/// are not part of a Transform (camera zoom/time, scales, path sizes, and so on).
fn finite_scalars(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Null => false,
        serde_json::Value::Array(values) => values.iter().all(finite_scalars),
        serde_json::Value::Object(fields) => fields.values().all(finite_scalars),
        _ => true,
    }
}

/// Rebuild all supported sections from editor state, intentionally discarding unsupported source data.
pub fn rebuild(world: &mut World, source: &LoadedKmp) -> Result<KmpFile> {
    ensure!(!source.original.stgi.is_empty(), "Loaded KMP has no STGI record");
    let track = world
        .get_resource::<TrackInfo>()
        .context("Track information is missing")?
        .clone();
    ensure!(
        track.speed_mod.is_finite() && track.speed_mod >= 0.,
        "Speed modifier must be finite and nonnegative (0 means normal speed)"
    );
    let ktpt = ordered::<StartPoint>(world)?;
    let enpt = ordered::<EnemyPathPoint>(world)?;
    let itpt = ordered::<ItemPathPoint>(world)?;
    let ckpt = ordered::<Checkpoint>(world)?;
    let gobj = ordered::<Object>(world)?;
    let route_points = ordered::<RoutePoint>(world)?;
    let area = ordered::<AreaPoint>(world)?;
    let came = ordered::<KmpCamera>(world)?;
    let jgpt = ordered::<RespawnPoint>(world)?;
    let cnpt = ordered::<CannonPoint>(world)?;
    let mspt = ordered::<BattleFinishPoint>(world)?;
    ensure!(
        enpt.len() <= 255 && itpt.len() <= 255,
        "ENPT and ITPT support at most 255 points"
    );
    let enemies = graph(world, &enpt, false)?;
    let items = graph(world, &itpt, false)?;
    let checkpoints = graph(world, &ckpt, false)?;
    // Index 255 is an absent neighbor, but a singleton at that group start is representable.
    for group in &checkpoints.groups {
        for pair in group.windows(2) {
            ensure!(
                checkpoints.indices[&pair[0]] < 255 && checkpoints.indices[&pair[1]] < 255,
                "CKPT adjacency exceeds the 8-bit index range; split or shorten the checkpoint path"
            );
        }
    }
    let routes = graph(world, &route_points, true)?;
    let respawns = index_map(&jgpt);
    let cameras = index_map(&came);
    let mut route_ids = HashMap::new();
    let mut route_settings = Vec::new();
    for (id, group) in routes.groups.iter().enumerate() {
        for &e in group {
            route_ids.insert(e, id);
        }
        // Prefer the actual chain start, then the first surviving member with settings.
        // Newly created chains without any settings use the editor's cyclic/non-smooth default.
        route_settings.push(
            group
                .iter()
                .find_map(|e| world.get::<RouteSettings>(*e))
                .cloned()
                .unwrap_or_default(),
        );
    }
    for &e in gobj.iter().chain(&area).chain(&came) {
        if let Some(link) = world.get::<RouteLink>(e) {
            let id = *route_ids
                .get(&link.0)
                .context("Route target was deleted; remove or relink the route reference")?;
            let limit = if world.get::<Object>(e).is_some() { 65535 } else { 255 };
            ensure!(
                id < limit,
                "Route reference exceeds this section's index width; relink it"
            );
        }
    }
    for &e in &ckpt {
        let pair = world
            .get::<CheckpointLeft>(e)
            .context("Checkpoint has no right-side pair")?;
        ensure!(
            pair.right != e && world.get::<CheckpointRight>(pair.right).is_some_and(|r| r.left == e),
            "Checkpoint pair is not reciprocal"
        );
        let right = world
            .get::<Transform>(pair.right)
            .context("Checkpoint right side has no transform")?;
        ensure!(
            right.translation.is_finite() && right.rotation.is_finite() && right.scale.is_finite(),
            "Checkpoint right transform is non-finite"
        );
        let link = world
            .get::<CheckpointRespawnLink>(e)
            .context("Checkpoint has no respawn target; relink it before rebuilding")?;
        let id = respawns
            .get(&link.0)
            .context("Checkpoint respawn target was deleted; relink it before rebuilding")?;
        ensure!(*id <= 255, "Checkpoint respawn reference exceeds 8 bits; relink it");
        if let Some(Checkpoint {
            kind: CheckpointKind::Key(id),
        }) = world.get::<Checkpoint>(e)
        {
            ensure!((1..=127).contains(id), "Checkpoint key ID must be between 1 and 127");
        }
    }
    for (e, right) in world.query::<(Entity, &CheckpointRight)>().iter(world) {
        ensure!(
            checkpoints.indices.contains_key(&right.left)
                && world.get::<CheckpointLeft>(right.left).is_some_and(|p| p.right == e),
            "Orphaned checkpoint right side; repair the pair before rebuilding"
        );
    }
    let intros: Vec<_> = world
        .query_filtered::<Entity, With<KmpCameraIntroStart>>()
        .iter(world)
        .collect();
    ensure!(intros.len() <= 1, "Multiple intro camera starts cannot be rebuilt");
    let intro = match intros.first() {
        Some(e) => {
            let id = *cameras.get(e).context("Intro marker is not on a camera")?;
            ensure!(id < 255, "Intro camera index exceeds 8 bits");
            u16::try_from(id)?
        }
        None => 255,
    };
    let mut camera_next = Vec::new();
    for &e in &came {
        let camera = world.get::<KmpCamera>(e).context("Missing camera")?;
        let index = source.original_index("KmpCamera", e);
        camera_next.push(raw_target(
            camera.next_index,
            index.and_then(|i| source.baseline.came.get(i)).map(|c| c.next_index),
            index.and_then(|i| source.original.came.get(i)).map(|c| c.next_index),
            source,
            "KmpCamera",
            &came,
            &cameras,
            true,
        )?);
    }
    let mut area_targets = Vec::new();
    for &e in &area {
        let point = world.get::<AreaPoint>(e).context("Missing area")?;
        let index = source.original_index("AreaPoint", e);
        let baseline = index.and_then(|i| source.baseline.area.get(i));
        let original = index.and_then(|i| source.original.area.get(i));
        let target = match point.kind {
            AreaKind::Camera { cam_index } => Some(raw_target(
                cam_index,
                baseline.filter(|a| a.kind == 0).map(|a| a.came_index),
                original.map(|a| a.came_index),
                source,
                "KmpCamera",
                &came,
                &cameras,
                true,
            )?),
            AreaKind::ForceRecalc { enemy_path_id } => Some(raw_target(
                enemy_path_id,
                baseline.filter(|a| a.kind == 4).map(|a| a.enpt_id),
                original.map(|a| a.enpt_id),
                source,
                "EnemyPathPoint",
                &enpt,
                &enemies.indices,
                false,
            )?),
            _ => None,
        };
        area_targets.push(target);
    }
    // All user-data validation finishes before converters can read these temporary lookup resources.
    let old_routes = world.remove_resource::<KmpSectionEntityIdMap<RouteSettings>>();
    let old_respawns = world.remove_resource::<KmpSectionEntityIdMap<RespawnPoint>>();
    world.insert_resource(KmpSectionEntityIdMap::<RouteSettings>::new(
        route_ids.iter().map(|(&e, &i)| (e, i as u16)).collect(),
    ));
    world.insert_resource(KmpSectionEntityIdMap::<RespawnPoint>::new(
        respawns.iter().map(|(&e, &i)| (e, i as u16)).collect(),
    ));
    let result = (|| -> Result<KmpFile> {
        let mut file = KmpFile {
            ktpt: points::<StartPoint>(world, &ktpt)?,
            ..Default::default()
        };
        (file.enpt, file.enph) = paths::<EnemyPathPoint>(world, &enemies)?;
        (file.itpt, file.itph) = paths::<ItemPathPoint>(world, &items)?;
        (file.ckpt, file.ckph) = paths::<Checkpoint>(world, &checkpoints)?;
        file.gobj = points::<Object>(world, &gobj)?;
        let mut poti = Vec::new();
        for (group, settings) in routes.groups.iter().zip(&route_settings) {
            let records = group
                .iter()
                .map(|&e| convert::<RoutePoint>(world, e))
                .collect::<Result<Vec<_>>>()?;
            poti.push(Poti {
                num_points: u16::try_from(records.len())?,
                setting_1: u8::from(settings.smooth_motion),
                setting_2: match settings.loop_style {
                    RouteLoopStyle::Cyclic => 0,
                    RouteLoopStyle::Mirror => 1,
                },
                points: records,
            });
        }
        file.poti = Section::try_new(poti)?;
        file.poti.section_header.additional_value = u16::try_from(route_points.len())?;
        file.area = points::<AreaPoint>(world, &area)?;
        for (record, target) in file.area.iter_mut().zip(area_targets) {
            if let Some(target) = target {
                if record.kind == 0 {
                    record.came_index = target;
                } else {
                    record.enpt_id = target;
                }
            }
        }
        file.came = points::<KmpCamera>(world, &came)?;
        for (record, next) in file.came.iter_mut().zip(camera_next) {
            record.next_index = next;
        }
        // CAME's low byte selects a camera for pre-rendered track-selection
        // videos (ignored by Wii runtime). Preserve its target when it resolves;
        // do not mistake it for the replay-camera links stored in AREA records.
        let old_selection = (source.original.came.section_header.additional_value & 0xff) as u8;
        let selection = match source.source_entity("KmpCamera", usize::from(old_selection)) {
            Some(entity) => cameras
                .get(&entity)
                .copied()
                .filter(|&id| id < 255)
                .map(|id| id as u8)
                .unwrap_or(255),
            None => old_selection,
        };
        file.came.section_header.additional_value = (intro << 8) | u16::from(selection);
        file.jgpt = points::<RespawnPoint>(world, &jgpt)?;
        for (index, record) in file.jgpt.iter_mut().enumerate() {
            record.respawn_id = u16::try_from(index)?;
        }
        // Canonical local IDs follow final section order, as on Nintendo tracks.
        // KCL cannon triggers reference CNPT array positions, NOT these IDs;
        // users must update external triggers when cannon order changes.
        file.cnpt = points::<CannonPoint>(world, &cnpt)?;
        file.mspt = points::<BattleFinishPoint>(world, &mspt)?;
        for (index, record) in file.cnpt.iter_mut().enumerate() {
            record.id = u16::try_from(index)?;
        }
        for (index, record) in file.mspt.iter_mut().enumerate() {
            record.id = u16::try_from(index)?;
        }
        file.stgi = Section::try_new(vec![track.to_kmp(Transform::IDENTITY, world, Entity::PLACEHOLDER)])?;
        ensure!(
            finite_scalars(&serde_json::to_value(&file)?),
            "A KMP field contains NaN or infinity; correct it before rebuilding"
        );
        Ok(file)
    })();
    // Do not leave export-only indices installed in the live editor, even when conversion fails.
    world.remove_resource::<KmpSectionEntityIdMap<RouteSettings>>();
    world.remove_resource::<KmpSectionEntityIdMap<RespawnPoint>>();
    if let Some(previous) = old_routes {
        world.insert_resource(previous);
    }
    if let Some(previous) = old_respawns {
        world.insert_resource(previous);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::super::KmpSectionIdEntityMap;
    use super::*;
    use std::{
        io::Cursor,
        sync::atomic::{AtomicU64, Ordering},
    };

    // Build the smallest document snapshot without rendering plugins or course fixtures.
    fn empty_source(world: &mut World) -> LoadedKmp {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        world.init_resource::<TrackInfo>();
        world.init_resource::<KmpSectionIdEntityMap<RoutePoint>>();
        world.init_resource::<KmpSectionIdEntityMap<RespawnPoint>>();
        let mut original = KmpFile {
            stgi: Section::new(vec![Stgi::default()]),
            ..Default::default()
        };
        let cameras = ordered::<KmpCamera>(world).unwrap();
        original.came = points::<KmpCamera>(world, &cameras).unwrap();
        let path = std::env::temp_dir().join(format!(
            "kmpeek-rebuild-{}-{}.kmp",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let mut bytes = Cursor::new(Vec::new());
        original.clone().write(&mut bytes).unwrap();
        let bytes = bytes.into_inner();
        std::fs::write(&path, &bytes).unwrap();
        let source = LoadedKmp::capture(world, path.clone(), bytes, original).unwrap();
        std::fs::remove_file(path).unwrap();
        source
    }

    // Spawn only the data components needed by the rebuild planner.
    fn node<T: Component + Default>(world: &mut World, order: u32) -> Entity {
        world
            .spawn((
                T::default(),
                Transform::from_xyz(order as f32, 0., 0.),
                OrderId(order),
                KmpPathNode::default(),
            ))
            .id()
    }

    // Import-style links intentionally permit cycles and self-edges for graph regression coverage.
    fn link(world: &mut World, from: Entity, to: Entity) {
        world.get_mut::<KmpPathNode>(from).unwrap().next_nodes.insert(to);
        world.get_mut::<KmpPathNode>(to).unwrap().prev_nodes.insert(from);
    }

    /// Rebuild alone renumbers local IDs; opaque mission data is still retained.
    #[test]
    fn canonical_point_ids_ignore_sparse_editor_order_and_raw_ids() {
        let mut world = World::new();
        let source = empty_source(&mut world);
        world.spawn((
            CannonPoint { id: 1234, ..default() },
            Transform::IDENTITY,
            OrderId(70000),
        ));
        world.spawn((
            BattleFinishPoint {
                id: 4321,
                unknown: 0xabcd,
            },
            Transform::IDENTITY,
            OrderId(90000),
        ));
        world.spawn((
            RespawnPoint {
                respawn_id: 2222,
                extra_data: -123,
            },
            Transform::IDENTITY,
            OrderId(80000),
        ));
        let file = rebuild(&mut world, &source).unwrap();
        assert_eq!(file.cnpt[0].id, 0);
        assert_eq!(file.mspt[0].id, 0);
        assert_eq!(file.mspt[0].unknown, 0xabcd);
        assert_eq!(file.jgpt[0].respawn_id, 0);
        assert_eq!(file.jgpt[0].extra_data, -123);
    }

    #[test]
    fn branches_cycles_singletons_are_deterministic_and_covered_once() {
        let mut world = World::new();
        let a = node::<EnemyPathPoint>(&mut world, 40);
        let b = node::<EnemyPathPoint>(&mut world, 10);
        let c = node::<EnemyPathPoint>(&mut world, 30);
        let d = node::<EnemyPathPoint>(&mut world, 20);
        let lone = node::<EnemyPathPoint>(&mut world, 50);
        world.entity_mut(a).insert(PathOverallStart);
        for (from, to) in [(a, b), (b, c), (b, d), (c, a), (d, a), (lone, lone)] {
            link(&mut world, from, to);
        }
        let ordered = ordered::<EnemyPathPoint>(&mut world).unwrap();
        let first = graph(&world, &ordered, false).unwrap();
        let second = graph(&world, &ordered, false).unwrap();
        assert_eq!(first.groups, second.groups);
        assert_eq!(first.groups[0], vec![a, b]);
        assert_eq!(first.indices.len(), 5);
        assert_eq!(first.groups.iter().flatten().collect::<HashSet<_>>().len(), 5);
        assert_eq!(first.next[0], vec![1, 2]);
        assert_eq!(first.next[3], vec![3]);
    }

    #[test]
    fn every_three_node_topology_preserves_exact_edges_and_membership() {
        // Exhaust all directed graphs, including self-links, with and without an anchored start.
        for anchored in [false, true] {
            for bits in 0..512 {
                let mut world = World::new();
                let entities: Vec<_> = (0..3).map(|i| node::<EnemyPathPoint>(&mut world, i)).collect();
                if anchored {
                    world.entity_mut(entities[1]).insert(PathOverallStart);
                }
                let mut expected = HashSet::new();
                for from in 0..3 {
                    for to in 0..3 {
                        if bits & (1 << (from * 3 + to)) != 0 {
                            link(&mut world, entities[from], entities[to]);
                            expected.insert((entities[from], entities[to]));
                        }
                    }
                }
                let plan = graph(&world, &entities, false).unwrap();
                assert_eq!(plan.groups.iter().map(Vec::len).sum::<usize>(), 3);
                assert_eq!(plan.indices.len(), 3);
                let mut actual = HashSet::new();
                for (id, group) in plan.groups.iter().enumerate() {
                    for pair in group.windows(2) {
                        actual.insert((pair[0], pair[1]));
                    }
                    for &next in &plan.next[id] {
                        actual.insert((*group.last().unwrap(), plan.groups[next][0]));
                    }
                }
                assert_eq!(actual, expected, "graph {bits}, anchored {anchored}");
            }
        }
    }

    #[test]
    fn routes_find_actual_start_keep_member_settings_and_nonstart_links() {
        let mut world = World::new();
        let source = empty_source(&mut world);
        let tail = node::<RoutePoint>(&mut world, 0);
        let head = node::<RoutePoint>(&mut world, 9);
        world.entity_mut(tail).insert(RouteSettings {
            smooth_motion: true,
            loop_style: RouteLoopStyle::Mirror,
        });
        link(&mut world, head, tail);
        world.spawn((
            Object {
                settings: [91; 8],
                ..default()
            },
            Transform::IDENTITY,
            OrderId(0),
            RouteLink(tail),
        ));
        let rebuilt = rebuild(&mut world, &source).unwrap();
        assert_eq!(rebuilt.poti.len(), 1);
        assert_eq!(
            rebuilt.poti[0].points.iter().map(|p| p.position[0]).collect::<Vec<_>>(),
            vec![9., 0.]
        );
        assert_eq!((rebuilt.poti[0].setting_1, rebuilt.poti[0].setting_2), (1, 1));
        assert_eq!(rebuilt.gobj[0].route, 0);
        assert_eq!(rebuilt.gobj[0].settings, [91; 8]);
        link(&mut world, tail, head);
        assert!(rebuild(&mut world, &source).unwrap_err().to_string().contains("cycle"));
    }

    #[test]
    fn stale_asymmetric_cross_section_and_excess_links_are_errors() {
        let mut world = World::new();
        let a = node::<EnemyPathPoint>(&mut world, 0);
        let b = node::<EnemyPathPoint>(&mut world, 1);
        world.get_mut::<KmpPathNode>(a).unwrap().next_nodes.insert(b);
        assert!(graph(&world, &[a, b], false)
            .unwrap_err()
            .to_string()
            .contains("Asymmetric"));
        world.get_mut::<KmpPathNode>(b).unwrap().prev_nodes.insert(a);
        assert!(graph(&world, &[a], false)
            .unwrap_err()
            .to_string()
            .contains("cross-section"));
        let mut entities = vec![a, b];
        for i in 2..8 {
            let e = node::<EnemyPathPoint>(&mut world, i);
            link(&mut world, a, e);
            entities.push(e);
        }
        assert!(graph(&world, &entities, false)
            .unwrap_err()
            .to_string()
            .contains("too many"));
        world.despawn(b);
        assert!(graph(&world, &[a], false).is_err());
    }

    #[test]
    fn width_limits_and_long_chain_splitting_are_checked() {
        let mut world = World::new();
        let source = empty_source(&mut world);
        let entities: Vec<_> = (0..256).map(|i| node::<EnemyPathPoint>(&mut world, i)).collect();
        for pair in entities.windows(2) {
            link(&mut world, pair[0], pair[1]);
        }
        let plan = graph(&world, &entities, false).unwrap();
        assert_eq!(plan.groups.iter().map(Vec::len).collect::<Vec<_>>(), vec![255, 1]);
        assert_eq!(plan.next[0], vec![1]);
        assert!(rebuild(&mut world, &source)
            .unwrap_err()
            .to_string()
            .contains("at most 255"));
        let extra = node::<EnemyPathPoint>(&mut world, 256);
        let mut isolated = entities.clone();
        isolated.push(extra);
        // A disconnected group after a 256-point chain cannot encode its start.
        assert!(graph(&world, &isolated, false)
            .unwrap_err()
            .to_string()
            .contains("start exceeds"));
    }

    #[test]
    fn structural_rebuild_remaps_enemy_camera_and_respawn_indices_and_roundtrips() {
        let mut world = World::new();
        let source = empty_source(&mut world);
        let a = node::<EnemyPathPoint>(&mut world, 20);
        let b = node::<EnemyPathPoint>(&mut world, 0);
        link(&mut world, a, b);
        world.entity_mut(a).insert(PathOverallStart);
        world.spawn((
            AreaPoint {
                kind: AreaKind::ForceRecalc { enemy_path_id: 0 },
                ..default()
            },
            Transform::IDENTITY,
            OrderId(0),
        ));
        world.spawn((
            KmpCamera {
                next_index: 1,
                ..default()
            },
            Transform::IDENTITY,
            OrderId(2),
            KmpCameraIntroStart,
        ));
        world.spawn((
            KmpCamera {
                next_index: 255,
                ..default()
            },
            Transform::IDENTITY,
            OrderId(8),
        ));
        let respawn = world
            .spawn((RespawnPoint::default(), Transform::IDENTITY, OrderId(900)))
            .id();
        let cp1 = node::<Checkpoint>(&mut world, 90);
        let cp2 = node::<Checkpoint>(&mut world, 10);
        for left in [cp1, cp2] {
            let right = world
                .spawn((Transform::IDENTITY, CheckpointRight { left, ..default() }))
                .id();
            world
                .entity_mut(left)
                .insert((CheckpointLeft { right, ..default() }, CheckpointRespawnLink(respawn)));
        }
        world.entity_mut(cp1).insert(PathOverallStart);
        link(&mut world, cp1, cp2);
        let rebuilt = rebuild(&mut world, &source).unwrap();
        assert_eq!(rebuilt.area[0].enpt_id, 1);
        assert_eq!(rebuilt.came[0].next_index, 1);
        // Unresolved selection-video metadata is retained, not called a replay sentinel.
        assert_eq!(rebuilt.came.section_header.additional_value, 0x0000);
        assert_eq!(rebuilt.jgpt[0].respawn_id, 0);
        assert_eq!((rebuilt.ckpt[0].prev_cp, rebuilt.ckpt[0].next_cp), (255, 1));
        assert_eq!((rebuilt.ckpt[1].prev_cp, rebuilt.ckpt[1].next_cp), (0, 255));
        assert_eq!(rebuilt.ckpt[1].respawn_pos, 0);
        let mut encoded = Cursor::new(Vec::new());
        rebuilt.write(&mut encoded).unwrap();
        let parsed = KmpFile::read(&mut Cursor::new(encoded.into_inner())).unwrap();
        assert_eq!(parsed.area[0].enpt_id, 1);
        assert_eq!(parsed.enpt.len(), 2);
        world.despawn(respawn);
        assert!(rebuild(&mut world, &source).unwrap_err().to_string().contains("relink"));
    }

    #[test]
    fn raw_reference_identity_differs_from_edited_numeric_order() {
        let mut world = World::new();
        let a = world
            .spawn((
                KmpCamera {
                    next_index: 1,
                    ..default()
                },
                Transform::IDENTITY,
                OrderId(0),
            ))
            .id();
        let b = world
            .spawn((
                KmpCamera {
                    next_index: 255,
                    ..default()
                },
                Transform::IDENTITY,
                OrderId(1),
            ))
            .id();
        // Supply matching raw records so capture can remember their source identities.
        let mut source = empty_source(&mut world);
        source.original.came = Section::new(vec![
            Came {
                next_index: 1,
                ..default()
            },
            Came {
                next_index: 255,
                ..default()
            },
        ]);
        world.entity_mut(a).insert(OrderId(10));
        world.entity_mut(b).insert(OrderId(0));
        assert_eq!(rebuild(&mut world, &source).unwrap().came[1].next_index, 0);
        let current = vec![b, a];
        let indices = index_map(&current);
        assert_eq!(
            raw_target(1, Some(1), Some(1), &source, "KmpCamera", &current, &indices, true).unwrap(),
            0
        );
        assert_eq!(
            raw_target(0, Some(1), Some(1), &source, "KmpCamera", &current, &indices, true).unwrap(),
            0
        );
        world.despawn(b);
        assert_eq!(rebuild(&mut world, &source).unwrap().came[0].next_index, 255);
        let deleted = index_map(&[a]);
        assert_eq!(
            raw_target(1, Some(1), Some(1), &source, "KmpCamera", &[a], &deleted, true).unwrap(),
            255
        );
        assert!(
            raw_target(1, Some(1), Some(1), &source, "KmpCamera", &[a], &deleted, false)
                .unwrap_err()
                .to_string()
                .contains("relink")
        );
        assert_eq!(
            raw_target(255, None, None, &source, "KmpCamera", &[], &HashMap::new(), false).unwrap(),
            255
        );
    }

    #[test]
    fn route_indices_are_u16_for_objects_but_checked_u8_for_cameras() {
        let mut world = World::new();
        let source = empty_source(&mut world);
        let routes: Vec<_> = (0..257).map(|i| node::<RoutePoint>(&mut world, i)).collect();
        world.spawn((
            Object::default(),
            Transform::IDENTITY,
            OrderId(0),
            RouteLink(routes[256]),
        ));
        let rebuilt = rebuild(&mut world, &source).unwrap();
        assert_eq!(rebuilt.gobj[0].route, 256);
        assert_eq!(rebuilt.poti.len(), 257);
        let previous = world.resource::<KmpSectionEntityIdMap<RouteSettings>>().clone();
        world.spawn((
            KmpCamera {
                next_index: 255,
                ..default()
            },
            Transform::IDENTITY,
            OrderId(0),
            RouteLink(routes[256]),
        ));
        assert!(rebuild(&mut world, &source)
            .unwrap_err()
            .to_string()
            .contains("index width"));
        assert_eq!(world.resource::<KmpSectionEntityIdMap<RouteSettings>>().0, previous.0);
    }

    #[test]
    fn route_split_merge_and_delete_rebuild_current_members_only() {
        let mut world = World::new();
        let source = empty_source(&mut world);
        let a = node::<RoutePoint>(&mut world, 2);
        let b = node::<RoutePoint>(&mut world, 1);
        let c = node::<RoutePoint>(&mut world, 0);
        link(&mut world, a, b);
        link(&mut world, b, c);
        assert_eq!(rebuild(&mut world, &source).unwrap().poti[0].num_points, 3);
        world.get_mut::<KmpPathNode>(a).unwrap().next_nodes.clear();
        world.get_mut::<KmpPathNode>(b).unwrap().prev_nodes.clear();
        assert_eq!(rebuild(&mut world, &source).unwrap().poti.len(), 2);
        link(&mut world, a, b);
        assert_eq!(rebuild(&mut world, &source).unwrap().poti.len(), 1);
        world.get_mut::<KmpPathNode>(b).unwrap().next_nodes.clear();
        world.despawn(c);
        let rebuilt = rebuild(&mut world, &source).unwrap();
        assert_eq!(rebuilt.poti[0].num_points, 2);
        assert_eq!(rebuilt.poti.section_header.additional_value, 2);
        let branch = node::<RoutePoint>(&mut world, 10);
        link(&mut world, a, branch);
        assert!(rebuild(&mut world, &source)
            .unwrap_err()
            .to_string()
            .contains("cannot branch"));
    }

    #[test]
    fn malformed_points_pairs_missing_track_and_stale_routes_fail_without_panicking() {
        let mut world = World::new();
        let source = empty_source(&mut world);
        let malformed = world.spawn(StartPoint::default()).id();
        assert!(rebuild(&mut world, &source).is_err());
        world.despawn(malformed);
        let cp = node::<Checkpoint>(&mut world, 0);
        assert!(rebuild(&mut world, &source).unwrap_err().to_string().contains("pair"));
        world.despawn(cp);
        let object = world
            .spawn((
                Object::default(),
                Transform::IDENTITY,
                OrderId(0),
                RouteLink(Entity::PLACEHOLDER),
            ))
            .id();
        assert!(rebuild(&mut world, &source).unwrap_err().to_string().contains("relink"));
        world.despawn(object);
        world.remove_resource::<TrackInfo>();
        assert!(rebuild(&mut world, &source)
            .unwrap_err()
            .to_string()
            .contains("Track information"));
    }
}
