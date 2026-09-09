use crate::viewer::edit::select::Selected;

use super::{
    path::{KmpPathNode, RecalcPaths},
    KmpComponent, KmpFile, KmpSectionIdEntityMap, RoutePoint, RouteSettings, Spawner,
};
use bevy::{
    ecs::{entity::EntityHashSet, system::SystemParam},
    platform::collections::HashMap,
    prelude::*,
};

use serde::{Deserialize, Serialize};

pub fn routes_plugin(app: &mut App) {
    app.add_systems(Update, update_routes)
        .add_observer(on_add_route_linked_entities)
        .add_observer(on_remove_route_linked_entities)
        .add_observer(on_add_route_link)
        .add_observer(on_remove_route_link)
        .add_observer(on_add_route_pt)
        .add_observer(on_remove_route_pt);
}

#[derive(Component, Default, Clone, Serialize, Deserialize, Debug, Deref, DerefMut)]
/// Struct that is attached to the start route and contains links to entities that this route is linked to
pub struct RouteLinkedEntities(pub EntityHashSet);
impl RouteLinkedEntities {
    /// Move the route start to a new entity, updating all the route references
    pub fn move_route_start(&self, world: &mut World, self_e: Entity, new_e: Entity) {
        if world.get_entity(new_e).is_err() {
            return;
        }

        if let Ok(mut old_start) = world.get_entity_mut(self_e) {
            old_start.remove::<RouteLinkedEntities>();
        }

        let mut moved_links = RouteLinkedEntities::default();
        for linked_e in self.iter() {
            let Some(mut route_link) = world.get_mut::<RouteLink>(*linked_e) else {
                continue;
            };
            if route_link.0 != self_e {
                continue;
            }
            **route_link = new_e;
            moved_links.insert(*linked_e);
        }
        world.entity_mut(new_e).insert(moved_links);
    }
}

#[derive(Bundle, Default)]
pub struct RouteStartBundle {
    route_linked_entities: RouteLinkedEntities,
    route_settings: RouteSettings,
}

fn on_add_route_linked_entities(
    trigger: On<Add, RouteLinkedEntities>,
    q_route_linked_es: Query<&RouteLinkedEntities>,
    mut commands: Commands,
) {
    let e = trigger.event().entity;
    let Ok(route_linked_es) = q_route_linked_es.get(e) else {
        return;
    };
    let linked_entities = route_linked_es.0.clone();

    // Make reciprocal links for live entities and discard stale references.
    commands.queue(move |world: &mut World| {
        for linked_e in linked_entities {
            if world.get_entity(linked_e).is_err() {
                if let Some(mut route_linked_es) = world.get_mut::<RouteLinkedEntities>(e) {
                    route_linked_es.remove(&linked_e);
                }
                continue;
            }

            match world.get::<RouteLink>(linked_e) {
                Some(link) if link.0 == e => {}
                Some(_) => {
                    if let Some(mut route_linked_es) = world.get_mut::<RouteLinkedEntities>(e) {
                        route_linked_es.remove(&linked_e);
                    }
                }
                None => {
                    world.entity_mut(linked_e).insert(RouteLink(e));
                }
            }
        }
    });
}
fn on_remove_route_linked_entities(
    trigger: On<Remove, RouteLinkedEntities>,
    q_route_linked_es: Query<&RouteLinkedEntities>,
    q_kmp_path_node: Query<&KmpPathNode>,
    mut commands: Commands,
) {
    let e = trigger.event().entity;
    let Ok(route_linked_es) = q_route_linked_es.get(e).cloned() else {
        return;
    };
    let next_e = q_kmp_path_node
        .get(e)
        .ok()
        .and_then(|node| node.next_nodes.iter().next().copied());

    commands.queue(move |world: &mut World| {
        // Removing a stale RouteStartBundle from a live point is housekeeping:
        // update_routes has already transferred its links to the canonical start.
        // Only migrate links here when the route-start entity itself was despawned.
        if world.get_entity(e).is_ok() {
            return;
        }

        // When possible, move the route start forward. If its path data or next
        // node has already gone, remove only reciprocal references to this start.
        if let Some(next_e) = next_e
            .filter(|next_e| world.get::<KmpPathNode>(*next_e).is_some() && world.get::<RoutePoint>(*next_e).is_some())
        {
            route_linked_es.move_route_start(world, e, next_e);
        } else {
            for linked_e in route_linked_es.iter() {
                if world.get::<RouteLink>(*linked_e).is_some_and(|link| link.0 == e) {
                    world.entity_mut(*linked_e).remove::<RouteLink>();
                }
            }
        }
    });
}

#[derive(Component, Clone, Serialize, Deserialize, Debug, Deref, DerefMut)]
pub struct RouteLink(pub Entity);

fn on_add_route_link(
    trigger: On<Add, RouteLink>,
    q_route_link: Query<&RouteLink>,
    mut q_route_linked_es: Query<&mut RouteLinkedEntities>,
    mut commands: Commands,
) {
    let e = trigger.event().entity;
    let Ok(linked_e) = q_route_link.get(e).map(|link| link.0) else {
        return;
    };

    let Ok(mut route_linked_es) = q_route_linked_es.get_mut(linked_e) else {
        commands.entity(e).remove::<RouteLink>();
        return;
    };
    // check that the we are included in the list of linked entities
    route_linked_es.insert(e);
}
fn on_remove_route_link(
    trigger: On<Remove, RouteLink>,
    q_route_link: Query<&RouteLink>,
    mut q_route_linked_es: Query<&mut RouteLinkedEntities>,
) {
    let e = trigger.event().entity;
    let Ok(linked_e) = q_route_link.get(e).map(|link| link.0) else {
        return;
    };

    if let Ok(mut route_linked_es) = q_route_linked_es.get_mut(linked_e) {
        // remove ourselves from the list of linked entities to the route
        route_linked_es.remove(&e);
    }
}

fn on_add_route_pt(trigger: On<Add, RoutePoint>, q_kmp_path_node: Query<&KmpPathNode>, mut commands: Commands) {
    let e = trigger.event().entity;
    let Ok(kmp_path_node) = q_kmp_path_node.get(e) else {
        return;
    };

    // if we have started a new route path, add route settings and route linked entities to it because it is the first point
    if kmp_path_node.prev_nodes.is_empty() {
        commands.entity(e).insert(RouteStartBundle::default());
    }
}

fn on_remove_route_pt(
    trigger: On<Remove, RoutePoint>,
    mut commands: Commands,
    q_kmp_path_node: Query<&KmpPathNode>,
    mut ev_recalc_paths: MessageWriter<RecalcPaths>,
) {
    // we will have to add 'route settings' and 'route linked entities' components to the next entity,
    // because that entity is now the start of a new route now that we've been deleted
    let e = trigger.event().entity;
    // check if there is a next entity because we might be at the end of the route
    if let Some(new_start_e) = q_kmp_path_node
        .get(e)
        .ok()
        .and_then(|node| node.next_nodes.iter().next())
        .filter(|new_start_e| q_kmp_path_node.contains(**new_start_e))
    {
        commands.entity(*new_start_e).insert(RouteStartBundle::default());
        ev_recalc_paths.write(RecalcPaths::route());
    }
}

pub fn spawn_route_section(world: &mut World, kmp: &KmpFile) -> KmpSectionIdEntityMap<RoutePoint> {
    let mut id_entity_map = HashMap::default();
    for (i, route) in kmp.poti.iter().enumerate() {
        let mut prev_e: Option<Entity> = None;
        for route_pt in route.points.iter() {
            let e = Spawner::builder()
                .component(RoutePoint::from_kmp(route_pt, world))
                .pos(route_pt.position)
                .visible(false)
                .prev_nodes(prev_e.into_iter().collect::<EntityHashSet>())
                .max(1)
                .build()
                .spawn(world);

            // insert the route settings to the first route point
            if prev_e.is_none() {
                let route_settings = RouteSettings::from_kmp(route, world);
                world.entity_mut(e).insert(RouteStartBundle {
                    route_settings,
                    ..default()
                });
            }

            // if we are at the first route point
            if prev_e.is_none() {
                id_entity_map.insert(i as u32, e);
            }

            prev_e = Some(e);
        }
    }
    KmpSectionIdEntityMap::new(id_entity_map)
}

pub fn update_routes(
    q_route_pts: Query<(Entity, Has<RouteLinkedEntities>, &KmpPathNode), With<RoutePoint>>,
    get_route_start: GetRouteStart,
    mut q_linked_entities: Query<&mut RouteLinkedEntities>,
    mut q_route_link: Query<&mut RouteLink>,
    mut commands: Commands,
    q_route_start: Query<(), (With<RouteSettings>, With<RouteLinkedEntities>)>,
) {
    for (e, is_route_start, kmp_path_node) in q_route_pts.iter() {
        let route_start_e = get_route_start.get_entity(e);

        if !q_route_start.contains(route_start_e) {
            commands.entity(route_start_e).insert(RouteStartBundle::default());
            continue;
        }

        // check if there are any entities linked to parts of the route that are not the start
        // if so, we need to link the entities to the start component
        if is_route_start && !kmp_path_node.prev_nodes.is_empty() {
            let Ok(linked_entities) = q_linked_entities.get(e).cloned() else {
                continue;
            };

            // Move only live reciprocal links to the actual route start.
            let Ok(mut route_start_linked_entities) = q_linked_entities.get_mut(route_start_e) else {
                continue;
            };
            for linked_e in linked_entities.iter() {
                let Ok(mut route_link) = q_route_link.get_mut(*linked_e) else {
                    continue;
                };
                if route_link.0 != e {
                    continue;
                }
                **route_link = route_start_e;
                route_start_linked_entities.insert(*linked_e);
            }
            commands.entity(e).remove::<RouteStartBundle>();
        }
        //
    }
}

#[derive(SystemParam)]
pub struct GetRouteStart<'w, 's> {
    q: Query<'w, 's, (Entity, &'static KmpPathNode)>,
    q_selected: Query<'w, 's, Entity, (With<Selected>, With<RoutePoint>)>,
}
impl GetRouteStart<'_, '_> {
    pub fn get_entity(&self, mut cur_e: Entity) -> Entity {
        let mut visited = EntityHashSet::default();
        while visited.insert(cur_e) {
            let Some(prev_e) = self.q.get(cur_e).ok().and_then(|x| x.1.prev_nodes.iter().next()) else {
                break;
            };
            let Ok((_, prev_node)) = self.q.get(*prev_e) else {
                break;
            };
            if !prev_node.next_nodes.contains(&cur_e) {
                break;
            }
            cur_e = *prev_e;
        }
        cur_e
    }
    pub fn get_selected(&self) -> EntityHashSet {
        let entities = self.q_selected.iter();
        self.get_multiple_entities(entities)
    }
    pub fn get_multiple_entities(&self, entities: impl IntoIterator<Item = Entity>) -> EntityHashSet {
        let mut start_es = EntityHashSet::default();
        for e in entities {
            let start_e = self.get_entity(e);
            start_es.insert(start_e);
        }
        start_es
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::SystemState;

    #[test]
    fn moving_route_start_drops_stale_linked_entities() {
        let mut world = World::new();
        let old_start = world.spawn_empty().id();
        let new_start = world.spawn_empty().id();
        let linked = world.spawn(RouteLink(old_start)).id();
        let stale = world.spawn_empty().id();
        world.despawn(stale);

        let mut links = RouteLinkedEntities::default();
        links.extend([linked, stale]);
        links.move_route_start(&mut world, old_start, new_start);

        assert_eq!(world.get::<RouteLink>(linked).unwrap().0, new_start);
        let moved_links = world.get::<RouteLinkedEntities>(new_start).unwrap();
        assert_eq!(moved_links.len(), 1);
        assert!(moved_links.contains(&linked));
    }

    #[test]
    fn removing_stale_route_start_from_live_point_does_not_move_links() {
        let mut app = App::new();
        app.add_observer(on_remove_route_linked_entities);

        let canonical_start = app
            .world_mut()
            .spawn((RoutePoint::default(), KmpPathNode::default()))
            .id();
        let linked = app.world_mut().spawn(RouteLink(canonical_start)).id();
        let mut canonical_links = RouteLinkedEntities::default();
        canonical_links.insert(linked);
        app.world_mut().entity_mut(canonical_start).insert(canonical_links);

        let stale_start = app
            .world_mut()
            .spawn((
                RoutePoint::default(),
                KmpPathNode::default().with_next([canonical_start]),
                RouteLinkedEntities::default(),
            ))
            .id();
        app.world_mut().entity_mut(stale_start).remove::<RouteLinkedEntities>();

        let canonical_links = app.world().get::<RouteLinkedEntities>(canonical_start).unwrap();
        assert_eq!(canonical_links.len(), 1);
        assert!(canonical_links.contains(&linked));
        assert!(app.world().get::<RouteLinkedEntities>(stale_start).is_none());
    }

    #[test]
    fn route_start_traversal_stops_at_stale_predecessor() {
        let mut world = World::new();
        let stale = world.spawn_empty().id();
        world.despawn(stale);
        let route_point = world.spawn(KmpPathNode::default().with_prev([stale])).id();

        let mut state = SystemState::<GetRouteStart>::new(&mut world);
        let route_start = state.get_mut(&mut world).get_entity(route_point);
        state.apply(&mut world);

        assert_eq!(route_start, route_point);
    }
}
