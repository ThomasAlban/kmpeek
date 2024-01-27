use super::{
    components::FromKmp,
    meshes_materials::{KmpMeshes, PathMaterials},
    settings::OutlineSettings,
    KmpSelectablePoint,
};
use crate::{
    util::kmp_file::{KmpFile, KmpGetPathSection, KmpGetSection, KmpPositionPoint},
    viewer::{edit::select::Selected, normalize::Normalize},
};
use bevy::prelude::*;
use bevy_mod_outline::{OutlineBundle, OutlineVolume};
use std::fmt::Debug;
use std::{collections::HashSet, sync::Arc};

// represents a link between 2 nodes
#[derive(Component)]
pub struct KmpPathNodeLink {
    pub prev_node: Entity,
    pub next_node: Entity,
}

// represents the line that links the 2 entities
#[derive(Component)]
pub struct KmpPathNodeLinkLine;

#[derive(Debug)]
pub struct KmpPathNodeError;

// component attached to kmp entities which are linked to other kmp entities
#[derive(Component)]
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
    pub fn is_next_node_of(&self, other: &KmpPathNode) -> bool {
        for self_prev in self.prev_nodes.iter() {
            for other_next in other.next_nodes.iter() {
                if self.prev_nodes.contains(other_next) && other.next_nodes.contains(self_prev) {
                    return true;
                }
            }
        }
        false
    }
    pub fn is_prev_node_of(&self, other: &KmpPathNode) -> bool {
        for self_next in self.next_nodes.iter() {
            for other_prev in other.prev_nodes.iter() {
                if self.next_nodes.contains(other_prev) && other.prev_nodes.contains(self_next) {
                    return true;
                }
            }
        }
        false
    }
    pub fn is_linked_with(&self, other: &KmpPathNode) -> bool {
        self.is_next_node_of(other) || self.is_prev_node_of(other)
    }
    #[allow(dead_code)]
    pub fn delete(&mut self, mut q_kmp_node: &mut Query<&mut KmpPathNode, Without<Selected>>) {
        // for all next nodes
        for e in self.next_nodes.iter() {
            // delete all references to self
            let Ok(mut next_node) = q_kmp_node.get_mut(*e) else {
                continue;
            };
            next_node.prev_nodes.retain(|x| x != e);
        }
        // for all previous nodes
        for e in self.prev_nodes.iter() {
            // delete all references to self
            let Ok(mut prev_node) = q_kmp_node.get_mut(*e) else {
                continue;
            };
            prev_node.next_nodes.retain(|x| x != e);
        }
    }
    // link nodes, taking in a kmp node query
    fn link_nodes(
        prev_node_entity: Entity,
        next_node_entity: Entity,
        q_kmp_node: &mut Query<&mut KmpPathNode>,
    ) -> Result<(), KmpPathNodeError> {
        let mut next_node = match q_kmp_node.get_mut(next_node_entity) {
            Ok(next_node) => next_node,
            Err(_) => return Err(KmpPathNodeError),
        };
        if next_node.prev_nodes.len() >= 6 {
            return Err(KmpPathNodeError);
        }
        next_node.prev_nodes.insert(prev_node_entity);

        let mut prev_node = match q_kmp_node.get_mut(prev_node_entity) {
            Ok(prev_node) => prev_node,
            Err(_) => return Err(KmpPathNodeError),
        };
        if prev_node.next_nodes.len() >= 6 {
            return Err(KmpPathNodeError);
        }
        prev_node.next_nodes.insert(prev_node_entity);

        Ok(())
    }
    // link nodes if direct world access is available
    fn link_nodes_world_access(
        prev_node_entity: Entity,
        next_node_entity: Entity,
        world: &mut World,
    ) -> Result<(), KmpPathNodeError> {
        let mut next_node = match world.get_mut::<KmpPathNode>(next_node_entity) {
            Some(next_node) => next_node,
            None => return Err(KmpPathNodeError),
        };
        if next_node.prev_nodes.len() >= 6 {
            return Err(KmpPathNodeError);
        }
        next_node.prev_nodes.insert(prev_node_entity);

        let mut prev_node = match world.get_mut::<KmpPathNode>(prev_node_entity) {
            Some(prev_node) => prev_node,
            None => return Err(KmpPathNodeError),
        };
        if prev_node.next_nodes.len() >= 6 {
            return Err(KmpPathNodeError);
        }
        prev_node.next_nodes.insert(next_node_entity);
        Ok(())
    }
}

#[derive(Clone)]
pub struct EntityGroup {
    pub entities: Vec<Entity>,
    pub next_groups: Vec<u8>,
}

struct KmpDataGroup<T> {
    nodes: Vec<T>,
    next_groups: Vec<u8>,
}

pub fn spawn_path_section<
    T: KmpGetSection + KmpGetPathSection + KmpPositionPoint + Send + 'static + Clone,
    U: Component + FromKmp<T>,
    V: Component + Default,
>(
    commands: &mut Commands,
    kmp: Arc<KmpFile>,
    meshes: KmpMeshes,
    materials: PathMaterials,
    outline: OutlineSettings,
) {
    let pathgroup_entries = &T::get_path_section(kmp.as_ref()).entries;
    let node_entries = &T::get_section(kmp.as_ref()).entries;

    let mut kmp_data_groups: Vec<KmpDataGroup<T>> = Vec::new();

    for group in pathgroup_entries.iter() {
        let mut next_groups = Vec::new();
        for next_group in group.next_group {
            if next_group != 0xff {
                next_groups.push(next_group);
            }
        }
        let mut nodes = Vec::new();
        for i in group.start..(group.start + group.group_length) {
            let node = &node_entries[i as usize];
            nodes.push(node.clone());
        }
        kmp_data_groups.push(KmpDataGroup { nodes, next_groups });
    }

    // let mut entity_groups: Vec<EntityGroup> = Vec::with_capacity(kmp_data_groups.len());
    // for group in kmp_data_groups {
    //     let mut entity_group = EntityGroup {
    //         entities: Vec::with_capacity(group.nodes.len()),
    //         next_groups: group.next_groups,
    //     };

    //     for node in group.nodes.iter() {
    //         let position: Vec3 = node.get_position().into();
    //         let spawned_entity = commands.spawn((
    //             PbrBundle {
    //                 mesh: meshes.sphere.clone(),
    //                 material: materials.point.clone(),
    //                 transform: Transform::from_translation(position),
    //                 visibility: Visibility::Hidden,
    //                 ..default()
    //             },
    //             V::default(),
    //             U::from_kmp(node),
    //             KmpSelectablePoint,
    //             Normalize::new(200., 30., BVec3::TRUE),
    //             OutlineBundle {
    //                 outline: OutlineVolume {
    //                     visible: false,
    //                     colour: outline.color,
    //                     width: outline.width,
    //                 },
    //                 ..default()
    //             },
    //         ));
    //         entity_group.entities.push(spawned_entity.id());
    //     }
    //     entity_groups.push(entity_group);
    // }

    commands.add(move |world: &mut World| {
        // spawn all the entities, saving the entity IDs into 'entity_groups'
        let mut entity_groups: Vec<EntityGroup> = Vec::new();
        for group in kmp_data_groups {
            let mut entity_group = EntityGroup {
                entities: Vec::new(),
                next_groups: group.next_groups,
            };
            for node in group.nodes.iter() {
                let position: Vec3 = node.get_position().into();
                let spawned_entity = world.spawn((
                    PbrBundle {
                        mesh: meshes.sphere.clone(),
                        material: materials.point.clone(),
                        transform: Transform::from_translation(position),
                        visibility: Visibility::Hidden,
                        ..default()
                    },
                    KmpPathNode::new(),
                    V::default(),
                    U::from_kmp(node),
                    KmpSelectablePoint,
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
                        meshes.cylinder.clone(),
                        meshes.frustrum.clone(),
                        materials.line.clone(),
                        materials.arrow.clone(),
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
                    meshes.cylinder.clone(),
                    meshes.frustrum.clone(),
                    materials.line.clone(),
                    materials.arrow.clone(),
                );
            }
        }
    });
}

// pub fn spawn_route_section(
//     commands: &mut Commands,
//     kmp: Arc<Kmp>,
//     meshes: PathMeshes,
//     materials: PathMaterials,
// ) {
//     let poti_entries = Poti::get_section(kmp.as_ref()).entries.clone();

//     commands.add(move |world: &mut World| {
//         // spawn all the entities, saving the entity IDs into 'entity_groups'
//         let mut entity_groups: Vec<Vec<Entity>> = Vec::new();
//         for group in poti_entries.iter() {
//             let mut entity_group = Vec::new();

//             let mut parent = world.spawn((SpatialBundle::default(), Route::from_kmp(group)));

//             parent.with_children(|parent| {
//                 for node in group.routes.iter() {
//                     let spawned_entity = parent.spawn((
//                         PbrBundle {
//                             mesh: meshes.sphere.clone(),
//                             material: materials.point.clone(),
//                             transform: Transform::from_translation(node.position.into()),
//                             visibility: Visibility::Hidden,
//                             ..default()
//                         },
//                         KmpPathNode::new(),
//                         RouteMarker,
//                         RoutePoint::from_kmp(node),
//                         KmpSelectablePoint,
//                         Normalize::new(200., 30., BVec3::TRUE),
//                     ));
//                     entity_group.push(spawned_entity.id());
//                 }
//             });
//             entity_groups.push(entity_group);
//         }
//         // link the entities together
//         for group in entity_groups.iter() {
//             let mut prev_entity: Option<Entity> = None;
//             // in each group, link the previous node to the current node
//             for entity in group.iter() {
//                 if let Some(prev_entity) = prev_entity {
//                     KmpPathNode::link_nodes_world_access(prev_entity, *entity, world).unwrap();
//                     spawn_node_link::<RouteMarker>(
//                         world,
//                         prev_entity,
//                         *entity,
//                         meshes.cylinder.clone(),
//                         meshes.frustrum.clone(),
//                         materials.line.clone(),
//                         materials.arrow.clone(),
//                     );
//                 }
//                 prev_entity = Some(*entity);
//             }
//         }
//     });
// }

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
            // KmpSection,
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
    q_kmp_node: Query<(Entity, &KmpPathNode)>,
    mut q_transform: Query<&mut Transform>,
    q_line: Query<&KmpPathNodeLinkLine>,
    mut commands: Commands,
) {
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
    for (link_entity, kmp_node_link, children, visibility) in q_kmp_node_link.iter() {
        // don't bother unless the kmp node link is actually visible
        if *visibility == ViewVisibility::HIDDEN {
            continue;
        }

        if !nodes_to_be_linked.contains(&(kmp_node_link.prev_node, kmp_node_link.next_node)) {
            commands.entity(link_entity).despawn();
            return;
        }

        // see https://github.com/bevyengine/bevy/issues/11517
        let [prev_transform, next_transform] = q_transform
            .many_mut([kmp_node_link.prev_node, kmp_node_link.next_node])
            .map(Ref::from);

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
        for child in children {
            if q_line.get(*child).is_ok() {
                let mut line_transform = q_transform.get_mut(*child).unwrap();
                *line_transform = new_line_transform;
                break;
            }
        }
        nodes_to_be_linked.remove(&(kmp_node_link.prev_node, kmp_node_link.next_node));
    }
    for node_not_linked in nodes_to_be_linked.iter() {}
}
