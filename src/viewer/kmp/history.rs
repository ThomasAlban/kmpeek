//! Typed, in-memory document history, independent of the KMP encoder.
//!
//! Reading path: `record_frame` captures document state each `Last` frame;
//! `observe` groups those samples using focus/pointer heuristics, not explicit
//! per-tool commands. `current` stays at the committed state while `pending`
//! follows an active edit. Committing pushes the old current onto `undo`.
//! Save and history navigation force a checkpoint, including this frame's edits.
//!
//! Snapshots contain document data only, including invalid intermediate states;
//! bitwise float comparison avoids inventing edits for unchanged NaNs. Selection
//! and display flags live in a separate view cache, and disk baselines/path live
//! in `document::LoadedKmp`, so neither is rewound as document content.
//!
//! Restore tears down rendered points, allocates all replacements, then installs
//! payloads and links in observer-safe phases before rebuilding derived caches.
//! Entity values in snapshots are logical keys: resolve them through the live
//! identity map rather than dereferencing them as current ECS allocations.
use super::{
    checkpoints::{checkpoint_spawner, CheckpointLeft, CheckpointRespawnLink, CheckpointRight},
    components::*,
    ordering::OrderId,
    path::{KmpPathNode, RecalcPaths},
    routes::{RouteLink, RouteLinkedEntities},
};
use crate::viewer::edit::{select::Selected, transform_gizmo::TransformGizmoState};
use bevy::{ecs::entity::EntityHashMap, prelude::*};

/// Stable document token, initially borrowed from the point's first allocation.
/// After undo, DocumentId(A) can live on entity B: A still names the same point,
/// but only B may be used to access its components. This is not an output row ID.
#[derive(Component, Clone, Copy)]
pub struct DocumentId(pub Entity);

/// Incremented on restoration so Local interaction state can discard stale handles.
#[derive(Resource, Default)]
pub struct RestoreGeneration(pub u64);

/// Suppress route spawn/link side effects while typed state is installed.
#[derive(Resource)]
pub(crate) struct RestoringDocument;

/// Keep live allocations and deleted allocations still referenced by live
/// document links. Snapshots/provenance already use logical keys and need no
/// historical allocation aliases. Rebuild this map at capture/restore boundaries.
#[derive(Resource, Default)]
struct DocumentAliases(EntityHashMap<Entity>);

fn reference_key(world: &World, e: Entity) -> Entity {
    world
        .get::<DocumentId>(e)
        .map(|id| id.0)
        .or_else(|| {
            world
                .get_resource::<DocumentAliases>()
                .and_then(|a| a.0.get(&e).copied())
        })
        .unwrap_or(e)
}

#[derive(Clone, PartialEq)]
enum Payload {
    Start(StartPoint),
    Enemy(EnemyPathPoint),
    Item(ItemPathPoint),
    Checkpoint(Checkpoint),
    Right,
    Object(Object),
    Route(RoutePoint),
    Area(AreaPoint),
    Camera(KmpCamera),
    Respawn(RespawnPoint),
    Cannon(CannonPoint),
    Finish(BattleFinishPoint),
}

#[derive(Clone, PartialEq)]
struct PointSnapshot {
    id: Entity,
    payload: Payload,
    transform: Option<Transform>,
    rotation: Option<KmpEulerRotation>,
    order: Option<u32>,
    node: Option<KmpPathNode>,
    pair: Option<Entity>,
    route: Option<Entity>,
    respawn: Option<Entity>,
    settings: Option<RouteSettings>,
    references: Option<super::references::NumericReferences>,
    overall_start: bool,
    intro_start: bool,
}

#[derive(Clone, PartialEq)]
pub struct DocumentSnapshot {
    points: Vec<PointSnapshot>,
    track: Option<TrackInfo>,
}

#[derive(Clone, Copy)]
struct PointView {
    selected: bool,
    visibility: Visibility,
    show_area: bool,
}

/// View caches are deliberately outside snapshots. Changing selection or hiding
/// a point does not create an edit; resurrected points recover their last view.
#[derive(Resource, Default)]
pub struct DocumentHistory {
    // Last committed state; intentionally not the latest frame during a gesture.
    current: Option<DocumentSnapshot>,
    // Latest observed state, replaced each frame until the group commits.
    pending: Option<DocumentSnapshot>,
    group: Option<EditGroup>,
    // Latch real edits for this text session, including after native undo has
    // returned to its baseline: native redo still belongs to that field.
    text_edited: bool,
    // Each stack's tail is the next destination, not a command to replay.
    undo: Vec<DocumentSnapshot>,
    redo: Vec<DocumentSnapshot>,
    views: EntityHashMap<PointView>,
    // UI queues navigation for Last, after this frame's editor mutations.
    // false = undo, true = redo.
    pub request: Option<bool>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EditGroup {
    Text(bevy_egui::egui::Id),
    Pointer,
}

/// Small input adapter shared by egui and headless grouping tests.
/// Focused text takes precedence over a held pointer. `finish` means Enter or
/// Tab (in either direction); release/focus loss is otherwise inferred.
/// This samples input, so unrelated mutations during a gesture also coalesce.
#[derive(Default)]
struct GroupInput {
    text: Option<bevy_egui::egui::Id>,
    pointer: bool,
    finish: bool,
}

const CAPACITY: usize = 100;

#[cfg(test)]
#[path = "history_tests.rs"]
mod tests;

pub fn plugin(app: &mut App) {
    app.init_resource::<DocumentHistory>()
        .init_resource::<RestoreGeneration>()
        .add_systems(Last, record_frame.before(super::save_kmp));
}

pub fn identities(world: &mut World) -> EntityHashMap<Entity> {
    world
        .query::<(Entity, &DocumentId)>()
        .iter(world)
        .map(|(e, id)| (id.0, e))
        .collect()
}

fn assign_identities(world: &mut World) {
    let entities: Vec<_> = world
        .query_filtered::<Entity, (With<KmpSelectablePoint>, Without<DocumentId>)>()
        .iter(world)
        .collect();
    for e in entities {
        world.entity_mut(e).insert(DocumentId(e));
    }
    let mut allocations = EntityHashMap::default();
    let mut targets = bevy::ecs::entity::EntityHashSet::default();
    for (e, id, node, left, right, route, respawn) in world
        .query::<(
            Entity,
            &DocumentId,
            Option<&KmpPathNode>,
            Option<&CheckpointLeft>,
            Option<&CheckpointRight>,
            Option<&RouteLink>,
            Option<&CheckpointRespawnLink>,
        )>()
        .iter(world)
    {
        allocations.insert(e, id.0);
        if let Some(node) = node {
            targets.extend(node.prev_nodes.iter().chain(&node.next_nodes).copied());
        }
        if let Some(left) = left {
            targets.insert(left.right);
        }
        if let Some(right) = right {
            targets.insert(right.left);
        }
        if let Some(route) = route {
            targets.insert(route.0);
        }
        if let Some(respawn) = respawn {
            targets.insert(respawn.0);
        }
    }
    let previous = world.remove_resource::<DocumentAliases>().unwrap_or_default();
    for target in targets {
        if let Some(&id) = previous.0.get(&target) {
            allocations.entry(target).or_insert(id);
        }
    }
    // Replacing instead of retaining also releases excess hash-table capacity
    // when a large document shrinks. Do not prune mid-restore observer cascades.
    world.insert_resource(DocumentAliases(allocations));
}

impl DocumentSnapshot {
    /// Compare float *bits*, including invalid intermediate values. NaNs must
    /// neither generate an edit every frame nor be normalized by undo.
    fn equivalent(&self, other: &Self) -> bool {
        fn scrub(snapshot: &mut DocumentSnapshot) -> Vec<u32> {
            let mut bits = Vec::new();
            let mut scalar = |v: &mut f32| {
                bits.push(v.to_bits());
                *v = 0.;
            };
            if let Some(track) = &mut snapshot.track {
                scalar(&mut track.speed_mod);
            }
            for point in &mut snapshot.points {
                if let Some(t) = &mut point.transform {
                    let mut rotation = t.rotation.to_array();
                    for v in t
                        .translation
                        .as_mut()
                        .iter_mut()
                        .chain(rotation.iter_mut())
                        .chain(t.scale.as_mut())
                    {
                        scalar(v);
                    }
                    t.rotation = Quat::IDENTITY;
                }
                if let Some(rotation) = &mut point.rotation {
                    for value in rotation.0.as_mut() {
                        scalar(value);
                    }
                }
                match &mut point.payload {
                    Payload::Enemy(v) => scalar(&mut v.leniency),
                    Payload::Item(v) => scalar(&mut v.bullet_control),
                    Payload::Object(v) => {
                        for x in v.scale.as_mut() {
                            scalar(x);
                        }
                    }
                    Payload::Area(v) => {
                        for x in v.scale.as_mut() {
                            scalar(x);
                        }
                    }
                    Payload::Camera(v) => {
                        scalar(&mut v.zoom_start);
                        scalar(&mut v.zoom_end);
                        scalar(&mut v.duration);
                        for x in v.view_start.as_mut().iter_mut().chain(v.view_end.as_mut()) {
                            scalar(x);
                        }
                    }
                    _ => {}
                }
            }
            bits
        }
        let (mut a, mut b) = (self.clone(), other.clone());
        scrub(&mut a) == scrub(&mut b) && a == b
    }

    pub fn capture(world: &mut World) -> Self {
        assign_identities(world);
        super::references::refresh(world);
        let key = |e| reference_key(world, e);
        let mut points = Vec::new();
        // Do not query Transform: temporarily missing components are still edits.
        for entity in world.iter_entities() {
            let Some(id) = entity.get::<DocumentId>() else { continue };
            let e = entity.id();
            let payload;
            macro_rules! payloads { ($($ty:ty => $variant:ident),*) => {
                payload = $(if let Some(value) = entity.get::<$ty>() { Payload::$variant(value.clone()) } else)*
                if entity.contains::<CheckpointRight>() { Payload::Right } else { continue };
            }; }
            payloads!(StartPoint => Start, EnemyPathPoint => Enemy, ItemPathPoint => Item,
                Checkpoint => Checkpoint, Object => Object, RoutePoint => Route,
                AreaPoint => Area, KmpCamera => Camera, RespawnPoint => Respawn,
                CannonPoint => Cannon, BattleFinishPoint => Finish);
            let payload = match payload {
                Payload::Area(mut area) => {
                    area.show_area = false;
                    Payload::Area(area)
                }
                value => value,
            };
            let node = entity.get::<KmpPathNode>().cloned().map(|mut node| {
                node.prev_nodes = node.prev_nodes.into_iter().map(key).collect();
                node.next_nodes = node.next_nodes.into_iter().map(key).collect();
                node
            });
            points.push(PointSnapshot {
                id: id.0,
                payload,
                transform: entity.get::<Transform>().copied().map(|mut t| {
                    // Checkpoint elevation is a view setting, not editable geometry.
                    if entity.contains::<Checkpoint>() || entity.contains::<CheckpointRight>() {
                        t.translation.y = 0.;
                    }
                    t
                }),
                rotation: entity.get::<KmpEulerRotation>().copied(),
                order: entity.get::<OrderId>().map(|o| o.0),
                node,
                pair: entity
                    .get::<CheckpointLeft>()
                    .map(|p| key(p.right))
                    .or_else(|| entity.get::<CheckpointRight>().map(|p| key(p.left))),
                route: entity.get::<RouteLink>().map(|l| key(l.0)),
                respawn: entity.get::<CheckpointRespawnLink>().map(|l| key(l.0)),
                settings: entity.get::<RouteSettings>().cloned(),
                references: entity.get::<super::references::NumericReferences>().cloned(),
                overall_start: world.get::<PathOverallStart>(e).is_some(),
                intro_start: world.get::<KmpCameraIntroStart>(e).is_some(),
            });
        }
        points.sort_by_key(|p| p.id);
        Self {
            points,
            track: world.get_resource::<TrackInfo>().cloned().map(|mut track| {
                track.track_type = TrackType::default(); // editor-only view mode
                track
            }),
        }
    }

    fn restore(&self, world: &mut World, views: &EntityHashMap<PointView>) {
        // Drain observer cascades completely before allocating replacement nodes.
        super::despawn_kmp_points(world);
        world.flush();
        world.insert_resource(RestoringDocument);
        let links: Vec<_> = world
            .query_filtered::<Entity, With<super::path::KmpPathNodeLink>>()
            .iter(world)
            .collect();
        for e in links {
            world.despawn(e);
        }
        // Allocate every identity before spawning payloads: checkpoint partners
        // and forward references may appear later in snapshot order.
        let mut map = EntityHashMap::default();
        for p in &self.points {
            map.insert(p.id, world.spawn_empty().id());
        }
        for p in &self.points {
            let e = map[&p.id];
            macro_rules! spawn {
                ($value:expr) => {
                    Spawner::builder()
                        .component($value.clone())
                        .rot(p.rotation.map(|rotation| rotation.0).unwrap_or_default())
                        .e(e)
                        .order_id(p.order.unwrap_or(0))
                        .build()
                        .spawn(world)
                };
            }
            match &p.payload {
                Payload::Start(v) => {
                    spawn!(v);
                }
                Payload::Enemy(v) => {
                    spawn!(v);
                }
                Payload::Item(v) => {
                    spawn!(v);
                }
                Payload::Object(v) => {
                    spawn!(v);
                }
                Payload::Route(v) => {
                    spawn!(v);
                }
                Payload::Area(v) => {
                    spawn!(v);
                }
                Payload::Camera(v) => {
                    spawn!(v);
                }
                Payload::Respawn(v) => {
                    spawn!(v);
                    super::point::AddRespawnPointPreview(e).apply(world);
                }
                Payload::Cannon(v) => {
                    spawn!(v);
                }
                Payload::Finish(v) => {
                    spawn!(v);
                }
                Payload::Checkpoint(v) => {
                    let partner = p.pair.and_then(|id| {
                        self.points.iter().find(|other| {
                            other.id == id && matches!(other.payload, Payload::Right) && other.pair == Some(p.id)
                        })
                    });
                    if let Some(partner) = partner {
                        checkpoint_spawner()
                            .cp(v.clone())
                            .left_e(e)
                            .right_e(map[&partner.id])
                            .order_id(p.order.unwrap_or(0))
                            .world(world)
                            .call();
                    } else {
                        spawn_incomplete_checkpoint(world, e);
                        world.entity_mut(e).insert(v.clone());
                        if let Some(right) = p.pair {
                            world.entity_mut(e).insert(CheckpointLeft {
                                right: map.get(&right).copied().unwrap_or(right),
                                ..default()
                            });
                        }
                    }
                }
                Payload::Right => {
                    // Paired right halves are created with their left, regardless
                    // of snapshot ordering. Orphans must not acquire a new mate.
                    let paired = p.pair.is_some_and(|id| {
                        self.points.iter().any(|other| {
                            other.id == id
                                && matches!(other.payload, Payload::Checkpoint(_))
                                && other.pair == Some(p.id)
                        })
                    });
                    if !paired {
                        spawn_incomplete_checkpoint(world, e);
                        let left = p.pair.unwrap_or(Entity::PLACEHOLDER);
                        world.entity_mut(e).insert(CheckpointRight {
                            left: map.get(&left).copied().unwrap_or(left),
                            ..default()
                        });
                    }
                }
            }
        }
        world.flush();
        // Spawn observers see empty graphs only. Install all nodes first, then
        // assign adjacency directly (insertion observers would drop future edges).
        for p in &self.points {
            let e = map[&p.id];
            world.entity_mut(e).insert((DocumentId(p.id), KmpSelectablePoint));
            if p.node.is_some() {
                world.entity_mut(e).insert(KmpPathNode::default());
            } else {
                world.entity_mut(e).remove::<KmpPathNode>();
            }
            world.entity_mut(e).remove::<(
                RouteSettings,
                RouteLinkedEntities,
                RouteLink,
                PathOverallStart,
                KmpCameraIntroStart,
            )>();
        }
        world.flush();
        for p in &self.points {
            let e = map[&p.id];
            if let Some(node) = &p.node {
                let mut node = node.clone();
                node.prev_nodes = node
                    .prev_nodes
                    .into_iter()
                    .map(|id| map.get(&id).copied().unwrap_or(id))
                    .collect();
                node.next_nodes = node
                    .next_nodes
                    .into_iter()
                    .map(|id| map.get(&id).copied().unwrap_or(id))
                    .collect();
                *world.get_mut::<KmpPathNode>(e).unwrap() = node;
            }
            if let Some(settings) = &p.settings {
                world
                    .entity_mut(e)
                    .insert((settings.clone(), RouteLinkedEntities::default()));
            }
        }
        for p in &self.points {
            let e = map[&p.id];
            if let Some(mut t) = p.transform {
                if matches!(p.payload, Payload::Checkpoint(_) | Payload::Right) {
                    t.translation.y = world.resource::<super::checkpoints::CheckpointHeight>().0;
                }
                world.entity_mut(e).insert(t);
            } else {
                world.entity_mut(e).remove::<Transform>();
            }
            if let Some(rotation) = p.rotation {
                world.entity_mut(e).insert(rotation);
            } else {
                world.entity_mut(e).remove::<KmpEulerRotation>();
            }
            if let Some(order) = p.order {
                world.entity_mut(e).insert(OrderId(order));
            } else {
                world.entity_mut(e).remove::<OrderId>();
            }
            if let Some(references) = &p.references {
                // These contain logical DocumentIds, not the old ECS handles.
                // Keep them verbatim even when the referenced point is absent.
                world.entity_mut(e).insert(references.clone());
            }
            if p.overall_start {
                world.entity_mut(e).insert(PathOverallStart);
            }
            if p.intro_start {
                world.entity_mut(e).insert(KmpCameraIntroStart);
            }
            if let Some(id) = p.route {
                let target = map.get(&id).copied().unwrap_or(id);
                // Owner sets precede forward links, including intermediate routes
                // that have no settings yet.
                if let Ok(mut owner) = world.get_entity_mut(target) {
                    owner.insert_if_new(RouteLinkedEntities::default());
                }
                world.entity_mut(e).insert(RouteLink(target));
                if let Some(mut owners) = world.get_mut::<RouteLinkedEntities>(target) {
                    owners.insert(e);
                }
            }
            if let Some(id) = p.respawn {
                world
                    .entity_mut(e)
                    .insert(CheckpointRespawnLink(map.get(&id).copied().unwrap_or(id)));
            }
            if let Some(view) = views.get(&p.id) {
                world.entity_mut(e).insert(view.visibility);
                if view.selected {
                    world.entity_mut(e).insert(Selected);
                }
                if let Some(mut area) = world.get_mut::<AreaPoint>(e) {
                    area.show_area = view.show_area;
                }
            }
        }
        if let Some(track) = &self.track {
            let mut track = track.clone();
            track.track_type = world
                .get_resource::<TrackInfo>()
                .map(|current| current.track_type.clone())
                .unwrap_or_default();
            world.insert_resource(track);
        } else {
            world.remove_resource::<TrackInfo>();
        }
        world.flush();
        world.remove_resource::<RestoringDocument>();
        assign_identities(world);
        refresh_derived(world);
        world.init_resource::<RestoreGeneration>();
        world.resource_mut::<RestoreGeneration>().0 += 1;
        if let Some(mut gizmo) = world.get_resource_mut::<TransformGizmoState>() {
            gizmo.reset_interaction();
        }
    }
}

/// Malformed checkpoint halves remain selectable points, but have no invented
/// partner or pair geometry. Snapshot pairing is installed separately.
fn spawn_incomplete_checkpoint(world: &mut World, e: Entity) {
    use crate::viewer::edit::{
        transform_gizmo::GizmoTransformable,
        tweak::{SnapTo, Tweakable},
    };
    let mesh = world.resource::<super::meshes_materials::KmpMeshes>().sphere.clone();
    let material = world
        .resource::<super::meshes_materials::CheckpointMaterials>()
        .normal
        .clone();
    world.entity_mut(e).insert((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::IDENTITY,
        Visibility::Visible,
        KmpSelectablePoint,
        CheckpointMarker,
        GizmoTransformable,
        Tweakable(SnapTo::CheckpointPlane),
        TransformEditOptions::new(true, true),
        crate::viewer::normalize::Normalize::new(200., 30., BVec3::TRUE),
    ));
}

fn refresh_derived(world: &mut World) {
    use super::ordering::{NextOrderID, RefreshOrdering};
    use bevy::ecs::system::RunSystemOnce;
    // A queued deletion refresh must not renumber the exact restored OrderIds.
    if let Some(mut messages) = world.get_resource_mut::<Messages<RefreshOrdering>>() {
        messages.clear();
    }
    if let Some(mut messages) = world.get_resource_mut::<Messages<RecalcPaths>>() {
        messages.clear();
    }
    fn counter<T: Component>(world: &mut World) {
        let next = world
            .query_filtered::<&OrderId, With<T>>()
            .iter(world)
            .map(|id| id.0.saturating_add(1))
            .max()
            .unwrap_or(0);
        world.init_resource::<NextOrderID<T>>();
        world.resource::<NextOrderID<T>>().set(next);
    }
    macro_rules! counters { ($($t:ty),*) => { $(counter::<$t>(world);)* }; }
    counters!(
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
    macro_rules! groups { ($($t:ty),*) => { $(super::path::refresh_groups::<$t>(world);)* }; }
    groups!(EnemyPathPoint, ItemPathPoint, Checkpoint, RoutePoint);
    macro_rules! links { ($($t:ty),*) => { $(world.run_system_once(super::path::reconcile_node_links::<$t>)
        .expect("restore link queries are valid");)* }; }
    links!(EnemyPathPoint, ItemPathPoint, Checkpoint, CheckpointRight, RoutePoint);
    world.remove_resource::<super::KmpSectionEntityIdMap<RouteSettings>>();
    world.remove_resource::<super::KmpSectionEntityIdMap<RespawnPoint>>();
    world.flush();
}

fn cancel_interactions(world: &mut World) {
    use crate::viewer::edit::{
        area_gizmo::AreaGizmoOptions,
        create_delete::{CreatePoint, JustCreatedPoint},
        link_select_mode,
        select::SelectBox,
    };
    link_select_mode::cancel::<RoutePoint>(world);
    link_select_mode::cancel::<RespawnPoint>(world);
    if let Some(mut state) = world.get_resource_mut::<SelectBox>() {
        *state = SelectBox::default();
    }
    if let Some(mut state) = world.get_resource_mut::<AreaGizmoOptions>() {
        *state = AreaGizmoOptions::default();
    }
    if let Some(mut messages) = world.get_resource_mut::<Messages<CreatePoint>>() {
        messages.clear();
    }
    if let Some(mut messages) = world.get_resource_mut::<Messages<JustCreatedPoint>>() {
        messages.clear();
    }
}

fn cache_views(world: &mut World, history: &mut DocumentHistory) {
    for (e, id, visibility, selected) in world
        .query::<(Entity, &DocumentId, Option<&Visibility>, Has<Selected>)>()
        .iter(world)
    {
        history.views.insert(
            id.0,
            PointView {
                selected,
                visibility: visibility.copied().unwrap_or(Visibility::Visible),
                show_area: world.get::<AreaPoint>(e).is_some_and(|a| a.show_area),
            },
        );
    }
}

pub fn reset(world: &mut World) {
    // A successful open starts a new identity epoch. Never retain allocations
    // from the previous document (failed opens do not call reset).
    world.remove_resource::<DocumentAliases>();
    cancel_interactions(world);
    world.init_resource::<RestoreGeneration>();
    world.resource_mut::<RestoreGeneration>().0 += 1;
    if let Some(mut gizmo) = world.get_resource_mut::<TransformGizmoState>() {
        gizmo.reset_interaction();
    }
    let current = DocumentSnapshot::capture(world);
    let mut history = DocumentHistory {
        current: Some(current),
        ..default()
    };
    cache_views(world, &mut history);
    history.prune_views(world);
    world.insert_resource(history);
}

pub fn checkpoint(world: &mut World) {
    if !world.contains_resource::<super::document::LoadedKmp>() {
        return;
    }
    let snapshot = DocumentSnapshot::capture(world);
    let mut history = world.remove_resource::<DocumentHistory>().unwrap_or_default();
    cache_views(world, &mut history);
    history.stage(snapshot);
    history.commit();
    history.prune_views(world);
    world.insert_resource(history);
}

pub fn undo(world: &mut World, redo: bool) {
    cancel_interactions(world);
    checkpoint(world); // Flush a pending text/drag action before moving history.
    let mut history = world.remove_resource::<DocumentHistory>().unwrap_or_default();
    let snapshot = if redo { history.redo.pop() } else { history.undo.pop() };
    if let Some(snapshot) = snapshot {
        if let Some(current) = history.current.replace(snapshot.clone()) {
            if redo {
                history.undo.push(current);
            } else {
                history.redo.push(current);
            }
        }
        snapshot.restore(world, &history.views);
    }
    history.prune_views(world);
    world.insert_resource(history);
}

impl DocumentHistory {
    fn prune_views(&mut self, world: &mut World) {
        let mut reachable: bevy::ecs::entity::EntityHashSet =
            world.query::<&DocumentId>().iter(world).map(|id| id.0).collect();
        for snapshot in self
            .current
            .iter()
            .chain(self.pending.iter())
            .chain(&self.undo)
            .chain(&self.redo)
        {
            reachable.extend(snapshot.points.iter().map(|point| point.id));
        }
        self.views.retain(|id, _| reachable.contains(id));
        self.views.shrink_to_fit();
    }

    fn pending_changed(&self) -> bool {
        self.pending
            .as_ref()
            .is_some_and(|pending| self.current.as_ref().is_none_or(|current| !current.equivalent(pending)))
    }

    fn stage(&mut self, snapshot: DocumentSnapshot) {
        if self
            .current
            .as_ref()
            .is_none_or(|current| !current.equivalent(&snapshot))
        {
            // Invalidate immediately, not just on release/focus loss.
            self.redo.clear();
        }
        self.pending = Some(snapshot);
    }

    fn commit(&mut self) {
        if self.pending_changed() {
            if let Some(previous) = self.current.replace(self.pending.take().unwrap()) {
                self.undo.push(previous);
                if self.undo.len() > CAPACITY {
                    self.undo.remove(0);
                }
            }
        }
        self.pending = None;
        self.group = None;
        self.text_edited = false;
    }

    fn observe(&mut self, snapshot: DocumentSnapshot, input: GroupInput) {
        let next = input
            .text
            .map(EditGroup::Text)
            .or(input.pointer.then_some(EditGroup::Pointer));
        // Tab can already have moved focus forward, or can leave it on the old
        // field until the next pass (Shift+Tab in egui 0.36). In either case the
        // final value on this frame belongs to the transaction being finished.
        if input.finish {
            self.stage(snapshot);
            self.commit();
            return;
        }
        // Switching directly between fields or from a field to a drag ends the
        // previous session at its last observed value, not the new field's value.
        if self.group.is_some() && next.is_some() && self.group != next {
            self.commit();
        }
        self.stage(snapshot);
        self.group = next;
        self.text_edited |= matches!(next, Some(EditGroup::Text(_))) && self.pending_changed();
        // Include final release-frame mutations in the same action.
        if next.is_none() {
            self.commit();
        }
    }

    pub(crate) fn owns_text_undo(&self, focused: Option<bevy_egui::egui::Id>) -> bool {
        self.text_edited && focused.is_some_and(|id| self.group == Some(EditGroup::Text(id)))
    }

    pub fn can_undo(&self) -> bool {
        self.pending_changed() || !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.pending_changed() && !self.redo.is_empty()
    }
}

/// Recheck at dispatch as well as at shortcut/menu generation: a dialog can
/// open later in the frame after the keyboard system has queued its request.
pub fn requests_blocked(world: &World) -> bool {
    world
        .get_resource::<crate::ui::file_dialog::FileDialogRes>()
        .is_some_and(|dialog| dialog.0.is_some())
        || world
            .get_resource::<crate::ui::unsaved_changes::PendingDocumentAction>()
            .is_some_and(|action| action.is_pending())
}

fn record_with_input(world: &mut World, input: GroupInput) {
    if !world.contains_resource::<super::document::LoadedKmp>() {
        return;
    }
    let snapshot = DocumentSnapshot::capture(world);
    let mut history = world.remove_resource::<DocumentHistory>().unwrap_or_default();
    cache_views(world, &mut history);
    history.observe(snapshot, input);
    history.prune_views(world);
    world.insert_resource(history);
}

fn record_frame(world: &mut World) {
    let request = world.resource_mut::<DocumentHistory>().request.take();
    if let Some(redo) = request.filter(|_| !requests_blocked(world)) {
        undo(world, redo);
        return;
    }
    let mut input = GroupInput {
        pointer: world
            .get_resource::<ButtonInput<MouseButton>>()
            .is_some_and(|b| b.pressed(MouseButton::Left)),
        ..default()
    };
    if let Ok(mut context) = world
        .query_filtered::<&mut bevy_egui::EguiContext, With<bevy_egui::PrimaryEguiContext>>()
        .single_mut(world)
    {
        let ctx = context.get_mut();
        if ctx.egui_wants_keyboard_input() {
            input.text = ctx.memory(|memory| memory.focused());
        }
        input.finish =
            ctx.input(|i| i.key_pressed(bevy_egui::egui::Key::Enter) || i.key_pressed(bevy_egui::egui::Key::Tab));
        input.pointer |= ctx.input(|i| i.pointer.primary_down());
    }
    record_with_input(world, input);
}
