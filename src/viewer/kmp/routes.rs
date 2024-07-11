use super::{path::KmpPathNode, FromKmp, KmpError, KmpFile, RoutePoint, RouteSettings, Spawner};
use bevy::{
    ecs::{
        component::{ComponentHooks, StorageType},
        entity::EntityHashSet,
        system::SystemParam,
    },
    prelude::*,
    utils::EntityHashMap,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub fn routes_plugin(app: &mut App) {
    app.add_systems(Update, update_routes);
}

#[derive(Default, Clone, Serialize, Deserialize, Debug, Deref, DerefMut)]
/// Struct that is attached to the start route and contains links to entities that this route is linked to
pub struct RouteLinkedEntities(pub EntityHashSet);
impl RouteLinkedEntities {
    /// Move the route start to a new entity, updating all the route references
    pub fn move_route_start(&self, world: &mut World, self_e: Entity, new_e: Entity) {
        world.entity_mut(self_e).remove::<RouteLinkedEntities>();
        for linked_e in self.iter() {
            let mut route_link = world.get_mut::<RouteLink>(*linked_e).unwrap();
            **route_link = new_e;
        }
        world.entity_mut(new_e).insert(self.clone());
    }
}

impl Component for RouteLinkedEntities {
    const STORAGE_TYPE: StorageType = StorageType::Table;
    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks.on_add(|mut world, e, _| {
            let route_linked_es: Vec<_> = world.get::<RouteLinkedEntities>(e).unwrap().iter().copied().collect();

            // make sure that all the entities we are linking to actually have the RouteLink component, if not, add it
            for linked_e in route_linked_es {
                if world.get::<RouteLink>(linked_e).is_none() {
                    world.commands().entity(linked_e).insert(RouteLink(e));
                }
            }
        });
        hooks.on_remove(|mut world, e, _| {
            let route_linked_es = world.get::<RouteLinkedEntities>(e).unwrap().clone();
            let kmp_path_node = world.get::<KmpPathNode>(e).unwrap();

            // when we delete, try to move the route start forward to the next in the path
            if let Some(next_e) = kmp_path_node.next_nodes.iter().next().copied() {
                world.commands().add(move |world: &mut World| {
                    route_linked_es.move_route_start(world, e, next_e);
                });
            } else {
                // if there wasn't a next node to move ourselves to, then we'll have to delete all the route references
                for linked_e in route_linked_es.iter() {
                    world.commands().entity(*linked_e).remove::<RouteLink>();
                }
            }
        });
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, Deref, DerefMut)]
pub struct RouteLink(pub Entity);
impl Component for RouteLink {
    const STORAGE_TYPE: StorageType = StorageType::Table;
    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks.on_add(|mut world, e, _| {
            let linked_e = world.get::<RouteLink>(e).unwrap().0;
            let mut route_linked_es = world.get_mut::<RouteLinkedEntities>(linked_e).unwrap();
            // check that the we are included in the list of linked entities
            route_linked_es.insert(e);
        });
        hooks.on_remove(|mut world, e, _| {
            let linked_e = world.get::<RouteLink>(e).unwrap().0;
            let mut route_linked_es = world.get_mut::<RouteLinkedEntities>(linked_e).unwrap();
            // remove ourselves from the list of linked entities to the route
            route_linked_es.remove(&e);
        });
    }
}

pub fn spawn_route_section(commands: &mut Commands, kmp: Arc<KmpFile>, kmp_errors: &mut Vec<KmpError>) {
    for route in kmp.poti.entries.iter() {
        let mut prev_e = None;
        for route_pt in route.points.iter() {
            let e = Spawner::new(RoutePoint::from_kmp(route_pt, kmp_errors))
                .pos(route_pt.position)
                .visible(false)
                .prev_nodes(prev_e) // will add the prev entity if it exists
                .max_connected(1)
                .spawn_command(commands);

            // insert the route settings to the first route point
            if prev_e.is_none() {
                commands.entity(e).insert((
                    RouteSettings::from_kmp(route, kmp_errors),
                    RouteLinkedEntities::default(),
                ));
            }

            prev_e = Some(e);
        }
    }
}

pub fn update_routes(
    q_route_pts: Query<(Entity, Has<RouteLinkedEntities>, &KmpPathNode), With<RoutePoint>>,
    get_route_start: GetRouteStart,
    mut q_linked_entities: Query<&mut RouteLinkedEntities>,
    mut q_route_link: Query<&mut RouteLink>,
    mut commands: Commands,
) {
    for (e, is_route_start, kmp_path_node) in q_route_pts.iter() {
        // check if there are any entities linked to parts of the route that are not the start
        // if so, we need to link the entities to the start component
        if is_route_start && !kmp_path_node.prev_nodes.is_empty() {
            let route_start_e = get_route_start.get_entities(e);
            let linked_entities = q_linked_entities.get(e).unwrap().clone();

            // append all the linked entities of the route to the new route start
            let mut route_start_linked_entities = q_linked_entities.get_mut(route_start_e).unwrap();
            route_start_linked_entities.extend(linked_entities.iter());
            commands.entity(e).remove::<(RouteLinkedEntities, RouteSettings)>();

            // update all the route references to reference the new route start
            for linked_e in linked_entities.iter() {
                let mut route_link = q_route_link.get_mut(*linked_e).unwrap();
                **route_link = e;
            }
        }
    }
}

#[derive(SystemParam)]
pub struct GetRouteStart<'w, 's> {
    q: Query<'w, 's, (Entity, &'static KmpPathNode)>,
    q_route_settings: Query<'w, 's, (Entity, &'static mut RouteSettings)>,
}
impl GetRouteStart<'_, '_> {
    pub fn get_entities(&self, mut cur_e: Entity) -> Entity {
        while let Some(prev_e) = self.q.get(cur_e).ok().and_then(|x| x.1.prev_nodes.iter().next()) {
            cur_e = *prev_e;
        }
        cur_e
    }
    pub fn get_multiple_entities(&self, entities: impl IntoIterator<Item = Entity>) -> impl Iterator<Item = Entity> {
        let mut start_es = EntityHashSet::default();
        for e in entities {
            let start_e = self.get_entities(e);
            start_es.insert(start_e);
        }
        start_es.into_iter()
    }
    pub fn get_multiple_mut(
        &mut self,
        entities: impl IntoIterator<Item = Entity>,
    ) -> impl Iterator<Item = (Entity, Mut<RouteSettings>)> {
        let start_entities: EntityHashSet = self.get_multiple_entities(entities).collect();
        let mut route_settings = EntityHashMap::default();
        for (e, r) in self.q_route_settings.iter_mut() {
            if start_entities.contains(&e) {
                route_settings.insert(e, r);
            }
        }
        route_settings.into_iter()
    }
}
