use bevy::prelude::*;
use std::collections::HashSet;

use crate::{
    util::kmp_file::{KmpData, KmpPositionPoint, PathGroup, Poti},
    viewer::normalize::Normalize,
};

use super::{
    components::{FromKmp, Route},
    KmpSection, RouteMarker, RoutePoint,
};

// component attached to kmp entities which are linked to other kmp entities
#[derive(Component)]
pub struct KmpPathNode {
    pub prev_nodes: HashSet<Entity>,
    pub next_nodes: HashSet<Entity>,
}

// represents a link between 2 nodes
#[derive(Component)]
pub struct KmpPathNodeLink {
    prev_node: Entity,
    next_node: Entity,
}

// represents the line that links the 2 entities
#[derive(Component)]
pub struct KmpPathNodeLinkLine;

#[derive(Debug)]
pub struct LinkPathNodeError;
impl KmpPathNode {
    pub fn new() -> Self {
        KmpPathNode {
            prev_nodes: HashSet::new(),
            next_nodes: HashSet::new(),
        }
    }
    #[allow(dead_code)]
    pub fn delete_self(&mut self, mut kmp_node_query: Query<&mut KmpPathNode>) {
        // for all next nodes
        for e in self.next_nodes.iter() {
            // delete all references to self
            let mut next_node = kmp_node_query.get_mut(*e).unwrap();
            next_node.prev_nodes.retain(|x| x != e);
        }
        // for all previous nodes
        for e in self.prev_nodes.iter() {
            // delete all references to self
            let mut prev_node = kmp_node_query.get_mut(*e).unwrap();
            prev_node.next_nodes.retain(|x| x != e);
        }
    }
    // link nodes, taking in a kmp node query
    fn link_nodes(
        prev_node_entity: Entity,
        next_node_entity: Entity,
        kmp_node_query: &mut Query<&mut KmpPathNode>,
    ) -> Result<(), LinkPathNodeError> {
        let mut next_node = match kmp_node_query.get_mut(next_node_entity) {
            Ok(next_node) => next_node,
            Err(_) => return Err(LinkPathNodeError),
        };
        next_node.prev_nodes.insert(prev_node_entity);

        let mut prev_node = match kmp_node_query.get_mut(prev_node_entity) {
            Ok(prev_node) => prev_node,
            Err(_) => return Err(LinkPathNodeError),
        };
        prev_node.next_nodes.insert(prev_node_entity);

        Ok(())
    }
    // link nodes if direct world access is available
    fn link_nodes_world_access(
        prev_node_entity: Entity,
        next_node_entity: Entity,
        world: &mut World,
    ) -> Result<(), LinkPathNodeError> {
        let mut next_node = match world.get_mut::<KmpPathNode>(next_node_entity) {
            Some(next_node) => next_node,
            None => return Err(LinkPathNodeError),
        };
        next_node.prev_nodes.insert(prev_node_entity);

        let mut prev_node = match world.get_mut::<KmpPathNode>(prev_node_entity) {
            Some(prev_node) => prev_node,
            None => return Err(LinkPathNodeError),
        };
        prev_node.next_nodes.insert(next_node_entity);
        Ok(())
    }
}

struct EntityGroup {
    entities: Vec<Entity>,
    next_groups: Vec<u8>,
}

struct KmpDataGroup<T> {
    nodes: Vec<T>,
    next_groups: Vec<u8>,
}

pub fn spawn_path_section<
    T: KmpData + KmpPositionPoint + Send + 'static + Clone,
    U: Component + FromKmp<T>,
    V: Component + Default,
>(
    commands: &mut Commands,
    kmp_pathgroup_entries: &[PathGroup],
    kmp_node_entries: &[T],

    sphere_mesh: Handle<Mesh>,
    cylinder_mesh: Handle<Mesh>,
    frustrum_mesh: Handle<Mesh>,

    point_material: Handle<StandardMaterial>,
    line_material: Handle<StandardMaterial>,
    arrow_material: Handle<StandardMaterial>,
) {
    let mut kmp_data_groups: Vec<KmpDataGroup<T>> = Vec::new();

    for group in kmp_pathgroup_entries.iter() {
        let mut next_groups = Vec::new();
        for next_group in group.next_group {
            if next_group != 0xff {
                next_groups.push(next_group);
            }
        }
        let mut nodes = Vec::new();
        for i in group.start..(group.start + group.group_length) {
            let node = &kmp_node_entries[i as usize];
            nodes.push(node.clone());
        }
        kmp_data_groups.push(KmpDataGroup { nodes, next_groups });
    }

    commands.add(move |world: &mut World| {
        // spawn all the entities, saving the entity IDs into 'entity_groups'
        let mut entity_groups: Vec<EntityGroup> = Vec::new();
        for group in kmp_data_groups {
            let mut entity_group = EntityGroup {
                entities: Vec::new(),
                next_groups: group.next_groups,
            };
            for node in group.nodes {
                let spawned_entity = world.spawn((
                    PbrBundle {
                        mesh: sphere_mesh.clone(),
                        material: point_material.clone(),
                        transform: Transform::from_translation(node.get_position()),
                        visibility: Visibility::Hidden,
                        ..default()
                    },
                    KmpPathNode::new(),
                    V::default(),
                    U::from_kmp(node),
                    KmpSection,
                    Normalize::new(200., 30., BVec3::TRUE),
                ));
                entity_group.entities.push(spawned_entity.id());
            }
            entity_groups.push(entity_group);
        }
        // link the entities together
        for group in entity_groups.iter() {
            let mut prev_entity: Option<Entity> = None;
            // in each group, link the previous node to the current node
            for entity in group.entities.iter() {
                if let Some(prev_entity) = prev_entity {
                    KmpPathNode::link_nodes_world_access(prev_entity, *entity, world).unwrap();
                    spawn_node_link::<V>(
                        world,
                        prev_entity,
                        *entity,
                        cylinder_mesh.clone(),
                        frustrum_mesh.clone(),
                        line_material.clone(),
                        arrow_material.clone(),
                    );
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
                KmpPathNode::link_nodes_world_access(entity, next_entity, world).unwrap();
                spawn_node_link::<V>(
                    world,
                    entity,
                    next_entity,
                    cylinder_mesh.clone(),
                    frustrum_mesh.clone(),
                    line_material.clone(),
                    arrow_material.clone(),
                );
            }
        }
    });
}

pub fn spawn_route_section(
    commands: &mut Commands,
    kmp_poti_entries: Vec<Poti>,

    sphere_mesh: Handle<Mesh>,
    cylinder_mesh: Handle<Mesh>,
    frustrum_mesh: Handle<Mesh>,

    point_material: Handle<StandardMaterial>,
    line_material: Handle<StandardMaterial>,
    arrow_material: Handle<StandardMaterial>,
) {
    commands.add(move |world: &mut World| {
        // spawn all the entities, saving the entity IDs into 'entity_groups'
        let mut entity_groups: Vec<Vec<Entity>> = Vec::new();
        for group in kmp_poti_entries {
            let mut entity_group = Vec::new();

            let mut parent =
                world.spawn((SpatialBundle::default(), Route::from_kmp(group.clone())));

            parent.with_children(|parent| {
                for node in group.routes {
                    let spawned_entity = parent.spawn((
                        PbrBundle {
                            mesh: sphere_mesh.clone(),
                            material: point_material.clone(),
                            transform: Transform::from_translation(node.position),
                            visibility: Visibility::Hidden,
                            ..default()
                        },
                        KmpPathNode::new(),
                        RouteMarker,
                        RoutePoint::from_kmp(node),
                        KmpSection,
                        Normalize::new(200., 30., BVec3::TRUE),
                    ));
                    entity_group.push(spawned_entity.id());
                }
            });
            entity_groups.push(entity_group);
        }
        // link the entities together
        for group in entity_groups.iter() {
            let mut prev_entity: Option<Entity> = None;
            // in each group, link the previous node to the current node
            for entity in group.iter() {
                if let Some(prev_entity) = prev_entity {
                    KmpPathNode::link_nodes_world_access(prev_entity, *entity, world).unwrap();
                    spawn_node_link::<RouteMarker>(
                        world,
                        prev_entity,
                        *entity,
                        cylinder_mesh.clone(),
                        frustrum_mesh.clone(),
                        line_material.clone(),
                        arrow_material.clone(),
                    );
                }
                prev_entity = Some(*entity);
            }
        }
    });
}

fn spawn_node_link<T: Component + Default>(
    world: &mut World,
    prev_node: Entity,
    next_node: Entity,
    cylinder_mesh: Handle<Mesh>,
    frustrum_mesh: Handle<Mesh>,

    line_material: Handle<StandardMaterial>,
    arrow_material: Handle<StandardMaterial>,
) {
    let prev_pos = world.get::<Transform>(prev_node).unwrap().translation;
    let next_pos = world.get::<Transform>(next_node).unwrap().translation;

    let mut parent_transform =
        Transform::from_translation(prev_pos.lerp(next_pos, 0.5)).looking_at(next_pos, Vec3::Y);
    parent_transform.rotate_local_x(f32::to_radians(-90.));

    let mut line_transform = Transform::default();
    line_transform.scale.y = prev_pos.distance(next_pos);

    // spawn a parent component which contains a transform, and stores the entities of the nodes the node links
    world
        .spawn((
            SpatialBundle {
                transform: parent_transform,
                visibility: Visibility::Hidden,
                ..default()
            },
            KmpPathNodeLink {
                prev_node,
                next_node,
            },
            KmpSection,
            T::default(),
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
                KmpSection,
                Normalize::new(200., 30., BVec3::new(true, false, true)),
                KmpPathNodeLinkLine,
            ));
            parent.spawn((
                PbrBundle {
                    mesh: frustrum_mesh,
                    material: arrow_material,
                    ..default()
                },
                KmpSection,
                Normalize::new(200., 30., BVec3::TRUE),
            ));
        });
}

pub fn update_node_links(
    kmp_node_link_query: Query<(Entity, &KmpPathNodeLink, &Children, &ViewVisibility)>,
    mut transform_query: Query<&mut Transform>,
    line_query: Query<&KmpPathNodeLinkLine>,
) {
    // go through each node line
    for (entity, kmp_node_link, children, visibility) in kmp_node_link_query.iter() {
        // don't bother unless the kmp node link is actually visible
        if *visibility == ViewVisibility::HIDDEN {
            continue;
        }

        // get the positions of the previous and next nodes
        let prev_pos = transform_query
            .get(kmp_node_link.prev_node)
            .unwrap()
            .translation;
        let next_pos = transform_query
            .get(kmp_node_link.next_node)
            .unwrap()
            .translation;

        // calculate new transforms for the parent and the line
        let mut new_parent_transform =
            Transform::from_translation(prev_pos.lerp(next_pos, 0.5)).looking_at(next_pos, Vec3::Y);
        new_parent_transform.rotate_local_x(f32::to_radians(-90.));
        let mut new_line_transform = Transform::default();
        new_line_transform.scale.y = prev_pos.distance(next_pos);

        // set the transform of the parent
        let mut parent_transform = transform_query.get_mut(entity).unwrap();
        *parent_transform = new_parent_transform;

        // find the child of the kmp node link that has KmpNodeLinkLine, and set its transform
        for child in children {
            if line_query.get(*child).is_ok() {
                let mut line_transform = transform_query.get_mut(*child).unwrap();
                *line_transform = new_line_transform;
                break;
            }
        }
    }
}
