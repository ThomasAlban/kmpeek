use super::{
    checkpoints::CheckpointRight,
    meshes_materials::{CheckpointMaterials, KmpMeshes, PathMaterials},
    ordering::{NextOrderID, OrderId},
    Checkpoint, EnemyPathPoint, ItemPathPoint, KmpComponent, KmpSectionName, KmpSelectablePoint, PathGroup,
    PathOverallStart, RoutePoint, Section, Spawn, Spawner, TransformEditOptions,
};
use crate::{
    ui::settings::AppSettings,
    util::{
        kmp_file::{KmpFile, KmpGetPathSection, KmpGetSection, KmpPositionPoint},
        try_despawn,
    },
    viewer::{
        edit::{
            create_delete::DeleteSet,
            transform_gizmo::GizmoTransformable,
            tweak::{SnapTo, Tweakable},
        },
        normalize::Normalize,
    },
};
use bevy::{
    ecs::{
        entity::EntityHashMap,
        system::{SystemParam, SystemState},
    },
    prelude::*,
    utils::{HashMap, HashSet},
};
use bevy_mod_outline::{OutlineBundle, OutlineVolume};
use derive_new::new;
use std::marker::PhantomData;
use std::{any::TypeId, fmt::Debug};

pub fn path_plugin(app: &mut App) {
    app.add_event::<RecalcPaths>()
        .add_systems(
            Update,
            (
                update_node_links::<EnemyPathPoint>,
                update_node_links::<ItemPathPoint>,
                update_node_links::<Checkpoint>,
                update_node_links::<CheckpointRight>,
                update_node_links::<RoutePoint>,
                traverse_paths,
            )
                .after(DeleteSet),
        )
        .observe(on_add_kmp_path_node)
        .observe(on_remove_kmp_path_node);
}

// represents a link between 2 nodes
#[derive(Component)]
pub struct KmpPathNodeLink {
    pub prev_node: Entity,
    pub next_node: Entity,
    pub kind: PathType,
}

#[derive(PartialEq, Clone, Copy)]
pub enum PathType {
    Enemy,
    Item,
    Checkpoint { right: bool },
    Route,
}
pub trait ToPathType {
    fn to_path_type() -> PathType;
}
impl ToPathType for EnemyPathPoint {
    fn to_path_type() -> PathType {
        PathType::Enemy
    }
}
impl ToPathType for ItemPathPoint {
    fn to_path_type() -> PathType {
        PathType::Item
    }
}
impl ToPathType for Checkpoint {
    fn to_path_type() -> PathType {
        PathType::Checkpoint { right: false }
    }
}
impl ToPathType for CheckpointRight {
    fn to_path_type() -> PathType {
        PathType::Checkpoint { right: true }
    }
}
impl ToPathType for RoutePoint {
    fn to_path_type() -> PathType {
        PathType::Route
    }
}

// represents the line that links the 2 entities
#[derive(Component)]
pub struct KmpPathNodeLinkLine;

// component attached to kmp entities which are linked to other kmp entities
#[derive(Component, Clone, Debug, PartialEq, new)]
pub struct KmpPathNode {
    pub max: u8,
    #[new(default)]
    pub prev_nodes: HashSet<Entity>,
    #[new(default)]
    pub next_nodes: HashSet<Entity>,
}
impl Default for KmpPathNode {
    fn default() -> Self {
        Self {
            max: 6,
            prev_nodes: HashSet::with_capacity(6),
            next_nodes: HashSet::with_capacity(6),
        }
    }
}

impl KmpPathNode {
    #[allow(dead_code)]
    pub fn with_next(mut self, next: impl IntoIterator<Item = Entity>) -> Self {
        for next_e in next.into_iter() {
            self.next_nodes.insert(next_e);
        }
        self
    }
    pub fn with_prev(mut self, prev: impl IntoIterator<Item = Entity>) -> Self {
        for prev_e in prev.into_iter() {
            self.prev_nodes.insert(prev_e);
        }
        self
    }
    pub fn get_next(&self) -> HashSet<Entity> {
        self.next_nodes.clone()
    }
    pub fn get_previous(&self) -> HashSet<Entity> {
        self.prev_nodes.clone()
    }
    pub fn is_next_node_of(&self, self_e: Entity, other: &KmpPathNode, other_e: Entity) -> bool {
        if self.prev_nodes.contains(&other_e) || other.next_nodes.contains(&self_e) {
            return true;
        }
        false
    }
    pub fn is_prev_node_of(&self, self_e: Entity, other: &KmpPathNode, other_e: Entity) -> bool {
        if self.next_nodes.contains(&other_e) || other.prev_nodes.contains(&self_e) {
            return true;
        }
        false
    }
    pub fn is_linked_with(&self, self_e: Entity, other: &KmpPathNode, other_e: Entity) -> bool {
        self.is_next_node_of(self_e, other, other_e) || self.is_prev_node_of(self_e, other, other_e)
    }

    pub fn link_nodes(prev_node_e: Entity, next_e: Entity, world: &mut World) -> bool {
        if prev_node_e == next_e {
            return false;
        }
        // get next and prev nodes immutably first so we can check if they are linked
        let Some(next_node) = world.get::<KmpPathNode>(next_e) else {
            return false;
        };
        let Some(prev_node) = world.get::<KmpPathNode>(prev_node_e) else {
            return false;
        };
        if prev_node.is_linked_with(prev_node_e, next_node, next_e) {
            return false;
        }
        if next_node.prev_nodes.len() >= next_node.max as usize || prev_node.next_nodes.len() >= prev_node.max as usize
        {
            return false;
        }

        // now get them mutably one at a time to link them
        let mut next_node = world.get_mut::<KmpPathNode>(next_e).unwrap();
        next_node.prev_nodes.insert(prev_node_e);
        let mut prev_node = world.get_mut::<KmpPathNode>(prev_node_e).unwrap();
        prev_node.next_nodes.insert(next_e);

        true
    }
    pub fn unlink_nodes(prev_node_entity: Entity, next_node_entity: Entity, world: &mut World) -> bool {
        let Some(next_node) = world.get::<KmpPathNode>(next_node_entity) else {
            return false;
        };
        let Some(prev_node) = world.get::<KmpPathNode>(prev_node_entity) else {
            return false;
        };
        if !prev_node.is_linked_with(prev_node_entity, next_node, next_node_entity) {
            return false;
        }

        let mut next_node = world.get_mut::<KmpPathNode>(next_node_entity).unwrap();
        next_node.prev_nodes.remove(&prev_node_entity);
        let mut prev_node = world.get_mut::<KmpPathNode>(prev_node_entity).unwrap();
        prev_node.next_nodes.remove(&next_node_entity);

        true
    }
    // pub fn at_max_prev(&self) -> bool {
    //     self.prev_nodes.len() >= self.max.into()
    // }
    pub fn at_max_next(&self) -> bool {
        self.next_nodes.len() >= self.max.into()
    }
}

fn on_add_kmp_path_node(trigger: Trigger<OnAdd, KmpPathNode>, mut q_kmp_path_node: Query<&mut KmpPathNode>) {
    // on adding this component, ensure that the next/prev nodes also all hold references to the current node
    let e = trigger.entity();

    let cur_node = q_kmp_path_node.get(e).unwrap();

    let next_nodes = cur_node.get_next();
    let prev_nodes = cur_node.get_previous();

    for next_entity in next_nodes {
        let mut next_node = q_kmp_path_node.get_mut(next_entity).unwrap();
        next_node.prev_nodes.insert(e);
    }
    for prev_entity in prev_nodes {
        let mut prev_node = q_kmp_path_node.get_mut(prev_entity).unwrap();
        prev_node.next_nodes.insert(e);
    }
}

fn on_remove_kmp_path_node(
    trigger: Trigger<OnRemove, KmpPathNode>,
    mut q_kmp_path_node: Query<&mut KmpPathNode>,
    mut ev_recalc_paths: EventWriter<RecalcPaths>,
    q_is_enemy_path_pt: Query<(), With<EnemyPathPoint>>,
    q_is_item_path_pt: Query<(), With<ItemPathPoint>>,
    q_is_checkpoint: Query<(), With<Checkpoint>>,
) {
    let e = trigger.entity();

    let cur_node = q_kmp_path_node.get(e).unwrap();
    let next_nodes = cur_node.get_next();
    let prev_nodes = cur_node.get_previous();

    for next_entity in next_nodes {
        let mut next_node = q_kmp_path_node.get_mut(next_entity).unwrap();
        next_node.prev_nodes.remove(&e);
    }
    for prev_entity in prev_nodes {
        let mut prev_node = q_kmp_path_node.get_mut(prev_entity).unwrap();
        prev_node.next_nodes.remove(&e);
    }
    if q_is_enemy_path_pt.get(e).is_ok() {
        ev_recalc_paths.send(RecalcPaths::enemy());
    } else if q_is_item_path_pt.get(e).is_ok() {
        ev_recalc_paths.send(RecalcPaths::item());
    } else if q_is_checkpoint.get(e).is_ok() {
        // don't need to check for cp right as we'll be despawning that one anyway in the same swoop
        ev_recalc_paths.send(RecalcPaths::cp());
    }
}

#[derive(Clone, Debug)]
pub struct EntityGroup {
    pub entities: Vec<Entity>,
    pub next_groups: Vec<u8>,
}

pub struct KmpDataGroup<T> {
    pub nodes: Vec<T>,
    pub next_groups: Vec<u8>,
}

// pub fn is_enemy_point<T: 'static>() -> bool {
//     TypeId::of::<T>() == TypeId::of::<EnemyPathPoint>()
// }
// pub fn is_item_point<T: 'static>() -> bool {
//     TypeId::of::<T>() == TypeId::of::<ItemPathPoint>()
// }
pub fn is_checkpoint<T: 'static>() -> bool {
    TypeId::of::<T>() == TypeId::of::<Checkpoint>()
}
pub fn is_checkpoint_right<T: 'static>() -> bool {
    TypeId::of::<T>() == TypeId::of::<CheckpointRight>()
}
pub fn is_route_pt<T: 'static>() -> bool {
    TypeId::of::<T>() == TypeId::of::<RoutePoint>()
}
// pub fn is_path<T: 'static>() -> bool {
//     is_enemy_point::<T>() || is_item_point::<T>() || is_checkpoint::<T>()
// }

pub fn spawn_enemy_item_path_section<T: KmpComponent + Spawn>(world: &mut World, kmp: &KmpFile)
where
    T::KmpFormat: KmpGetSection + KmpGetPathSection + KmpPositionPoint,
    PathGroup<T::KmpFormat>: KmpSectionName,
{
    let kmp_groups = get_kmp_data_and_component_groups::<T>(kmp, world);

    let mut entity_groups: Vec<EntityGroup> = Vec::with_capacity(kmp_groups.len());
    let mut acc = 0;
    for (i, (data_group, component_group)) in kmp_groups.iter().enumerate() {
        let mut entity_group = EntityGroup {
            entities: Vec::with_capacity(data_group.nodes.len()),
            next_groups: data_group.next_groups.clone(),
        };
        for (j, node) in data_group.nodes.iter().enumerate() {
            let kmp_component = component_group[j].clone();

            let spawned_entity = Spawner::builder()
                .component(kmp_component)
                .pos(node.get_position())
                .visible(false)
                .order_id(acc)
                .build()
                .spawn(world);

            if i == 0 && j == 0 {
                world.entity_mut(spawned_entity).insert(PathOverallStart);
            }
            entity_group.entities.push(spawned_entity);
            acc += 1;
        }
        entity_groups.push(entity_group);
    }
    link_entity_groups(world, entity_groups);
}

pub fn spawn_path<T: Spawn + Component + Clone>(spawner: Spawner<T>, world: &mut World) -> Entity {
    let mesh = world.resource::<KmpMeshes>().sphere.clone();
    let material = world.resource::<PathMaterials<T>>().point.clone();
    let outline = world.get_resource::<AppSettings>().unwrap().kmp_model.outline;

    // either gets the order id, or gets it from the NextOrderID (which will increment it for next time)
    let order_id = spawner
        .order_id
        .unwrap_or_else(|| world.resource::<NextOrderID<T>>().get());

    let mut entity = match spawner.e {
        Some(e) => world.entity_mut(e),
        None => world.spawn_empty(),
    };
    entity.insert((
        PbrBundle {
            mesh,
            material,
            transform: spawner.get_transform(),
            visibility: if spawner.visible {
                Visibility::Visible
            } else {
                Visibility::Hidden
            },
            ..default()
        },
        KmpPathNode::new(spawner.max).with_prev(spawner.prev_nodes.clone().unwrap_or_default()),
        spawner.component.clone(),
        KmpSelectablePoint,
        Tweakable(SnapTo::Kcl),
        OrderId(order_id),
        TransformEditOptions::new(true, false),
        GizmoTransformable,
        Normalize::new(200., 30., BVec3::TRUE),
        OutlineBundle {
            outline: OutlineVolume {
                visible: false,
                colour: outline.color,
                width: outline.width,
            },
            ..default()
        },
    ));
    entity.id()
}

/// converts points and paths in the kmp to a list of groups containing the data, and components that have been converted from that data
pub fn get_kmp_data_and_component_groups<T: KmpComponent>(
    kmp: &KmpFile,
    world: &mut World,
) -> Vec<(KmpDataGroup<T::KmpFormat>, Vec<T>)>
where
    T::KmpFormat: KmpGetSection + KmpGetPathSection,
    PathGroup<T::KmpFormat>: KmpSectionName,
{
    let pathgroup_entries = &**T::KmpFormat::get_path_section(kmp);
    let node_entries = &**T::KmpFormat::get_section(kmp);

    let mut result: Vec<(KmpDataGroup<T::KmpFormat>, Vec<T>)> = Vec::with_capacity(pathgroup_entries.len());

    for group in pathgroup_entries.iter() {
        let mut next_groups = Vec::new();
        let mut kmp_component_group = Vec::new();
        let mut nodes = Vec::with_capacity(group.group_length.into());

        for next_group in group.next_group {
            if next_group != 0xff {
                next_groups.push(next_group);
            }
        }

        for i in group.start..(group.start + group.group_length) {
            let node = &node_entries[i as usize];
            nodes.push(node.clone());
            let kmp_component = T::from_kmp(node, world);
            kmp_component_group.push(kmp_component);
        }
        result.push((KmpDataGroup { nodes, next_groups }, kmp_component_group));
    }
    result
}
// go through a list of entity groups and link them together
pub fn link_entity_groups(world: &mut World, entity_groups: Vec<EntityGroup>) {
    // link the entities together
    for group in entity_groups.iter() {
        let mut prev_entity: Option<Entity> = None;
        // in each group, link the previous node to the current node
        for entity in group.entities.iter() {
            if let Some(prev_entity) = prev_entity {
                KmpPathNode::link_nodes(prev_entity, *entity, world);
            }
            prev_entity = Some(*entity);
        }
        // get the last entity of the current group
        let Some(entity) = prev_entity else { continue };
        // for each next group linked to the current group
        for next_group_index in group.next_groups.iter() {
            // get the first entity in the next group
            let next_entity = entity_groups[*next_group_index as usize].entities[0];
            // link the last entity in the current group with the first entity in the next group
            KmpPathNode::link_nodes(entity, next_entity, world);
        }
    }
}

fn spawn_node_link<T: Component + Clone + ToPathType>(
    world: &mut World,
    prev_node: Entity,
    next_node: Entity,
    visible: bool,
) -> Entity {
    let meshes = world.resource::<KmpMeshes>().clone();
    let (line, arrow) = if is_checkpoint::<T>() || is_checkpoint_right::<T>() {
        let materials = world.resource::<CheckpointMaterials>().clone();
        (materials.line, materials.arrow)
    } else {
        let materials = world.resource::<PathMaterials<T>>().clone();
        (materials.line, materials.arrow)
    };

    let prev_pos = world.get::<Transform>(prev_node).unwrap().translation;
    let next_pos = world.get::<Transform>(next_node).unwrap().translation;

    let mut parent_transform = Transform::from_translation(prev_pos.lerp(next_pos, 0.5)).looking_at(next_pos, Vec3::Y);
    parent_transform.rotate_local_x(f32::to_radians(-90.));

    let mut line_transform = Transform::default();
    line_transform.scale.y = prev_pos.distance(next_pos);

    // spawn a parent component which contains a transform, and stores the entities of the nodes the node links
    let e = world
        .spawn((
            SpatialBundle {
                transform: parent_transform,
                visibility: if visible {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                },
                ..default()
            },
            KmpPathNodeLink {
                prev_node,
                next_node,
                kind: T::to_path_type(),
            },
        ))
        // spawn the line and arrow as children of this parent component, which will inherit its transform & visibility
        .with_children(|parent| {
            parent.spawn((
                PbrBundle {
                    mesh: meshes.cylinder,
                    material: line,
                    transform: line_transform,
                    ..default()
                },
                // KmpSection,
                Normalize::new(200., 30., BVec3::new(true, false, true)),
                KmpPathNodeLinkLine,
            ));
            parent.spawn((
                PbrBundle {
                    mesh: meshes.frustrum,
                    material: arrow,
                    ..default()
                },
                // KmpSection,
                Normalize::new(200., 30., BVec3::TRUE),
            ));
        })
        .id();
    e
}

// TODO: make this more efficient by attaching link lines to the kmp points themselves
pub fn update_node_links<T: Component + Clone + ToPathType>(
    // mode: Option<Res<KmpEditMode<T>>>,
    // cp_mode: Option<Res<KmpEditMode<Checkpoint>>>,
    q_visibility: Query<&Visibility, Without<KmpPathNodeLink>>,
    mut q_kmp_node_link: Query<(Entity, &KmpPathNodeLink, &Children, &mut Visibility)>,
    q_kmp_node: Query<(Entity, &KmpPathNode), With<T>>,
    mut q_transform: Query<&mut Transform>,
    q_line: Query<&KmpPathNodeLinkLine>,
    mut commands: Commands,
) {
    // if mode.is_none() && !(is_checkpoint_right::<T>() && cp_mode.is_some()) {
    //     return;
    // }

    let mut nodes_to_be_linked: HashSet<(Entity, Entity)> = HashSet::new();
    for (cur_node, node_data) in q_kmp_node.iter() {
        for prev_node in node_data.prev_nodes.iter() {
            nodes_to_be_linked.insert((*prev_node, cur_node));
        }
        for next_node in node_data.next_nodes.iter() {
            nodes_to_be_linked.insert((cur_node, *next_node));
        }
    }

    // go through each node line
    for (link_entity, kmp_node_link, children, mut visibility) in q_kmp_node_link.iter_mut() {
        if !nodes_to_be_linked.contains(&(kmp_node_link.prev_node, kmp_node_link.next_node))
            && kmp_node_link.kind == T::to_path_type()
        {
            try_despawn(&mut commands, link_entity);
            continue;
        }
        nodes_to_be_linked.remove(&(kmp_node_link.prev_node, kmp_node_link.next_node));

        // update visibility of node link based on the linking nodes
        if let Ok([prev_visib, next_visib]) = q_visibility.get_many([kmp_node_link.prev_node, kmp_node_link.next_node])
        {
            *visibility = if prev_visib == Visibility::Visible && next_visib == Visibility::Visible {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }

        // don't bother unless the kmp node link is actually visible
        if *visibility == Visibility::Hidden {
            continue;
        }

        // see https://github.com/bevyengine/bevy/issues/11517
        let Ok(transforms) = q_transform.get_many_mut([kmp_node_link.prev_node, kmp_node_link.next_node]) else {
            try_despawn(&mut commands, link_entity);
            continue;
        };
        let [prev_transform, next_transform] = transforms.map(Ref::from);

        if !prev_transform.is_changed() && !next_transform.is_changed() {
            continue;
        }

        // get the positions of the previous and next nodes
        let prev_pos = prev_transform.translation;
        let next_pos = next_transform.translation;

        // calculate new transforms for the parent and the line
        let mut new_parent_transform =
            Transform::from_translation(prev_pos.lerp(next_pos, 0.5)).looking_at(next_pos, Vec3::Y);
        new_parent_transform.rotate_local_x(f32::to_radians(-90.));
        let mut new_line_transform = Transform::default();
        new_line_transform.scale.y = prev_pos.distance(next_pos);

        // set the transform of the parent
        let mut parent_transform = q_transform.get_mut(link_entity).unwrap();
        *parent_transform = new_parent_transform;

        // find the child of the kmp node link that has KmpNodeLinkLine, and set its transform
        if let Some(child) = children.iter().find(|x| q_line.get(**x).is_ok()) {
            let mut line_transform = q_transform.get_mut(*child).unwrap();
            *line_transform = new_line_transform;
        }
    }
    // spawn any links in that need to be spawned
    for node_not_linked in nodes_to_be_linked.iter() {
        let (prev_node, next_node) = *node_not_linked;
        commands.add(move |world: &mut World| {
            spawn_node_link::<T>(world, prev_node, next_node, true);
        });
    }
}

#[derive(Event, Default)]
pub struct RecalcPaths {
    pub do_enemy: bool,
    pub do_item: bool,
    pub do_cp: bool,
    pub do_route: bool,
}
impl RecalcPaths {
    pub fn enemy() -> Self {
        Self {
            do_enemy: true,
            ..default()
        }
    }
    pub fn item() -> Self {
        Self {
            do_item: true,
            ..default()
        }
    }
    pub fn cp() -> Self {
        Self {
            do_cp: true,
            ..default()
        }
    }
    pub fn route() -> Self {
        Self {
            do_route: true,
            ..default()
        }
    }
    pub fn all() -> Self {
        Self {
            do_enemy: true,
            do_item: true,
            do_cp: true,
            do_route: true,
        }
    }
}

pub fn traverse_paths(
    mut ev_recalc_paths: EventReader<RecalcPaths>,
    mut commands: Commands,
    mut p: ParamSet<(
        TraversePath<EnemyPathPoint>,
        TraversePath<ItemPathPoint>,
        TraversePath<Checkpoint>,
        TraversePath<RoutePoint>,
    )>,
) {
    for ev in ev_recalc_paths.read() {
        if ev.do_enemy {
            commands.insert_resource(p.p0().traverse());
        }
        if ev.do_item {
            commands.insert_resource(p.p1().traverse());
        }
        if ev.do_cp {
            commands.insert_resource(p.p2().traverse());
        }
        if ev.do_route {
            commands.insert_resource(p.p3().traverse());
        }
    }
}

#[derive(SystemParam)]
pub struct TraversePath<'w, 's, T: Component> {
    q_start: Query<'w, 's, Entity, (With<PathOverallStart>, With<T>, With<KmpPathNode>)>,
    q: Query<'w, 's, (Entity, &'static KmpPathNode), With<T>>,
}
impl<'w, 's, T: Component> TraversePath<'w, 's, T> {
    fn traverse(self) -> EntityPathGroups<T> {
        let mut paths: Vec<EntityPathGroup> = Vec::new();
        let mut node_to_path_index: HashMap<Entity, usize> = HashMap::default();
        let battle_mode = false;

        let is_battle_dispatcher =
            |node: &KmpPathNode| battle_mode && (node.next_nodes.len() + node.next_nodes.len() > 2);

        let mut nodes_to_handle: EntityHashMap<&KmpPathNode> = self.q.iter().collect();
        if nodes_to_handle.is_empty() {
            return EntityPathGroups::new(Vec::new());
        }
        let first = self
            .q_start
            .get_single()
            .ok()
            .and_then(|x| nodes_to_handle.remove(&x).map(|y| (x, y)));

        let mut first_iter = true;
        while !nodes_to_handle.is_empty() {
            let (node_e, node) = match first.filter(|_| first_iter) {
                Some(first) => first,
                None => nodes_to_handle.iter().next().map(|x| (*x.0, *x.1)).unwrap(),
            };
            first_iter = false;

            let mut path: Vec<Entity> = Vec::new();
            let path_index = paths.len();

            if is_battle_dispatcher(node) {
                path.push(node_e);
                paths.push(EntityPathGroup { path, ..default() });
                nodes_to_handle.remove(&node_e);
                node_to_path_index.insert(node_e, path_index);
                continue;
            }

            // traverse backwards until we find a node at the start
            let (mut start_node_e, mut start_node) = (node_e, node);
            // if we are not at the overall first node
            if !first.map(|x| x.0 == node_e).unwrap_or(false) {
                // while there is only one previous node, and it only has one next node, and it is not a battle dispatcher
                while let Some((prev_node_e, prev_node)) = (start_node.prev_nodes.len() == 1)
                    .then(|| self.q.get(*start_node.prev_nodes.iter().next().unwrap()).ok())
                    .flatten()
                {
                    if prev_node.next_nodes.len() != 1 || is_battle_dispatcher(prev_node) {
                        break;
                    }
                    if node_to_path_index.contains_key(&prev_node_e) {
                        break;
                    }
                    (start_node_e, start_node) = (prev_node_e, prev_node);
                    if start_node == node {
                        break;
                    }
                }
            }

            path.push(start_node_e);
            nodes_to_handle.remove(&start_node_e);
            node_to_path_index.insert(start_node_e, path_index);

            // traverse forwards through the path whose start we have now found
            #[allow(unused_assignments)]
            let (mut path_node_e, mut path_node) = (start_node_e, start_node);
            while let Some((next_node_e, next_node)) = (path_node.next_nodes.len() == 1)
                .then(|| self.q.get(*path_node.next_nodes.iter().next().unwrap()).ok())
                .flatten()
                .filter(|x| nodes_to_handle.contains_key(&x.0))
            {
                if next_node.prev_nodes.len() != 1 || is_battle_dispatcher(next_node) {
                    break;
                }
                (path_node_e, path_node) = (next_node_e, next_node);

                path.push(path_node_e);
                nodes_to_handle.remove(&path_node_e);
                node_to_path_index.insert(next_node_e, path_index);
            }
            paths.push(EntityPathGroup { path, ..default() });
        }

        for i in 0..paths.len() {
            let Some(last) = paths[i].path.last() else {
                continue;
            };
            for next in self.q.get(*last).unwrap().1.next_nodes.iter() {
                let Some(next_i) = node_to_path_index.get(next) else {
                    continue;
                };
                paths[i].next_paths.push(*next_i);
                paths[*next_i].prev_paths.push(i);
            }
        }

        EntityPathGroups::new(paths)
    }
}

#[derive(Clone, Debug, Default)]
pub struct EntityPathGroup {
    pub path: Vec<Entity>,
    pub prev_paths: Vec<usize>,
    pub next_paths: Vec<usize>,
}

#[derive(Resource, Clone, new, Deref, DerefMut)]
pub struct EntityPathGroups<T: Component>(#[deref] pub Vec<EntityPathGroup>, PhantomData<T>);

pub fn save_path_section<T: KmpComponent>(
    world: &mut World,
) -> (Section<T::KmpFormat>, Section<PathGroup<T::KmpFormat>>)
where
    PathGroup<T::KmpFormat>: KmpSectionName,
{
    let mut ss = SystemState::<TraversePath<T>>::new(world);
    let traverse_path = ss.get_mut(world);
    traverse_path.traverse();
    ss.apply(world);

    let mut points = Vec::new();
    let mut paths = Vec::new();

    let entity_paths = world.resource::<EntityPathGroups<T>>().clone();
    for entity_path in entity_paths.iter() {
        let start = points.len() as u8;
        let group_length = entity_path.path.len() as u8;

        let mut prev_group = [0xffu8; 6];
        for (i, index) in entity_path.prev_paths.iter().enumerate() {
            prev_group[i] = *index as u8;
        }
        let mut next_group = [0xffu8; 6];
        for (i, index) in entity_path.next_paths.iter().enumerate() {
            next_group[i] = *index as u8;
        }

        for e in entity_path.path.iter() {
            let transform = world.entity(*e).get::<Transform>().unwrap();
            let pt = world
                .entity(*e)
                .get::<T>()
                .unwrap()
                .clone()
                .to_kmp(*transform, world, *e);
            points.push(pt);
        }
        paths.push(PathGroup::new(start, group_length, prev_group, next_group, 0));
    }

    (Section::new(points), Section::new(paths))
}
