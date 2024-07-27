use crate::{
    ui::viewport::ViewportInfo,
    util::{ui_viewport_to_ndc, RaycastFromCam},
    viewer::{
        camera::Gizmo2dCam,
        edit::select::{SelectSet, Selected},
    },
};

use super::{
    path::{KmpPathNode, RecalcPaths},
    FromKmp, KmpError, KmpFile, KmpSelectablePoint, RoutePoint, RouteSettings, Spawner,
};
use bevy::{
    ecs::{
        entity::{EntityHashMap, EntityHashSet},
        system::SystemParam,
    },
    prelude::*,
    utils::HashMap,
};
use bevy_mod_raycast::prelude::Raycast;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub fn routes_plugin(app: &mut App) {
    app.add_systems(Update, update_routes)
        .observe(on_add_route_linked_entities)
        .observe(on_remove_route_linked_entities)
        .observe(on_add_route_link)
        .observe(on_remove_route_link)
        .observe(on_add_route_pt)
        .observe(on_remove_route_pt)
        .add_systems(Update, update_route_selection_mode.after(SelectSet));
}

#[derive(Component, Default, Clone, Serialize, Deserialize, Debug, Deref, DerefMut)]
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

#[derive(Bundle, Default)]
pub struct RouteStartBundle {
    route_linked_entities: RouteLinkedEntities,
    route_settings: RouteSettings,
}

fn on_add_route_linked_entities(
    trigger: Trigger<OnAdd, RouteLinkedEntities>,
    q_route_linked_es: Query<&RouteLinkedEntities>,
    q_route_link: Query<&RouteLink>,
    mut commands: Commands,
) {
    let e = trigger.entity();
    let route_linked_es = q_route_linked_es.get(e).unwrap();

    // make sure that all the entities we are linking to actually have the RouteLink component, if not, add it
    for linked_e in route_linked_es.iter() {
        if q_route_link.get(*linked_e).is_err() {
            commands.entity(*linked_e).insert(RouteLink(e));
        }
    }
}
fn on_remove_route_linked_entities(
    trigger: Trigger<OnRemove, RouteLinkedEntities>,
    q_route_linked_es: Query<&RouteLinkedEntities>,
    q_kmp_path_node: Query<&KmpPathNode>,
    mut commands: Commands,
) {
    let e = trigger.entity();
    let route_linked_es = q_route_linked_es.get(e).unwrap().clone();
    let kmp_path_node = q_kmp_path_node.get(e).unwrap();

    // when we delete, try to move the route start forward to the next in the path
    if let Some(next_e) = kmp_path_node.next_nodes.iter().next().copied() {
        commands.add(move |world: &mut World| {
            route_linked_es.move_route_start(world, e, next_e);
        });
    } else {
        // if there wasn't a next node to move ourselves to, then we'll have to delete all the route references
        for linked_e in route_linked_es.iter() {
            commands.entity(*linked_e).remove::<RouteLink>();
        }
    }
}

#[derive(Component, Clone, Serialize, Deserialize, Debug, Deref, DerefMut)]
pub struct RouteLink(pub Entity);

fn on_add_route_link(
    trigger: Trigger<OnAdd, RouteLink>,
    q_route_link: Query<&RouteLink>,
    mut q_route_linked_es: Query<&mut RouteLinkedEntities>,
) {
    let e = trigger.entity();
    let linked_e = q_route_link.get(e).unwrap().0;

    let mut route_linked_es = q_route_linked_es.get_mut(linked_e).unwrap();
    // check that the we are included in the list of linked entities
    route_linked_es.insert(e);
}
fn on_remove_route_link(
    trigger: Trigger<OnAdd, RouteLink>,
    q_route_link: Query<&RouteLink>,

    mut q_route_linked_es: Query<&mut RouteLinkedEntities>,
) {
    let e = trigger.entity();
    let linked_e = q_route_link.get(e).unwrap().0;

    let mut route_linked_es = q_route_linked_es.get_mut(linked_e).unwrap();
    // remove ourselves from the list of linked entities to the route
    route_linked_es.remove(&e);
}

fn on_add_route_pt(trigger: Trigger<OnAdd, RoutePoint>, q_kmp_path_node: Query<&KmpPathNode>, mut commands: Commands) {
    let e = trigger.entity();
    let kmp_path_node = q_kmp_path_node.get(e).unwrap();

    // if we have started a new route path, add route settings and route linked entities to it because it is the first point
    if kmp_path_node.prev_nodes.is_empty() {
        commands.entity(e).insert(RouteStartBundle::default());
    }
}

fn on_remove_route_pt(
    trigger: Trigger<OnRemove, RoutePoint>,
    mut commands: Commands,
    q_kmp_path_node: Query<&KmpPathNode>,
    mut ev_recalc_paths: EventWriter<RecalcPaths>,
) {
    // we will have to add 'route settings' and 'route linked entities' components to the next entity,
    // because that entity is now the start of a new route now that we've been deleted
    let e = trigger.entity();
    // check if there is a next entity because we might be at the end of the route
    if let Some(new_start_e) = q_kmp_path_node.get(e).unwrap().next_nodes.iter().next() {
        commands.entity(*new_start_e).insert(RouteStartBundle::default());
        ev_recalc_paths.send(RecalcPaths::route());
    }
}

pub fn spawn_route_section(
    commands: &mut Commands,
    kmp: Arc<KmpFile>,
    kmp_errors: &mut Vec<KmpError>,
) -> HashMap<u8, Entity> {
    let mut id_entity_map = HashMap::default();
    for (i, route) in kmp.poti.entries.iter().enumerate() {
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
                commands.entity(e).insert(RouteStartBundle {
                    route_settings: RouteSettings::from_kmp(route, kmp_errors),
                    ..default()
                });
            }

            // if we are at the first route point
            if prev_e.is_none() {
                id_entity_map.insert(i as u8, e);
            }

            prev_e = Some(e);
        }
    }
    id_entity_map
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
        }

        // check if there are any entities linked to parts of the route that are not the start
        // if so, we need to link the entities to the start component
        if is_route_start && !kmp_path_node.prev_nodes.is_empty() {
            let linked_entities = q_linked_entities.get(e).unwrap().clone();

            // append all the linked entities of the route to the new route start
            let mut route_start_linked_entities = q_linked_entities.get_mut(route_start_e).unwrap();
            route_start_linked_entities.extend(linked_entities.iter());
            commands.entity(e).remove::<RouteStartBundle>();

            // update all the route references to reference the new route start
            for linked_e in linked_entities.iter() {
                let mut route_link = q_route_link.get_mut(*linked_e).unwrap();
                **route_link = e;
            }
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
        while let Some(prev_e) = self.q.get(cur_e).ok().and_then(|x| x.1.prev_nodes.iter().next()) {
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

#[derive(Resource)]
pub struct InRouteSelectionMode(pub Vec<Entity>);

fn update_route_selection_mode(
    res: Option<Res<InRouteSelectionMode>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut q_visibility: Query<(Entity, &mut Visibility)>,
    // saves the visibility state of everything before we went into route selection mode
    mut e_v_map: Local<HashMap<Entity, Visibility>>,
    mut commands: Commands,
    get_route_start: GetRouteStart,
    q_camera: Query<(&mut Camera, &GlobalTransform), Without<Gizmo2dCam>>,
    viewport_info: Res<ViewportInfo>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    q_window: Query<&Window>,
    q_route_pt: Query<Entity, With<RoutePoint>>,
    mut raycast: Raycast,
    q_non_route_pt: Query<Entity, (With<KmpSelectablePoint>, Without<RoutePoint>)>,
) {
    let Some(res) = res else { return };

    if res.is_added() {
        // we only just went into route selection mode so we need to set everything up
        for (e, v) in q_visibility.iter() {
            e_v_map.insert(e, *v);
        }
        for e in q_non_route_pt.iter() {
            *q_visibility.get_mut(e).unwrap().1 = Visibility::Hidden;
        }
        for e in q_route_pt.iter() {
            *q_visibility.get_mut(e).unwrap().1 = Visibility::Visible;
        }
    }

    let mut reset_visibilities = || {
        for (e, v) in e_v_map.iter() {
            let (_, mut v_mut) = q_visibility.get_mut(*e).unwrap();
            *v_mut = *v;
        }
    };

    if keys.just_pressed(KeyCode::Escape) {
        commands.remove_resource::<InRouteSelectionMode>();
        reset_visibilities();
    }

    if !mouse_buttons.just_pressed(MouseButton::Left) {
        return;
    }

    commands.remove_resource::<InRouteSelectionMode>();

    let Some(mouse_pos) = q_window.single().cursor_position() else {
        return;
    };

    // get the active camera
    let cam = q_camera.iter().find(|cam| cam.0.is_active).unwrap();

    let mouse_pos_ndc = ui_viewport_to_ndc(mouse_pos, viewport_info.viewport_rect);

    let intersections = RaycastFromCam::new(cam, mouse_pos_ndc, &mut raycast)
        .filter(&|e| q_route_pt.contains(e))
        .cast();
    let Some(intersection_e) = intersections.first().map(|x| x.0) else {
        return;
    };

    let route_start_e = get_route_start.get_entity(intersection_e);

    for to_be_linked_e in res.0.iter() {
        commands.entity(*to_be_linked_e).insert(RouteLink(route_start_e));
    }

    reset_visibilities();
}
