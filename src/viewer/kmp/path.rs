#[cfg(test)]
use super::Section;
use super::{
    checkpoints::CheckpointRight,
    meshes_materials::{CheckpointMaterials, KmpMeshes, PathMaterials},
    ordering::{NextOrderID, OrderId},
    Checkpoint, EnemyPathPoint, ItemPathPoint, KmpComponent, KmpSectionName, KmpSelectablePoint, PathGroup,
    PathOverallStart, RoutePoint, RouteSettings, Spawn, Spawner, TransformEditOptions,
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
#[cfg(test)]
use bevy::ecs::system::SystemState;
use bevy::{
    ecs::{entity::EntityHashMap, system::SystemParam},
    platform::collections::{HashMap, HashSet},
    prelude::*,
    transform::TransformSystems,
};
use bevy_mod_outline::OutlineVolume;
use derive_new::new;
use std::marker::PhantomData;
use std::{any::TypeId, fmt::Debug};

pub fn path_plugin(app: &mut App) {
    app.add_message::<RecalcPaths>()
        .add_systems(
            Update,
            (
                traverse_paths,
                reconcile_node_links::<EnemyPathPoint>,
                reconcile_node_links::<ItemPathPoint>,
                reconcile_node_links::<Checkpoint>,
                reconcile_node_links::<CheckpointRight>,
                reconcile_node_links::<RoutePoint>,
                update_path_start_colors::<EnemyPathPoint>,
                update_path_start_colors::<ItemPathPoint>,
            )
                .after(DeleteSet),
        )
        .add_systems(
            PostUpdate,
            (
                update_node_links::<EnemyPathPoint>,
                update_node_links::<ItemPathPoint>,
                update_node_links::<Checkpoint>,
                update_node_links::<CheckpointRight>,
                update_node_links::<RoutePoint>,
            )
                .before(TransformSystems::Propagate),
        )
        .add_observer(on_add_kmp_path_node)
        .add_observer(on_remove_kmp_path_node);
}

fn update_path_start_colors<T: Component + Clone>(
    materials: Option<Res<PathMaterials<T>>>,
    mut q_points: Query<(Has<PathOverallStart>, &mut MeshMaterial3d<StandardMaterial>), With<T>>,
) {
    let Some(materials) = materials else {
        return;
    };
    for (is_start, mut material) in q_points.iter_mut() {
        let expected = if is_start {
            materials.start_point.clone()
        } else {
            materials.point.clone()
        };
        material.set_if_neq(MeshMaterial3d(expected));
    }
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

    fn links_are_valid(entity: Entity, node: &KmpPathNode, world: &World) -> bool {
        if node.prev_nodes.len() > node.max as usize || node.next_nodes.len() > node.max as usize {
            return false;
        }
        node.next_nodes.iter().all(|next| {
            world
                .get::<KmpPathNode>(*next)
                .is_some_and(|next_node| next_node.prev_nodes.contains(&entity))
        }) && node.prev_nodes.iter().all(|previous| {
            world
                .get::<KmpPathNode>(*previous)
                .is_some_and(|previous_node| previous_node.next_nodes.contains(&entity))
        })
    }

    fn reaches_target(
        current: Entity,
        target: Entity,
        world: &World,
        visiting: &mut HashSet<Entity>,
        visited: &mut HashSet<Entity>,
    ) -> Option<bool> {
        if current == target {
            return Some(true);
        }
        if visited.contains(&current) {
            return Some(false);
        }
        // Encountering a node already on the current DFS stack means the
        // reachable graph already contains a cycle.
        if !visiting.insert(current) {
            return None;
        }

        let node = world.get::<KmpPathNode>(current)?;
        if !Self::links_are_valid(current, node, world) {
            return None;
        }

        for next in &node.next_nodes {
            match Self::reaches_target(*next, target, world, visiting, visited) {
                Some(true) => return Some(true),
                Some(false) => {}
                None => return None,
            }
        }

        visiting.remove(&current);
        visited.insert(current);
        Some(false)
    }

    pub fn can_link_nodes(prev_node_e: Entity, next_e: Entity, world: &World) -> bool {
        if prev_node_e == next_e {
            return false;
        }
        let Some(prev_node) = world.get::<KmpPathNode>(prev_node_e) else {
            return false;
        };
        let Some(next_node) = world.get::<KmpPathNode>(next_e) else {
            return false;
        };
        if !Self::links_are_valid(prev_node_e, prev_node, world)
            || !Self::links_are_valid(next_e, next_node, world)
            || prev_node.is_linked_with(prev_node_e, next_node, next_e)
            || prev_node.next_nodes.len() >= prev_node.max as usize
            || next_node.prev_nodes.len() >= next_node.max as usize
        {
            return false;
        }

        match Self::reaches_target(
            next_e,
            prev_node_e,
            world,
            &mut HashSet::default(),
            &mut HashSet::default(),
        ) {
            // No cycle is created.
            Some(false) => true,
            // A cycle is valid only when a terminal node closes the component
            // back to its canonical course/path start.
            Some(true) => {
                prev_node.next_nodes.is_empty()
                    && (world.get::<PathOverallStart>(next_e).is_some() || world.get::<RouteSettings>(next_e).is_some())
            }
            // Reject stale, asymmetric, or already-cyclic reachable graphs.
            None => false,
        }
    }

    pub fn link_nodes(prev_node_e: Entity, next_e: Entity, world: &mut World) -> bool {
        if !Self::can_link_nodes(prev_node_e, next_e, world) {
            return false;
        }

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

fn on_add_kmp_path_node(trigger: On<Add, KmpPathNode>, mut q_kmp_path_node: Query<&mut KmpPathNode>) {
    // on adding this component, ensure that the next/prev nodes also all hold references to the current node
    let e = trigger.event().entity;

    let Ok(cur_node) = q_kmp_path_node.get(e) else {
        return;
    };

    let next_nodes = cur_node.get_next();
    let prev_nodes = cur_node.get_previous();
    let mut stale_next_nodes = Vec::new();
    let mut stale_prev_nodes = Vec::new();

    for next_entity in next_nodes {
        if let Ok(mut next_node) = q_kmp_path_node.get_mut(next_entity) {
            next_node.prev_nodes.insert(e);
        } else {
            stale_next_nodes.push(next_entity);
        }
    }
    for prev_entity in prev_nodes {
        if let Ok(mut prev_node) = q_kmp_path_node.get_mut(prev_entity) {
            prev_node.next_nodes.insert(e);
        } else {
            stale_prev_nodes.push(prev_entity);
        }
    }

    if let Ok(mut cur_node) = q_kmp_path_node.get_mut(e) {
        cur_node.next_nodes.retain(|next| !stale_next_nodes.contains(next));
        cur_node.prev_nodes.retain(|prev| !stale_prev_nodes.contains(prev));
    }
}

fn on_remove_kmp_path_node(
    trigger: On<Remove, KmpPathNode>,
    mut q_kmp_path_node: Query<&mut KmpPathNode>,
    mut ev_recalc_paths: MessageWriter<RecalcPaths>,
    q_is_enemy_path_pt: Query<(), With<EnemyPathPoint>>,
    q_is_item_path_pt: Query<(), With<ItemPathPoint>>,
    q_is_checkpoint: Query<(), With<Checkpoint>>,
) {
    let e = trigger.event().entity;

    let Ok(cur_node) = q_kmp_path_node.get(e) else {
        return;
    };
    let next_nodes = cur_node.get_next();
    let prev_nodes = cur_node.get_previous();

    for next_entity in next_nodes {
        if let Ok(mut next_node) = q_kmp_path_node.get_mut(next_entity) {
            next_node.prev_nodes.remove(&e);
        }
    }
    for prev_entity in prev_nodes {
        if let Ok(mut prev_node) = q_kmp_path_node.get_mut(prev_entity) {
            prev_node.next_nodes.remove(&e);
        }
    }
    if q_is_enemy_path_pt.get(e).is_ok() {
        ev_recalc_paths.write(RecalcPaths::enemy());
    } else if q_is_item_path_pt.get(e).is_ok() {
        ev_recalc_paths.write(RecalcPaths::item());
    } else if q_is_checkpoint.get(e).is_ok() {
        // don't need to check for cp right as we'll be despawning that one anyway in the same swoop
        ev_recalc_paths.write(RecalcPaths::cp());
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
        Mesh3d(mesh),
        MeshMaterial3d(material),
        spawner.get_transform(),
        if spawner.visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        },
        KmpPathNode::new(spawner.max).with_prev(spawner.prev_nodes.clone().unwrap_or_default()),
        spawner.component.clone(),
        KmpSelectablePoint,
        Tweakable(SnapTo::Kcl),
        OrderId(order_id),
        TransformEditOptions::new(true, false),
        GizmoTransformable,
        Normalize::new(200., 30., BVec3::TRUE),
        OutlineVolume {
            visible: false,
            colour: outline.color,
            width: outline.width,
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

        // Widen before addition: valid ranges can cross the u8 boundary.
        let start = usize::from(group.start);
        let end = start + usize::from(group.group_length);
        let Some(entries) = node_entries.get(start..end) else {
            warn!("Skipping invalid path group range {start}..{end}");
            // Keep the group slot so subsequent group indices remain valid.
            result.push((KmpDataGroup { nodes, next_groups }, kmp_component_group));
            continue;
        };
        for node in entries {
            nodes.push(node.clone());
            let kmp_component = T::from_kmp(node, world);
            kmp_component_group.push(kmp_component);
        }
        result.push((KmpDataGroup { nodes, next_groups }, kmp_component_group));
    }
    result
}
// Imported topology is not an interactive edit: reverse edges, self-links and
// cycles are legal. Validate both endpoints before mutating either, and retain
// reciprocal links even when imported in-degree exceeds the interactive cap.
fn link_imported_nodes(world: &mut World, previous: Entity, next: Entity) -> bool {
    if world.get::<KmpPathNode>(previous).is_none() || world.get::<KmpPathNode>(next).is_none() {
        return false;
    }
    world.get_mut::<KmpPathNode>(previous).unwrap().next_nodes.insert(next);
    world.get_mut::<KmpPathNode>(next).unwrap().prev_nodes.insert(previous);
    true
}

// go through a list of entity groups and link them together
pub fn link_entity_groups(world: &mut World, entity_groups: Vec<EntityGroup>) {
    // link the entities together
    for group in entity_groups.iter() {
        let mut prev_entity: Option<Entity> = None;
        // in each group, link the previous node to the current node
        for entity in group.entities.iter() {
            if let Some(prev_entity) = prev_entity {
                link_imported_nodes(world, prev_entity, *entity);
            }
            prev_entity = Some(*entity);
        }
        // get the last entity of the current group
        let Some(entity) = prev_entity else { continue };
        // for each next group linked to the current group
        for next_group_index in group.next_groups.iter() {
            // get the first entity in the next group
            let Some(next_entity) = entity_groups
                .get(usize::from(*next_group_index))
                .and_then(|group| group.entities.first())
            else {
                warn!("Skipping link to missing or empty path group {next_group_index}");
                continue;
            };
            // link the last entity in the current group with the first entity in the next group
            link_imported_nodes(world, entity, *next_entity);
        }
    }
}

fn path_link_transforms(prev_pos: Vec3, next_pos: Vec3) -> (Transform, Transform) {
    let distance = prev_pos.distance(next_pos);
    let mut parent_transform = Transform::from_translation(prev_pos.lerp(next_pos, 0.5));
    if distance > f32::EPSILON {
        parent_transform.look_at(next_pos, Vec3::Y);
        parent_transform.rotate_local_x(f32::to_radians(-90.));
    }

    let mut line_transform = Transform::default();
    line_transform.scale.y = distance;
    (parent_transform, line_transform)
}

fn spawn_node_link<T: Component + Clone + ToPathType>(
    world: &mut World,
    prev_node: Entity,
    next_node: Entity,
) -> Option<Entity> {
    let prev_pos = world.get::<Transform>(prev_node)?.translation;
    let next_pos = world.get::<Transform>(next_node)?.translation;

    let meshes = world.resource::<KmpMeshes>().clone();
    let (line, arrow) = if is_checkpoint::<T>() || is_checkpoint_right::<T>() {
        let materials = world.resource::<CheckpointMaterials>().clone();
        (materials.line, materials.arrow)
    } else {
        let materials = world.resource::<PathMaterials<T>>().clone();
        (materials.line, materials.arrow)
    };

    let (parent_transform, line_transform) = path_link_transforms(prev_pos, next_pos);
    let visibility = if world.get::<Visibility>(prev_node) == Some(&Visibility::Visible)
        && world.get::<Visibility>(next_node) == Some(&Visibility::Visible)
    {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };

    // spawn a parent component which contains a transform, and stores the entities of the nodes the node links
    let e = world
        .spawn((
            parent_transform,
            visibility,
            KmpPathNodeLink {
                prev_node,
                next_node,
                kind: T::to_path_type(),
            },
        ))
        // spawn the line and arrow as children of this parent component, which will inherit its transform & visibility
        .with_children(|parent| {
            parent.spawn((
                Mesh3d(meshes.cylinder),
                MeshMaterial3d(line),
                line_transform,
                // KmpSection,
                Normalize::new(200., 30., BVec3::new(true, false, true)),
                KmpPathNodeLinkLine,
            ));
            parent.spawn((
                Mesh3d(meshes.frustrum),
                MeshMaterial3d(arrow),
                // KmpSection,
                Normalize::new(200., 30., BVec3::TRUE),
            ));
        })
        .id();
    Some(e)
}

// Reconcile link entities during Update, before Bevy's PostUpdate render bookkeeping.
fn reconcile_node_links<T: Component + Clone + ToPathType>(
    q_kmp_node: Query<(Entity, &KmpPathNode), With<T>>,
    q_kmp_node_link: Query<(Entity, &KmpPathNodeLink)>,
    mut commands: Commands,
) {
    let mut nodes_to_be_linked: HashSet<(Entity, Entity)> = HashSet::new();
    for (cur_node, node_data) in &q_kmp_node {
        for prev_node in &node_data.prev_nodes {
            nodes_to_be_linked.insert((*prev_node, cur_node));
        }
        for next_node in &node_data.next_nodes {
            nodes_to_be_linked.insert((cur_node, *next_node));
        }
    }

    for (link_entity, link) in &q_kmp_node_link {
        if link.kind != T::to_path_type() {
            continue;
        }

        if !nodes_to_be_linked.remove(&(link.prev_node, link.next_node)) {
            try_despawn(&mut commands, link_entity);
        }
    }

    for (prev_node, next_node) in nodes_to_be_linked {
        commands.queue(move |world: &mut World| {
            spawn_node_link::<T>(world, prev_node, next_node);
        });
    }
}

// Update existing link geometry after point movement, but do not create render entities this late.
pub fn update_node_links<T: Component + Clone + ToPathType>(
    mut q_kmp_node_link: Query<(Entity, &KmpPathNodeLink, &Children, &mut Visibility, &mut Transform)>,
    q_node: Query<(Ref<Transform>, &Visibility), (Without<KmpPathNodeLink>, Without<KmpPathNodeLinkLine>)>,
    mut q_line: Query<&mut Transform, (With<KmpPathNodeLinkLine>, Without<KmpPathNodeLink>)>,
    mut commands: Commands,
) {
    for (link_entity, link, children, mut visibility, mut parent_transform) in q_kmp_node_link.iter_mut() {
        if link.kind != T::to_path_type() {
            continue;
        }

        let Ok([(prev_transform, prev_visibility), (next_transform, next_visibility)]) =
            q_node.get_many([link.prev_node, link.next_node])
        else {
            *visibility = Visibility::Hidden;
            try_despawn(&mut commands, link_entity);
            continue;
        };

        *visibility = if prev_visibility == Visibility::Visible && next_visibility == Visibility::Visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };

        // Keep hidden links up to date too. Otherwise endpoint change ticks can
        // expire while hidden, leaving stale geometry when the link is shown again.
        if !prev_transform.is_changed() && !next_transform.is_changed() {
            continue;
        }

        let (new_parent_transform, new_line_transform) =
            path_link_transforms(prev_transform.translation, next_transform.translation);
        *parent_transform = new_parent_transform;

        if let Some(child) = children.iter().find(|child| q_line.contains(*child)) {
            if let Ok(mut line_transform) = q_line.get_mut(child) {
                *line_transform = new_line_transform;
            }
        }
    }
}

#[derive(Message)]
pub struct RecalcPaths {
    pub do_enemy: bool,
    pub do_item: bool,
    pub do_cp: bool,
    pub do_route: bool,
}
impl RecalcPaths {
    pub fn for_path_type(path_type: PathType) -> Self {
        match path_type {
            PathType::Enemy => Self::enemy(),
            PathType::Item => Self::item(),
            PathType::Checkpoint { .. } => Self::cp(),
            PathType::Route => Self::route(),
        }
    }

    pub fn enemy() -> Self {
        Self {
            do_enemy: true,
            do_item: false,
            do_cp: false,
            do_route: false,
        }
    }
    pub fn item() -> Self {
        Self {
            do_enemy: false,
            do_item: true,
            do_cp: false,
            do_route: false,
        }
    }
    pub fn cp() -> Self {
        Self {
            do_enemy: false,
            do_item: false,
            do_cp: true,
            do_route: false,
        }
    }
    pub fn route() -> Self {
        Self {
            do_enemy: false,
            do_item: false,
            do_cp: false,
            do_route: true,
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
    mut ev_recalc_paths: MessageReader<RecalcPaths>,
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
    q_order: Query<'w, 's, &'static OrderId, With<T>>,
}
impl<'w, 's, T: Component> TraversePath<'w, 's, T> {
    fn traverse(self) -> EntityPathGroups<T> {
        let mut paths: Vec<EntityPathGroup> = Vec::new();
        let mut node_to_path_index: HashMap<Entity, usize> = HashMap::default();
        let battle_mode = false;

        let is_battle_dispatcher =
            |node: &KmpPathNode| battle_mode && (node.prev_nodes.len() + node.next_nodes.len() > 2);

        let mut nodes_to_handle: EntityHashMap<&KmpPathNode> = self.q.iter().collect();
        if nodes_to_handle.is_empty() {
            return EntityPathGroups::new(Vec::new());
        }
        // OrderId is the stable editor order. Entity breaks ties for missing or
        // duplicate IDs without relying on randomized collection iteration.
        let mut ordered_nodes: Vec<_> = nodes_to_handle.keys().copied().collect();
        ordered_nodes.sort_by_key(|e| (self.q_order.get(*e).map_or(u32::MAX, |id| id.0), e.to_bits()));
        let first = ordered_nodes
            .iter()
            .copied()
            .find(|e| self.q_start.contains(*e))
            .map(|e| (e, nodes_to_handle[&e]));

        let mut first_iter = true;
        while !nodes_to_handle.is_empty() {
            let (node_e, node) = match first.filter(|_| first_iter) {
                Some(first) => first,
                None => {
                    let e = *ordered_nodes.iter().find(|e| nodes_to_handle.contains_key(*e)).unwrap();
                    (e, nodes_to_handle[&e])
                }
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
                let mut visited: HashSet<Entity> = HashSet::from_iter([node_e]);
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
                    if !visited.insert(prev_node_e) {
                        // An unanchored cycle starts at its lowest ordered seed.
                        (start_node_e, start_node) = (node_e, node);
                        break;
                    }
                    (start_node_e, start_node) = (prev_node_e, prev_node);
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

        for path in &mut paths {
            path.prev_paths.sort_unstable();
            path.prev_paths.dedup();
            path.next_paths.sort_unstable();
            path.next_paths.dedup();
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

// Kept only for traversal regression tests. Application saving must use the
// loaded-document patcher: graph regeneration cannot preserve source metadata.
#[cfg(test)]
pub fn save_path_section<T: KmpComponent>(
    world: &mut World,
) -> (Section<T::KmpFormat>, Section<PathGroup<T::KmpFormat>>)
where
    PathGroup<T::KmpFormat>: KmpSectionName,
{
    let entity_paths = {
        let mut ss = SystemState::<TraversePath<T>>::new(world);
        let entity_paths = ss
            .get_mut(world)
            .expect("path traversal system state should be valid")
            .traverse();
        ss.apply(world);
        entity_paths
    };

    let mut points = Vec::new();
    let mut paths = Vec::new();

    assert!(
        entity_paths.len() <= 255,
        "path group indices must not use the 0xff sentinel"
    );
    for entity_path in entity_paths.iter() {
        let start = u8::try_from(points.len()).expect("path group start exceeds u8 index range");
        let group_length = u8::try_from(entity_path.path.len()).expect("path group length exceeds u8 range");
        assert!(
            entity_path.prev_paths.len() <= 6 && entity_path.next_paths.len() <= 6,
            "path group has more than six links"
        );

        let mut prev_group = [0xffu8; 6];
        for (i, index) in entity_path.prev_paths.iter().enumerate() {
            prev_group[i] = *index as u8;
        }
        let mut next_group = [0xffu8; 6];
        for (i, index) in entity_path.next_paths.iter().enumerate() {
            next_group[i] = *index as u8;
        }

        for (offset, e) in entity_path.path.iter().enumerate() {
            let transform = world.entity(*e).get::<Transform>().unwrap();
            let mut pt = world
                .entity(*e)
                .get::<T>()
                .unwrap()
                .clone()
                .to_kmp(*transform, world, *e);
            // CKPT links describe adjacency within the serialized CKPH group,
            // not editor OrderIds. Keep this format-specific correction local
            // rather than temporarily mutating the world's ordering components.
            if let Some(cp) = (&mut pt as &mut dyn std::any::Any).downcast_mut::<crate::util::kmp_file::Ckpt>() {
                let (previous, next) =
                    checkpoint_serialized_neighbors(usize::from(start), offset, entity_path.path.len());
                cp.prev_cp = previous;
                cp.next_cp = next;
            }
            points.push(pt);
        }
        paths.push(PathGroup::new(start, group_length, prev_group, next_group, 0));
    }

    (Section::new(points), Section::new(paths))
}

#[cfg(test)]
fn checkpoint_serialized_neighbors(start: usize, offset: usize, length: usize) -> (u8, u8) {
    let index = |index| u8::try_from(index).ok().filter(|index| *index != 0xff).unwrap_or(0xff);
    (
        if offset > 0 { index(start + offset - 1) } else { 0xff },
        if offset + 1 < length {
            index(start + offset + 1)
        } else {
            0xff
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Component)]
    struct TestPathPoint;

    fn spawn_node(world: &mut World, overall_start: bool) -> Entity {
        let mut entity = world.spawn((TestPathPoint, KmpPathNode::default()));
        if overall_start {
            entity.insert(PathOverallStart);
        }
        entity.id()
    }

    fn traverse(world: &mut World) -> EntityPathGroups<TestPathPoint> {
        let mut state = SystemState::<TraversePath<TestPathPoint>>::new(world);
        let paths = state
            .get_mut(world)
            .expect("path traversal system state should be valid")
            .traverse();
        state.apply(world);
        paths
    }

    fn path_containing(paths: &EntityPathGroups<TestPathPoint>, entity: Entity) -> usize {
        paths
            .iter()
            .position(|path| path.path.contains(&entity))
            .expect("entity should belong to a traversed path")
    }

    #[test]
    fn import_ranges_widen_before_addition_and_keep_invalid_slots() {
        use crate::util::kmp_file::Enpt;
        let mut world = World::new();
        let kmp = KmpFile {
            enpt: Section::new(
                (0..260)
                    .map(|i| Enpt {
                        leniency: i as f32,
                        ..default()
                    })
                    .collect(),
            ),
            enph: Section::new(vec![
                PathGroup::new(250, 10, [0xff; 6], [1, 0xff, 0xff, 0xff, 0xff, 0xff], 0),
                PathGroup::new(255, 10, [0xff; 6], [0xff; 6], 0),
                PathGroup::new(0, 1, [0xff; 6], [0xff; 6], 0),
            ]),
            ..default()
        };
        let groups = get_kmp_data_and_component_groups::<EnemyPathPoint>(&kmp, &mut world);
        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].0.nodes.len(), 10);
        assert_eq!(groups[0].0.nodes[9].leniency, 259.0);
        assert!(groups[1].0.nodes.is_empty());
        assert_eq!(groups[2].0.nodes.len(), 1);
    }

    #[test]
    fn imported_edges_allow_reverse_self_cycles_and_ignore_invalid_targets() {
        let mut world = World::new();
        let a = spawn_node(&mut world, false);
        let b = spawn_node(&mut world, false);
        let c = spawn_node(&mut world, false);
        link_entity_groups(
            &mut world,
            vec![
                EntityGroup {
                    entities: vec![a],
                    next_groups: vec![0, 1, 1, 3, 99],
                },
                EntityGroup {
                    entities: vec![b],
                    next_groups: vec![0, 2],
                },
                EntityGroup {
                    entities: vec![c],
                    next_groups: vec![0],
                },
                EntityGroup {
                    entities: vec![],
                    next_groups: vec![],
                },
            ],
        );
        for (previous, next) in [(a, a), (a, b), (b, a), (b, c), (c, a)] {
            assert!(world.get::<KmpPathNode>(previous).unwrap().next_nodes.contains(&next));
            assert!(world.get::<KmpPathNode>(next).unwrap().prev_nodes.contains(&previous));
        }
        assert_eq!(world.get::<KmpPathNode>(a).unwrap().next_nodes.len(), 2);
        let stale = world.spawn_empty().id();
        world.despawn(stale);
        let before = world.get::<KmpPathNode>(a).unwrap().clone();
        assert!(!link_imported_nodes(&mut world, a, stale));
        assert_eq!(*world.get::<KmpPathNode>(a).unwrap(), before);
        assert!(!KmpPathNode::can_link_nodes(a, a, &world));
        let paths = traverse(&mut world);
        assert_eq!(paths.iter().map(|p| p.path.len()).sum::<usize>(), 3);
    }

    #[test]
    fn traverse_singleton_including_self_link() {
        let mut world = World::new();
        let a = spawn_node(&mut world, true);
        let paths = traverse(&mut world);
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].path, vec![a]);
        link_imported_nodes(&mut world, a, a);
        let paths = traverse(&mut world);
        assert_eq!(paths[0].path, vec![a]);
        assert_eq!(paths[0].prev_paths, vec![0]);
        assert_eq!(paths[0].next_paths, vec![0]);
    }

    #[test]
    fn unanchored_cycle_starts_at_lowest_order_id() {
        let mut world = World::new();
        let a = spawn_node(&mut world, false);
        let b = spawn_node(&mut world, false);
        let c = spawn_node(&mut world, false);
        world.entity_mut(a).insert(OrderId(30));
        world.entity_mut(b).insert(OrderId(10));
        world.entity_mut(c).insert(OrderId(20));
        for (previous, next) in [(a, b), (b, c), (c, a)] {
            link_imported_nodes(&mut world, previous, next);
        }
        let paths = traverse(&mut world);
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].path, vec![b, c, a]);
        assert_eq!(paths[0].next_paths, vec![0]);
    }

    #[test]
    fn singleton_exports_one_point_and_one_group() {
        let mut world = World::new();
        world.spawn((
            EnemyPathPoint::default(),
            KmpPathNode::default(),
            Transform::default(),
            OrderId(42),
            PathOverallStart,
        ));
        let (points, groups) = save_path_section::<EnemyPathPoint>(&mut world);
        assert_eq!(points.len(), 1);
        assert_eq!(groups.len(), 1);
        assert_eq!((groups[0].start, groups[0].group_length), (0, 1));
    }

    #[test]
    #[should_panic(expected = "path group length exceeds u8 range")]
    fn export_rejects_group_length_overflow_instead_of_wrapping() {
        let mut world = World::new();
        let entities: Vec<_> = (0..256)
            .map(|order| {
                world
                    .spawn((
                        EnemyPathPoint::default(),
                        KmpPathNode::default(),
                        Transform::default(),
                        OrderId(order),
                    ))
                    .id()
            })
            .collect();
        for pair in entities.windows(2) {
            link_imported_nodes(&mut world, pair[0], pair[1]);
        }
        save_path_section::<EnemyPathPoint>(&mut world);
    }

    #[test]
    fn export_is_ordered_and_byte_stable_across_spawn_and_link_order() {
        use binrw::BinWrite;
        fn export(reverse: bool) -> Vec<u8> {
            let mut world = World::new();
            let orders = if reverse { vec![3, 2, 1, 0] } else { vec![0, 1, 2, 3] };
            let mut entities = [Entity::PLACEHOLDER; 4];
            for order in orders {
                entities[order] = world
                    .spawn((
                        EnemyPathPoint::default(),
                        Transform::from_xyz(order as f32, 0., 0.),
                        KmpPathNode::default(),
                        OrderId(order as u32),
                    ))
                    .id();
            }
            let targets = if reverse { vec![2, 1] } else { vec![1, 2] };
            for target in targets {
                link_imported_nodes(&mut world, entities[0], entities[target]);
            }
            let (points, groups) = save_path_section::<EnemyPathPoint>(&mut world);
            assert_eq!(
                points.iter().map(|p| p.position[0]).collect::<Vec<_>>(),
                vec![0., 1., 2., 3.]
            );
            assert_eq!(groups[0].next_group, [1, 2, 0xff, 0xff, 0xff, 0xff]);
            let mut bytes = std::io::Cursor::new(Vec::new());
            points.write_be(&mut bytes).unwrap();
            groups.write_be(&mut bytes).unwrap();
            bytes.into_inner()
        }
        assert_eq!(export(false), export(true));
    }

    #[test]
    fn checkpoint_export_uses_serialized_indices_not_order_ids() {
        use super::super::checkpoints::CheckpointLeft;
        let mut world = World::new();
        let mut entities = Vec::new();
        for order in [90, 10, 50] {
            let right = world.spawn(Transform::default()).id();
            entities.push(
                world
                    .spawn((
                        Checkpoint::default(),
                        CheckpointLeft { right, ..default() },
                        Transform::default(),
                        KmpPathNode::default(),
                        OrderId(order),
                    ))
                    .id(),
            );
        }
        world.entity_mut(entities[0]).insert(PathOverallStart);
        link_imported_nodes(&mut world, entities[0], entities[1]);
        link_imported_nodes(&mut world, entities[1], entities[2]);
        link_imported_nodes(&mut world, entities[2], entities[0]);
        let (points, groups) = save_path_section::<Checkpoint>(&mut world);
        assert_eq!(groups.len(), 1);
        assert_eq!(
            points.iter().map(|cp| (cp.prev_cp, cp.next_cp)).collect::<Vec<_>>(),
            vec![(0xff, 1), (0, 2), (1, 0xff)]
        );
        assert_eq!(checkpoint_serialized_neighbors(254, 0, 3), (0xff, 0xff));
        assert_eq!(checkpoint_serialized_neighbors(254, 2, 3), (0xff, 0xff));
    }

    #[test]
    fn adding_node_discards_stale_links() {
        let mut app = App::new();
        app.add_observer(on_add_kmp_path_node);

        let stale = app.world_mut().spawn_empty().id();
        app.world_mut().despawn(stale);
        let node = app
            .world_mut()
            .spawn((TestPathPoint, KmpPathNode::default().with_next([stale])))
            .id();

        assert!(app.world().get::<KmpPathNode>(node).unwrap().next_nodes.is_empty());
    }

    #[test]
    fn traverse_linear_path() {
        let mut world = World::new();
        let a = spawn_node(&mut world, true);
        let b = spawn_node(&mut world, false);
        let c = spawn_node(&mut world, false);
        assert!(KmpPathNode::link_nodes(a, b, &mut world));
        assert!(KmpPathNode::link_nodes(b, c, &mut world));

        let paths = traverse(&mut world);

        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].path, vec![a, b, c]);
        assert!(paths[0].prev_paths.is_empty());
        assert!(paths[0].next_paths.is_empty());
    }

    #[test]
    fn traverse_branch_creates_separate_groups() {
        let mut world = World::new();
        let a = spawn_node(&mut world, true);
        let branch = spawn_node(&mut world, false);
        let left = spawn_node(&mut world, false);
        let right = spawn_node(&mut world, false);
        assert!(KmpPathNode::link_nodes(a, branch, &mut world));
        assert!(KmpPathNode::link_nodes(branch, left, &mut world));
        assert!(KmpPathNode::link_nodes(branch, right, &mut world));

        let paths = traverse(&mut world);
        let root_path = path_containing(&paths, a);
        let left_path = path_containing(&paths, left);
        let right_path = path_containing(&paths, right);

        assert_eq!(paths.len(), 3);
        assert_eq!(paths[root_path].path, vec![a, branch]);
        assert_eq!(paths[root_path].next_paths.len(), 2);
        assert!(paths[root_path].next_paths.contains(&left_path));
        assert!(paths[root_path].next_paths.contains(&right_path));
        assert_eq!(paths[left_path].prev_paths, vec![root_path]);
        assert_eq!(paths[right_path].prev_paths, vec![root_path]);
    }

    #[test]
    fn traverse_disconnected_nodes_creates_separate_groups() {
        let mut world = World::new();
        let a = spawn_node(&mut world, true);
        let b = spawn_node(&mut world, false);

        let paths = traverse(&mut world);

        assert_eq!(paths.len(), 2);
        assert_ne!(path_containing(&paths, a), path_containing(&paths, b));
    }

    #[test]
    fn linking_allows_terminal_node_to_close_main_cycle() {
        let mut world = World::new();
        let a = spawn_node(&mut world, true);
        let b = spawn_node(&mut world, false);
        let c = spawn_node(&mut world, false);
        assert!(KmpPathNode::link_nodes(a, b, &mut world));
        assert!(KmpPathNode::link_nodes(b, c, &mut world));

        assert!(KmpPathNode::link_nodes(c, a, &mut world));
        assert!(world.get::<KmpPathNode>(c).unwrap().next_nodes.contains(&a));
        assert!(world.get::<KmpPathNode>(a).unwrap().prev_nodes.contains(&c));
    }

    #[test]
    fn linking_rejects_inner_cycle_without_mutating_graph() {
        let mut world = World::new();
        let a = spawn_node(&mut world, true);
        let b = spawn_node(&mut world, false);
        let c = spawn_node(&mut world, false);
        let d = spawn_node(&mut world, false);
        assert!(KmpPathNode::link_nodes(a, b, &mut world));
        assert!(KmpPathNode::link_nodes(b, c, &mut world));
        assert!(KmpPathNode::link_nodes(c, d, &mut world));

        assert!(!KmpPathNode::link_nodes(d, b, &mut world));
        assert!(!world.get::<KmpPathNode>(d).unwrap().next_nodes.contains(&b));
        assert!(!world.get::<KmpPathNode>(b).unwrap().prev_nodes.contains(&d));
    }

    #[test]
    fn linking_rejects_full_endpoints() {
        let mut world = World::new();
        let a = world.spawn((TestPathPoint, KmpPathNode::new(0))).id();
        let b = spawn_node(&mut world, false);

        assert!(!KmpPathNode::link_nodes(a, b, &mut world));
        assert!(world.get::<KmpPathNode>(a).unwrap().next_nodes.is_empty());
        assert!(world.get::<KmpPathNode>(b).unwrap().prev_nodes.is_empty());
    }

    #[test]
    fn traverse_imported_cycle_terminates_and_links_group_to_itself() {
        let mut world = World::new();
        let a = spawn_node(&mut world, true);
        let b = spawn_node(&mut world, false);
        let c = spawn_node(&mut world, false);
        assert!(KmpPathNode::link_nodes(a, b, &mut world));
        assert!(KmpPathNode::link_nodes(b, c, &mut world));
        // Simulate cyclic data imported from a file. Interactive linking rejects
        // this edge, but traversal must still handle malformed/existing cycles.
        world.get_mut::<KmpPathNode>(c).unwrap().next_nodes.insert(a);
        world.get_mut::<KmpPathNode>(a).unwrap().prev_nodes.insert(c);

        let paths = traverse(&mut world);

        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].path.len(), 3);
        assert_eq!(paths[0].next_paths, vec![0]);
        assert_eq!(paths[0].prev_paths, vec![0]);
    }
}
