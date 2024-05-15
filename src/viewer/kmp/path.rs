#![allow(dead_code)]

use super::{
    components::FromKmp,
    meshes_materials::KmpMeshesMaterials,
    sections::{KmpEditMode, KmpSections},
    CheckpointLeft, CheckpointRight, EnemyPathMarker, EnemyPathPoint, GetPathMaterialSection, HideRotation,
    ItemPathMarker, ItemPathPoint, KmpError, KmpSelectablePoint, PathOverallStart,
};
use crate::{
    ui::settings::AppSettings,
    util::kmp_file::{KmpFile, KmpGetPathSection, KmpGetSection, KmpPositionPoint},
    viewer::{
        edit::{
            transform_gizmo::GizmoTransformable,
            tweak::{SnapTo, Tweakable},
        },
        normalize::Normalize,
    },
};
use bevy::{
    ecs::query::{QueryData, QueryFilter, WorldQuery},
    prelude::*,
    utils::HashMap,
};
use bevy_mod_outline::{OutlineBundle, OutlineVolume};
use std::fmt::Debug;
use std::{collections::HashSet, sync::Arc};

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
    CheckpointLeft,
    CheckpointRight,
}

// represents the line that links the 2 entities
#[derive(Component)]
pub struct KmpPathNodeLinkLine;

#[derive(Debug)]
pub struct KmpPathNodeError;

// component attached to kmp entities which are linked to other kmp entities
#[derive(Component, Clone, Debug, Default)]
pub struct KmpPathNode {
    pub prev_nodes: HashSet<Entity>,
    pub next_nodes: HashSet<Entity>,
}
impl KmpPathNode {
    pub fn new() -> Self {
        KmpPathNode {
            prev_nodes: HashSet::with_capacity(6),
            next_nodes: HashSet::with_capacity(6),
        }
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
    #[allow(dead_code)]
    pub fn delete<Q: QueryData, F: QueryFilter>(self, self_entity: Entity, q_kmp_path_node: &mut Query<'_, '_, Q, F>)
    where
        for<'a> Q: WorldQuery<Item<'a> = Mut<'a, KmpPathNode>>,
    {
        // for all next nodes
        for e in self.next_nodes.iter() {
            // delete all references to self
            let Ok(mut next_node) = q_kmp_path_node.get_mut(*e) else {
                continue;
            };
            next_node.prev_nodes.retain(|x| *x != self_entity);
        }
        // for all previous nodes
        for e in self.prev_nodes.iter() {
            // delete all references to self
            let Ok(mut prev_node) = q_kmp_path_node.get_mut(*e) else {
                continue;
            };
            prev_node.next_nodes.retain(|x| *x != self_entity);
        }
    }

    pub fn link_nodes(prev_node_entity: Entity, next_node_entity: Entity, world: &mut World) -> bool {
        if prev_node_entity == next_node_entity {
            return false;
        }
        // get next and prev nodes immutably first so we can check if they are linked
        let Some(next_node) = world.get::<KmpPathNode>(next_node_entity) else {
            return false;
        };
        let Some(prev_node) = world.get::<KmpPathNode>(prev_node_entity) else {
            return false;
        };
        if prev_node.is_linked_with(prev_node_entity, next_node, next_node_entity) {
            return false;
        }
        if next_node.prev_nodes.len() >= 6 || prev_node.next_nodes.len() >= 6 {
            return false;
        }

        // now get them mutably one at a time to link them
        let mut next_node = world.get_mut::<KmpPathNode>(next_node_entity).unwrap();
        next_node.prev_nodes.insert(prev_node_entity);
        let mut prev_node = world.get_mut::<KmpPathNode>(prev_node_entity).unwrap();
        prev_node.next_nodes.insert(next_node_entity);

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

pub struct PathPointSpawner<U, Marker> {
    position: Vec3,
    rotation: Quat,
    kmp_component: U,
    visible: bool,
    marker: Marker,
    prev_nodes: HashSet<Entity>,
    e: Option<Entity>,
}
impl<U: Component + Clone + GetPathMaterialSection, Marker: Component + Default> PathPointSpawner<U, Marker> {
    pub fn new(kmp_component: U) -> Self {
        Self {
            position: Vec3::default(),
            rotation: Quat::default(),
            kmp_component,
            visible: true,
            marker: Marker::default(),
            prev_nodes: HashSet::new(),
            e: None,
        }
    }
    pub fn pos(mut self, pos: Vec3) -> Self {
        self.position = pos;
        self
    }
    pub fn rot(mut self, rot: Quat) -> Self {
        self.rotation = rot;
        self
    }
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }
    pub fn prev_nodes(mut self, prev_nodes: HashSet<Entity>) -> Self {
        self.prev_nodes = prev_nodes;
        self
    }

    pub fn spawn_command(mut self, commands: &mut Commands) -> Entity {
        let e = self.e.unwrap_or_else(|| commands.spawn_empty().id());
        self.e = Some(e);
        commands.add(|world: &mut World| {
            self.spawn(world);
        });
        e
    }
    pub fn spawn(self, world: &mut World) -> Entity {
        let meshes_materials = world.resource::<KmpMeshesMaterials>();
        let mesh = meshes_materials.meshes.sphere.clone();
        let material = U::get_materials(&meshes_materials.materials).point.clone();
        let outline = world.get_resource::<AppSettings>().unwrap().kmp_model.outline.clone();

        let mut entity = match self.e {
            Some(e) => world.entity_mut(e),
            None => world.spawn_empty(),
        };
        entity.insert((
            PbrBundle {
                mesh,
                material,
                transform: Transform::from_translation(self.position).with_rotation(self.rotation),
                visibility: if self.visible {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                },
                ..default()
            },
            KmpPathNode {
                prev_nodes: self.prev_nodes.clone(),
                next_nodes: HashSet::new(),
            },
            Marker::default(),
            self.kmp_component.clone(),
            KmpSelectablePoint,
            Tweakable(SnapTo::Kcl),
            HideRotation,
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
}

pub fn spawn_enemy_item_path_section<
    T: KmpGetSection + KmpGetPathSection + KmpPositionPoint + Send + 'static + Clone,
    U: Component + FromKmp<T> + Clone + Debug + GetPathMaterialSection,
    Marker: Component + Default,
>(
    commands: &mut Commands,
    kmp: Arc<KmpFile>,
    kmp_errors: &mut Vec<KmpError>,
) {
    let kmp_groups = get_kmp_data_and_component_groups::<T, U>(kmp, kmp_errors);

    commands.add(move |world: &mut World| {
        let mut entity_groups: Vec<EntityGroup> = Vec::with_capacity(kmp_groups.len());
        for (i, (data_group, component_group)) in kmp_groups.iter().enumerate() {
            let mut entity_group = EntityGroup {
                entities: Vec::with_capacity(data_group.nodes.len()),
                next_groups: data_group.next_groups.clone(),
            };
            for (j, node) in data_group.nodes.iter().enumerate() {
                let kmp_component = component_group[j].clone();
                let spawned_entity = PathPointSpawner::<_, Marker>::new(kmp_component)
                    .pos(node.get_position().into())
                    .visible(false)
                    .spawn(world);
                if i == 0 && j == 0 {
                    world.entity_mut(spawned_entity).insert(PathOverallStart);
                }
                entity_group.entities.push(spawned_entity);
            }
            entity_groups.push(entity_group);
        }
        link_entity_groups(world, entity_groups);
    });
}

/// converts points and paths in the kmp to a list of groups containing the data, and components that have been converted from that data
pub fn get_kmp_data_and_component_groups<
    // this is the original kmp data from the file
    T: KmpGetSection + KmpGetPathSection + Send + 'static + Clone,
    // this is the kmp component which corresponds to the kmp data
    U: Component + FromKmp<T> + Clone + Debug,
>(
    kmp: Arc<KmpFile>,
    kmp_errors: &mut Vec<KmpError>,
) -> Vec<(KmpDataGroup<T>, Vec<U>)> {
    let pathgroup_entries = &T::get_path_section(kmp.as_ref()).entries;
    let node_entries = &T::get_section(kmp.as_ref()).entries;

    let mut result: Vec<(KmpDataGroup<T>, Vec<U>)> = Vec::with_capacity(pathgroup_entries.len());

    let mut acc = 0;
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
            let kmp_component = U::from_kmp(node, kmp_errors, acc);
            kmp_component_group.push(kmp_component);
            acc += 1;
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

pub fn spawn_path_section<
    // this is the original kmp data from the file
    T: KmpGetSection + KmpGetPathSection + Send + 'static + Clone,
    // this is the kmp component which corresponds to the kmp data
    U: Component + FromKmp<T> + Clone + Debug,
>(
    commands: &mut Commands,
    kmp: Arc<KmpFile>,
    kmp_errors: &mut Vec<KmpError>,
    spawn: impl Fn(&T, U, &mut World) -> Entity + Send + 'static,
) {
    let kmp_groups = get_kmp_data_and_component_groups::<T, U>(kmp, kmp_errors);

    commands.add(move |world: &mut World| {
        // spawn all the entities, saving the entity IDs into 'entity_groups'
        let mut entity_groups: Vec<EntityGroup> = Vec::with_capacity(kmp_groups.len());
        for (i, (data_group, component_group)) in kmp_groups.iter().enumerate() {
            let mut entity_group = EntityGroup {
                entities: Vec::with_capacity(data_group.nodes.len()),
                next_groups: data_group.next_groups.clone(),
            };
            for (j, node) in data_group.nodes.iter().enumerate() {
                let kmp_component = component_group[j].clone();
                // we don't know how each entity is going to be spawned in this function
                // this is good because we can use it both for checkpoints and enemy/item paths
                let spawned_entity = spawn(node, kmp_component, world);
                // if we are at the start then add the start marker to the point
                if i == 0 && j == 0 {
                    world.entity_mut(spawned_entity).insert(PathOverallStart);
                }
                entity_group.entities.push(spawned_entity);
            }
            entity_groups.push(entity_group);
        }
        link_entity_groups(world, entity_groups);
    });
}

fn spawn_node_link(
    world: &mut World,
    prev_node: Entity,
    next_node: Entity,
    kind: PathType,
    cylinder_mesh: Handle<Mesh>,
    frustrum_mesh: Handle<Mesh>,
    line_material: Handle<StandardMaterial>,
    arrow_material: Handle<StandardMaterial>,
    visible: bool,
) {
    let prev_pos = world.get::<Transform>(prev_node).unwrap().translation;
    let next_pos = world.get::<Transform>(next_node).unwrap().translation;

    let mut parent_transform = Transform::from_translation(prev_pos.lerp(next_pos, 0.5)).looking_at(next_pos, Vec3::Y);
    parent_transform.rotate_local_x(f32::to_radians(-90.));

    let mut line_transform = Transform::default();
    line_transform.scale.y = prev_pos.distance(next_pos);

    // spawn a parent component which contains a transform, and stores the entities of the nodes the node links
    world
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
                kind,
            },
        ))
        // spawn the line and arrow as children of this parent component, which will inherit its transform & visibility
        .with_children(|parent| {
            parent.spawn((
                PbrBundle {
                    mesh: cylinder_mesh,
                    material: line_material,
                    transform: line_transform,
                    ..default()
                },
                // KmpSection,
                Normalize::new(200., 30., BVec3::new(true, false, true)),
                KmpPathNodeLinkLine,
            ));
            parent.spawn((
                PbrBundle {
                    mesh: frustrum_mesh,
                    material: arrow_material,
                    ..default()
                },
                // KmpSection,
                Normalize::new(200., 30., BVec3::TRUE),
            ));
        });
}

pub fn update_node_links(
    q_kmp_node_link: Query<(Entity, &KmpPathNodeLink, &Children, &ViewVisibility)>,
    q_kmp_node: Query<(
        Entity,
        Has<EnemyPathMarker>,
        Has<ItemPathMarker>,
        Has<CheckpointLeft>,
        Has<CheckpointRight>,
        &KmpPathNode,
    )>,
    mut q_transform: Query<&mut Transform>,
    q_line: Query<&KmpPathNodeLinkLine>,
    mut commands: Commands,
    kmp_edit_mode: Res<KmpEditMode>,
) {
    let mut nodes_to_be_linked: HashMap<(Entity, Entity), PathType> = HashMap::new();
    for (cur_node, is_enemy, is_item, is_cp_left, _, node_data) in q_kmp_node.iter() {
        let path_type = if is_enemy {
            PathType::Enemy
        } else if is_item {
            PathType::Item
        } else if is_cp_left {
            PathType::CheckpointLeft
        } else {
            PathType::CheckpointRight
        };
        for prev_node in node_data.prev_nodes.iter() {
            nodes_to_be_linked.insert((*prev_node, cur_node), path_type);
        }
        for next_node in node_data.next_nodes.iter() {
            nodes_to_be_linked.insert((cur_node, *next_node), path_type);
        }
    }

    // go through each node line
    for (link_entity, kmp_node_link, children, visibility) in q_kmp_node_link.iter() {
        if !nodes_to_be_linked.contains_key(&(kmp_node_link.prev_node, kmp_node_link.next_node)) {
            commands.entity(link_entity).despawn_recursive();
            continue;
        }
        nodes_to_be_linked.remove(&(kmp_node_link.prev_node, kmp_node_link.next_node));
        // don't bother unless the kmp node link is actually visible
        if *visibility == ViewVisibility::HIDDEN {
            continue;
        }

        // see https://github.com/bevyengine/bevy/issues/11517
        let Ok(transforms) = q_transform.get_many_mut([kmp_node_link.prev_node, kmp_node_link.next_node]) else {
            commands.entity(link_entity).despawn_recursive();
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
        let (prev_node, next_node) = *node_not_linked.0;
        let path_type = *node_not_linked.1;
        let kmp_edit_mode = kmp_edit_mode.0;
        commands.add(move |world: &mut World| {
            let meshes_materials = world.resource::<KmpMeshesMaterials>();
            macro_rules! spawn_node_link {
                ($mat:ident, $edit_mode:ident, $path_type:ident) => {
                    spawn_node_link(
                        world,
                        prev_node,
                        next_node,
                        $path_type,
                        meshes_materials.meshes.cylinder.clone(),
                        meshes_materials.meshes.frustrum.clone(),
                        meshes_materials.materials.$mat.line.clone(),
                        meshes_materials.materials.$mat.arrow.clone(),
                        kmp_edit_mode == KmpSections::$edit_mode,
                    )
                };
            }
            match path_type {
                PathType::Enemy => spawn_node_link!(enemy_paths, EnemyPaths, path_type),
                PathType::Item => spawn_node_link!(item_paths, ItemPaths, path_type),
                PathType::CheckpointLeft | PathType::CheckpointRight => {
                    spawn_node_link!(checkpoints, Checkpoints, path_type)
                }
            };
        });
    }
}

#[derive(Event, Default)]
pub struct RecalculatePaths;

pub fn traverse_paths<'a>(
    mut p: ParamSet<(
        Query<Entity, (With<PathOverallStart>, With<EnemyPathMarker>)>,
        Query<Entity, (With<PathOverallStart>, With<ItemPathMarker>)>,
    )>,
    q_kmp_node: Query<(&'a KmpPathNode, Option<&'a EnemyPathPoint>, Option<&'a ItemPathPoint>)>,
    q_is_overall_start: Query<(), With<PathOverallStart>>,
    mut commands: Commands,
) {
    let Ok(enemy_start) = p.p0().get_single() else {
        return;
    };
    let Ok(item_start) = p.p1().get_single() else {
        return;
    };

    let mut traverser = Traverser::new(q_kmp_node, q_is_overall_start);

    let enemy_groups = traverser.traverse(enemy_start);
    let item_groups = traverser.traverse(item_start);

    let enemy_groups: Vec<_> = enemy_groups
        .iter()
        .map(|e| PathGroup {
            paths: e.clone(),
            visible: true,
        })
        .collect();
    let item_groups: Vec<_> = item_groups
        .iter()
        .map(|e| PathGroup {
            paths: e.clone(),
            visible: true,
        })
        .collect();

    commands.insert_resource(EnemyPathGroups(enemy_groups));
    commands.insert_resource(ItemPathGroups(item_groups));
}

#[derive(Clone)]
pub struct PathGroup {
    pub paths: Vec<Entity>,
    pub visible: bool,
}

#[derive(Resource, Clone)]
pub struct EnemyPathGroups(pub Vec<PathGroup>);
#[derive(Resource, Clone)]
pub struct ItemPathGroups(pub Vec<PathGroup>);

struct Traverser<'a, 'w, 's> {
    q_kmp_node: Query<'w, 's, (&'a KmpPathNode, Option<&'a EnemyPathPoint>, Option<&'a ItemPathPoint>)>,
    q_is_overall_start: Query<'w, 's, (), With<PathOverallStart>>,
    groups_accum: Vec<Vec<Entity>>,
    visited: HashSet<Entity>,
}
impl<'a, 'w, 's> Traverser<'a, 'w, 's> {
    pub fn new(
        q_kmp_node: Query<'w, 's, (&'a KmpPathNode, Option<&'a EnemyPathPoint>, Option<&'a ItemPathPoint>)>,
        q_is_overall_start: Query<'w, 's, (), With<PathOverallStart>>,
    ) -> Self {
        Self {
            q_kmp_node,
            q_is_overall_start,
            groups_accum: Vec::new(),
            visited: HashSet::new(),
        }
    }
    pub fn traverse(&mut self, start_node: Entity) -> Vec<Vec<Entity>> {
        self.traverse_internal(start_node, 0);
        let groups = self.groups_accum.clone();
        self.reset();
        groups
    }
    fn reset(&mut self) {
        self.groups_accum = Vec::new();
        self.visited = HashSet::new();
    }
    fn traverse_internal(&mut self, cur_node: Entity, mut cur_index: usize) {
        let (kmp_node, enemy_point, item_point) = self.q_kmp_node.get(cur_node).unwrap();

        let at_start = self.q_is_overall_start.get(cur_node).is_ok();
        let initial_start = at_start && self.groups_accum.is_empty();
        if !self.visited.insert(cur_node) {
            return;
        }

        let we_should_start_new_group = initial_start
            || kmp_node.prev_nodes.len() > 1
            || kmp_node
                .prev_nodes
                .iter()
                .any(|e| self.q_kmp_node.get(*e).unwrap().0.next_nodes.len() > 1)
            || enemy_point.map(|x| x.path_start_override).unwrap_or(false)
            || item_point.map(|x| x.path_start_override).unwrap_or(false);

        if we_should_start_new_group {
            self.groups_accum.push(vec![cur_node]);
            cur_index = self.groups_accum.len() - 1;
        } else {
            self.groups_accum[cur_index].push(cur_node)
        }

        if kmp_node.next_nodes.is_empty() {
            return;
        }

        for next_node in kmp_node.next_nodes.clone().iter() {
            // recursion, woohoo
            self.traverse_internal(*next_node, cur_index);
        }
    }
}
