//! Typed, in-memory document history. Entity values used as keys here are logical
//! identities, never handles to be dereferenced without the live identity map.
use super::{
    checkpoints::{checkpoint_spawner, CheckpointLeft, CheckpointRight, CheckpointRespawnLink},
    components::*, ordering::OrderId, path::{KmpPathNode, RecalcPaths},
    routes::{RouteLink, RouteLinkedEntities},
};
use crate::viewer::edit::{select::Selected, transform_gizmo::TransformGizmoState};
use bevy::{ecs::entity::EntityHashMap, prelude::*};

#[derive(Component, Clone, Copy)]
pub struct DocumentId(pub Entity);

/// Incremented on restoration so Local interaction state can discard stale handles.
#[derive(Resource, Default)]
pub struct RestoreGeneration(pub u64);

#[derive(Clone, PartialEq)]
enum Payload {
    Start(StartPoint), Enemy(EnemyPathPoint), Item(ItemPathPoint),
    Checkpoint(Checkpoint), Right, Object(Object), Route(RoutePoint),
    Area(AreaPoint), Camera(KmpCamera), Respawn(RespawnPoint),
    Cannon(CannonPoint), Finish(BattleFinishPoint),
}

#[derive(Clone, PartialEq)]
struct PointSnapshot {
    id: Entity,
    payload: Payload,
    transform: Option<Transform>,
    order: Option<u32>,
    node: Option<KmpPathNode>,
    pair: Option<Entity>,
    route: Option<Entity>,
    respawn: Option<Entity>,
    settings: Option<RouteSettings>,
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
    current: Option<DocumentSnapshot>,
    undo: Vec<DocumentSnapshot>,
    redo: Vec<DocumentSnapshot>,
    views: EntityHashMap<PointView>,
    pub request: Option<bool>,
}

const CAPACITY: usize = 100;

pub fn plugin(app: &mut App) {
    app.init_resource::<DocumentHistory>()
        .init_resource::<RestoreGeneration>()
        .add_systems(Last, record_frame.before(super::save_kmp));
}

pub fn identities(world: &mut World) -> EntityHashMap<Entity> {
    world.query::<(Entity, &DocumentId)>().iter(world).map(|(e, id)| (id.0, e)).collect()
}

fn assign_identities(world: &mut World) {
    let entities: Vec<_> = world.query_filtered::<Entity, (With<KmpSelectablePoint>, Without<DocumentId>)>()
        .iter(world).collect();
    for e in entities { world.entity_mut(e).insert(DocumentId(e)); }
}

impl DocumentSnapshot {
    pub fn capture(world: &mut World) -> Self {
        assign_identities(world);
        let key = |e| world.get::<DocumentId>(e).map_or(e, |id| id.0);
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
                Payload::Area(mut area) => { area.show_area = false; Payload::Area(area) },
                value => value,
            };
            let node = entity.get::<KmpPathNode>().cloned().map(|mut node| {
                node.prev_nodes = node.prev_nodes.into_iter().map(key).collect();
                node.next_nodes = node.next_nodes.into_iter().map(key).collect();
                node
            });
            points.push(PointSnapshot {
                id: id.0, payload, transform: entity.get::<Transform>().copied(),
                order: entity.get::<OrderId>().map(|o| o.0), node,
                pair: entity.get::<CheckpointLeft>().map(|p| key(p.right))
                    .or_else(|| entity.get::<CheckpointRight>().map(|p| key(p.left))),
                route: entity.get::<RouteLink>().map(|l| key(l.0)),
                respawn: entity.get::<CheckpointRespawnLink>().map(|l| key(l.0)),
                settings: entity.get::<RouteSettings>().cloned(),
                overall_start: world.get::<PathOverallStart>(e).is_some(),
                intro_start: world.get::<KmpCameraIntroStart>(e).is_some(),
            });
        }
        points.sort_by_key(|p| p.id);
        Self { points, track: world.get_resource::<TrackInfo>().cloned() }
    }

    fn restore(&self, world: &mut World, views: &EntityHashMap<PointView>) {
        // Drain observer cascades completely before allocating replacement nodes.
        super::despawn_kmp_points(world);
        world.flush();
        let mut map = EntityHashMap::default();
        for p in &self.points { map.insert(p.id, world.spawn_empty().id()); }
        for p in &self.points {
            let e = map[&p.id];
            macro_rules! spawn { ($value:expr) => {
                Spawner::builder().component($value.clone()).e(e).order_id(p.order.unwrap_or(0)).build().spawn(world)
            }; }
            match &p.payload {
                Payload::Start(v) => { spawn!(v); }, Payload::Enemy(v) => { spawn!(v); },
                Payload::Item(v) => { spawn!(v); }, Payload::Object(v) => { spawn!(v); },
                Payload::Route(v) => { spawn!(v); }, Payload::Area(v) => { spawn!(v); },
                Payload::Camera(v) => { spawn!(v); }, Payload::Respawn(v) => {
                    spawn!(v); super::point::AddRespawnPointPreview(e).apply(world);
                },
                Payload::Cannon(v) => { spawn!(v); }, Payload::Finish(v) => { spawn!(v); },
                Payload::Checkpoint(v) => {
                    let right = p.pair.and_then(|id| map.get(&id).copied());
                    checkpoint_spawner().cp(v.clone()).left_e(e).maybe_right_e(right)
                        .order_id(p.order.unwrap_or(0)).world(world).call();
                },
                Payload::Right => {},
            }
        }
        world.flush();
        // Spawn observers see empty graphs only. Install all nodes first, then
        // assign adjacency directly (insertion observers would drop future edges).
        for p in &self.points {
            let e = map[&p.id];
            world.entity_mut(e).insert((DocumentId(p.id), KmpSelectablePoint));
            if p.node.is_some() { world.entity_mut(e).insert(KmpPathNode::default()); }
            else { world.entity_mut(e).remove::<KmpPathNode>(); }
            world.entity_mut(e).remove::<(RouteSettings, RouteLinkedEntities, RouteLink, PathOverallStart, KmpCameraIntroStart)>();
        }
        world.flush();
        for p in &self.points {
            let e = map[&p.id];
            if let Some(node) = &p.node {
                let mut node = node.clone();
                node.prev_nodes = node.prev_nodes.into_iter().map(|id| map.get(&id).copied().unwrap_or(id)).collect();
                node.next_nodes = node.next_nodes.into_iter().map(|id| map.get(&id).copied().unwrap_or(id)).collect();
                *world.get_mut::<KmpPathNode>(e).unwrap() = node;
            }
            if let Some(settings) = &p.settings { world.entity_mut(e).insert((settings.clone(), RouteLinkedEntities::default())); }
        }
        for p in &self.points {
            let e = map[&p.id];
            if let Some(t) = p.transform { world.entity_mut(e).insert(t); }
            else { world.entity_mut(e).remove::<Transform>(); }
            if let Some(order) = p.order { world.entity_mut(e).insert(OrderId(order)); }
            else { world.entity_mut(e).remove::<OrderId>(); }
            if p.overall_start { world.entity_mut(e).insert(PathOverallStart); }
            if p.intro_start { world.entity_mut(e).insert(KmpCameraIntroStart); }
            if let Some(id) = p.route {
                let target = map.get(&id).copied().unwrap_or(id);
                // Owner sets precede forward links, including intermediate routes
                // that have no settings yet.
                if let Ok(mut owner) = world.get_entity_mut(target) { owner.insert_if_new(RouteLinkedEntities::default()); }
                world.entity_mut(e).insert(RouteLink(target));
            }
            if let Some(id) = p.respawn { world.entity_mut(e).insert(CheckpointRespawnLink(map.get(&id).copied().unwrap_or(id))); }
            if let Some(view) = views.get(&p.id) {
                world.entity_mut(e).insert(view.visibility);
                if view.selected { world.entity_mut(e).insert(Selected); }
                if let Some(mut area) = world.get_mut::<AreaPoint>(e) { area.show_area = view.show_area; }
            }
        }
        if let Some(track) = &self.track { world.insert_resource(track.clone()); }
        else { world.remove_resource::<TrackInfo>(); }
        world.flush();
        world.write_message(RecalcPaths::all());
        world.init_resource::<RestoreGeneration>();
        world.resource_mut::<RestoreGeneration>().0 += 1;
        if let Some(mut gizmo) = world.get_resource_mut::<TransformGizmoState>() { gizmo.reset_interaction(); }
    }
}

fn cache_views(world: &mut World, history: &mut DocumentHistory) {
    for (e, id, visibility, selected) in world.query::<(Entity, &DocumentId, Option<&Visibility>, Has<Selected>)>().iter(world) {
        history.views.insert(id.0, PointView {
            selected, visibility: visibility.copied().unwrap_or(Visibility::Visible),
            show_area: world.get::<AreaPoint>(e).is_some_and(|a| a.show_area),
        });
    }
}

pub fn reset(world: &mut World) {
    let current = DocumentSnapshot::capture(world);
    let mut history = DocumentHistory { current: Some(current), ..default() };
    cache_views(world, &mut history);
    world.insert_resource(history);
}

pub fn checkpoint(world: &mut World) {
    if !world.contains_resource::<super::document::LoadedKmp>() { return; }
    let snapshot = DocumentSnapshot::capture(world);
    let mut history = world.remove_resource::<DocumentHistory>().unwrap_or_default();
    cache_views(world, &mut history);
    if history.current.as_ref() != Some(&snapshot) {
        if let Some(previous) = history.current.replace(snapshot) {
            history.undo.push(previous);
            if history.undo.len() > CAPACITY { history.undo.remove(0); }
            history.redo.clear();
        }
    }
    world.insert_resource(history);
}

pub fn undo(world: &mut World, redo: bool) {
    checkpoint(world);
    let mut history = world.remove_resource::<DocumentHistory>().unwrap_or_default();
    let snapshot = if redo { history.redo.pop() } else { history.undo.pop() };
    if let Some(snapshot) = snapshot {
        if let Some(current) = history.current.replace(snapshot.clone()) {
            if redo { history.undo.push(current); } else { history.redo.push(current); }
        }
        snapshot.restore(world, &history.views);
    }
    world.insert_resource(history);
}

impl DocumentHistory {
    pub fn can_undo(&self) -> bool { !self.undo.is_empty() }
    pub fn can_redo(&self) -> bool { !self.redo.is_empty() }
}

fn record_frame(world: &mut World) {
    let request = world.resource_mut::<DocumentHistory>().request.take();
    if let Some(redo) = request { undo(world, redo); return; }
    let dragging = world.get_resource::<ButtonInput<MouseButton>>().is_some_and(|b| b.pressed(MouseButton::Left));
    if !dragging { checkpoint(world); }
}
