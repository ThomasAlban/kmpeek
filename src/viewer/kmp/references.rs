//! Bind numeric UI references to document identities when a value is edited,
//! not when it is eventually saved. Unchanged imported values use source rows;
//! explicit edits use editor row order. Bindings are part of typed undo snapshots.
use super::{components::*, document::LoadedKmp, history::DocumentId, ordering::OrderId};
use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Clone, PartialEq)]
struct Binding {
    // Last observed UI number; only a changed number (or AREA kind) rebinds it.
    value: u8,
    // Logical DocumentId, not a live allocation or an index into the next save.
    // E.g. entering row 0 keeps targeting that point after it moves to row 2.
    target: Option<Entity>,
    // An explicitly invalid index is an error even for optional camera links.
    invalid_edit: bool,
}

#[derive(Component, Clone, Default, PartialEq)]
pub struct NumericReferences {
    camera: Option<Binding>,
    area: Option<(bool, Binding)>, // true: camera, false: enemy point
}

fn rows<T: Component>(world: &mut World) -> Vec<Entity> {
    let mut rows: Vec<_> = world
        .query_filtered::<(Entity, Option<&OrderId>), With<T>>()
        .iter(world)
        .map(|(e, order)| (order.map(|o| o.0), e))
        .collect();
    rows.sort_unstable();
    rows.into_iter().map(|(_, e)| e).collect()
}

pub fn refresh(world: &mut World) {
    let cameras = rows::<KmpCamera>(world);
    let enemies = rows::<EnemyPathPoint>(world);
    let Some(source) = world.get_resource::<LoadedKmp>().cloned() else {
        return;
    };
    let source = source.live(world);
    let mut updates = Vec::new();
    for entity in world.iter_entities() {
        let e = entity.id();
        let old = entity.get::<NumericReferences>();
        let mut refs = old.cloned().unwrap_or_default();
        let binding = |value: u8, section: &str, current: &[Entity], imported: bool| {
            let target = if value == 255 {
                None
            } else if imported {
                source.source_entity(section, usize::from(value))
            } else {
                current.get(usize::from(value)).copied()
            };
            Binding {
                value,
                target: target.map(|e| world.get::<DocumentId>(e).map_or(e, |id| id.0)),
                invalid_edit: !imported && value != 255 && target.is_none(),
            }
        };
        if let Some(camera) = entity.get::<KmpCamera>() {
            if refs.camera.as_ref().is_none_or(|b| b.value != camera.next_index) {
                let imported = old.is_none() && source.original_index("KmpCamera", e).is_some();
                refs.camera = Some(binding(camera.next_index, "KmpCamera", &cameras, imported));
            }
        }
        if let Some(area) = entity.get::<AreaPoint>() {
            let value = match area.kind {
                AreaKind::Camera { cam_index } => Some((true, cam_index)),
                AreaKind::ForceRecalc { enemy_path_id } => Some((false, enemy_path_id)),
                _ => None,
            };
            match value {
                Some((camera, value))
                    if refs
                        .area
                        .as_ref()
                        .is_none_or(|(kind, b)| *kind != camera || b.value != value) =>
                {
                    let imported = old.is_none() && source.original_index("AreaPoint", e).is_some();
                    refs.area = Some((
                        camera,
                        binding(
                            value,
                            if camera { "KmpCamera" } else { "EnemyPathPoint" },
                            if camera { &cameras } else { &enemies },
                            imported,
                        ),
                    ));
                }
                None => refs.area = None,
                _ => {}
            }
        }
        if old != Some(&refs) && (entity.contains::<KmpCamera>() || entity.contains::<AreaPoint>()) {
            updates.push((e, refs));
        }
    }
    for (e, refs) in updates {
        world.entity_mut(e).insert(refs);
    }
}

/// None means this is a data-only caller without bindings (the planner's unit
/// tests also exercise the original source-reference path directly).
pub fn resolve(
    world: &World,
    entity: Entity,
    area: bool,
    value: u8,
    indices: &HashMap<Entity, usize>,
    optional: bool,
) -> Option<anyhow::Result<u8>> {
    resolve_inner(world, entity, area, value, indices, optional, false)
}

/// Original-layout patches preserve unresolved imported values, but resolved
/// bindings must map back to source rows (including holes), not display indices.
pub fn resolve_patch(
    world: &World,
    entity: Entity,
    area: bool,
    value: u8,
    indices: &HashMap<Entity, usize>,
    camera: bool,
) -> Option<anyhow::Result<u8>> {
    resolve_inner(world, entity, area, value, indices, camera, true)
}

fn resolve_inner(
    world: &World,
    entity: Entity,
    area: bool,
    value: u8,
    indices: &HashMap<Entity, usize>,
    optional: bool,
    patch: bool,
) -> Option<anyhow::Result<u8>> {
    let refs = world.get::<NumericReferences>(entity)?;
    let binding = if area {
        refs.area
            .as_ref()
            .filter(|(camera, _)| *camera == optional)
            .map(|(_, b)| b)
    } else {
        refs.camera.as_ref()
    }?;
    if binding.value != value {
        return None;
    }
    Some((|| {
        anyhow::ensure!(
            !binding.invalid_edit,
            "Edited reference index does not exist; choose a valid row or 255"
        );
        if value == 255 || (patch && binding.target.is_none()) {
            // invalid_edit was checked above: only untouched source-only values
            // may stay unresolved in byte-preserving patch mode.
            return Ok(value);
        }
        let index = binding.target.and_then(|target| {
            indices
                .iter()
                .find_map(|(&e, &index)| (world.get::<DocumentId>(e).map_or(e, |id| id.0) == target).then_some(index))
        });
        match index {
            Some(index) => {
                anyhow::ensure!(index < 255, "Reference exceeds the 8-bit index range; relink it");
                Ok(index as u8)
            }
            None if optional && !patch => Ok(255),
            None => anyhow::bail!("Referenced point was deleted or is unavailable; relink before rebuilding"),
        }
    })())
}
