use std::{ffi::OsStr, fs::File};

use crate::{
    ui::update_ui::KmpFileSelected, util::kmp_file::*, util::Cylinder, viewer::normalize::Normalize,
};
use bevy::{prelude::*, utils::HashSet};
use bevy_mod_outline::OutlineMeshExt;
use serde::{Deserialize, Serialize};

pub struct KmpPlugin;

impl Plugin for KmpPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (spawn_model, update_node_links));
        // normalize has to run after update_itpt otherwise the transform will be overwritten
        // app.add_systems(Update, (update_itpt, normalize_scale).chain());
    }
}

#[derive(Resource, Serialize, Deserialize)]
pub struct KmpModelSettings {
    pub normalize: bool,
    pub point_scale: f32,
}
impl Default for KmpModelSettings {
    fn default() -> Self {
        KmpModelSettings {
            normalize: true,
            point_scale: 1.,
        }
    }
}

#[derive(Component)]
pub struct KmpSection;

// components attached to kmp entities, to store data about them:
#[derive(Component)]
pub struct StartPoint;
#[derive(Component, Clone)]
pub struct EnemyPoint {
    pub leniency: f32,
    pub setting_1: u16,
    pub setting_2: u8,
    pub setting_3: u8,
}
#[derive(Component)]
pub struct ItemPoint {
    pub bullet_bill_control: f32,
    pub setting_1: u16,
    pub setting_2: u16,
}
#[derive(Component)]
pub struct Object;
#[derive(Component)]
pub struct AreaPoint;
#[derive(Component)]
pub struct Camera;
#[derive(Component)]
pub struct RespawnPoint;
#[derive(Component)]
pub struct CannonPoint;
#[derive(Component)]
pub struct FinishPoint;
#[derive(Component)]

// component attached to kmp entities which are linked to other kmp entities
pub struct KmpNode {
    pub prev_nodes: HashSet<Entity>,
    pub next_nodes: HashSet<Entity>,
}
#[derive(Debug)]
pub struct LinkNodeError;
impl KmpNode {
    pub fn new() -> Self {
        KmpNode {
            prev_nodes: HashSet::new(),
            next_nodes: HashSet::new(),
        }
    }
    #[allow(dead_code)]
    pub fn delete_self(&mut self, mut kmp_node_query: Query<&mut KmpNode>) {
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
        kmp_node_query: &mut Query<&mut KmpNode>,
    ) -> Result<(), LinkNodeError> {
        let mut next_node = match kmp_node_query.get_mut(next_node_entity) {
            Ok(next_node) => next_node,
            Err(_) => return Err(LinkNodeError),
        };
        next_node.prev_nodes.insert(prev_node_entity);

        let mut prev_node = match kmp_node_query.get_mut(prev_node_entity) {
            Ok(prev_node) => prev_node,
            Err(_) => return Err(LinkNodeError),
        };
        prev_node.next_nodes.insert(prev_node_entity);

        Ok(())
    }
    // link nodes if direct world access is available
    fn link_nodes_world_access(
        prev_node_entity: Entity,
        next_node_entity: Entity,
        world: &mut World,
    ) -> Result<(), LinkNodeError> {
        let mut next_node = match world.get_mut::<KmpNode>(next_node_entity) {
            Some(next_node) => next_node,
            None => return Err(LinkNodeError),
        };
        next_node.prev_nodes.insert(prev_node_entity);

        let mut prev_node = match world.get_mut::<KmpNode>(prev_node_entity) {
            Some(prev_node) => prev_node,
            None => return Err(LinkNodeError),
        };
        prev_node.next_nodes.insert(next_node_entity);
        Ok(())
    }
}

// struct to store
struct EntityGroup {
    entities: Vec<Entity>,
    next_groups: Vec<u8>,
}

#[allow(clippy::comparison_chain)]
pub fn spawn_model(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut ev_kmp_file_selected: EventReader<KmpFileSelected>,
    kmp_section_query: Query<Entity, With<KmpSection>>,
    // kmp_node_query: Query<&mut KmpNode>,
) {
    // if there is no kmp file selected event return
    let Some(ev) = ev_kmp_file_selected.read().next() else {
        return;
    };
    // if the file extension is not 'kmp' return
    if ev.0.extension() != Some(OsStr::new("kmp")) {
        return;
    }

    // open the KMP file and read it
    let kmp_file = File::open(ev.0.clone()).expect("could not open kmp file");
    let kmp = Kmp::read(kmp_file).expect("could not read kmp file");

    // despawn all kmp entities so we have a clean slate
    for entity in kmp_section_query.iter() {
        commands.entity(entity).despawn();
    }

    // meshes for the kmp model
    let mut sphere_mesh: Mesh = shape::UVSphere {
        radius: 100.,
        ..default()
    }
    .into();
    sphere_mesh.generate_outline_normals().unwrap();
    let sphere_mesh = meshes.add(sphere_mesh);

    let cylinder_mesh = meshes.add(Mesh::from(Cylinder {
        height: 1.,
        radius_bottom: 50.,
        radius_top: 50.,
        radial_segments: 32,
        height_segments: 32,
    }));
    let cone_mesh = meshes.add(Mesh::from(Cylinder {
        height: 100.,
        radius_bottom: 100.,
        radius_top: 50.,
        radial_segments: 32,
        height_segments: 32,
    }));

    // utility function for creating an unlit material of a certain colour
    let mut kmp_material_from_color = |color: Color| {
        materials.add(StandardMaterial {
            base_color: color,
            unlit: true,
            ..default()
        })
    };

    // materials
    let sphere_material = kmp_material_from_color(Color::rgb(0.0, 0.6, 0.0));
    let line_material = kmp_material_from_color(Color::rgb(0.2, 1.0, 0.2));
    let arrow_material = kmp_material_from_color(Color::WHITE);

    // --- START POINTS ---
    let start_point_material = kmp_material_from_color(Color::rgb(1., 0., 0.0));

    for start_point in kmp.ktpt.entries.iter() {
        commands.spawn((
            PbrBundle {
                mesh: sphere_mesh.clone(),
                material: start_point_material.clone(),
                transform: Transform::from_translation(start_point.position),
                ..default()
            },
            StartPoint,
            KmpSection,
            Normalize::new(200., 12., BVec3::TRUE),
        ));
    }

    // --- ENEMY POINTS ---

    let enemy_point_material = kmp_material_from_color(Color::rgb(0., 1., 0.0));

    struct EnptGroup {
        nodes: Vec<Enpt>,
        next_groups: Vec<u8>,
    }

    let mut enemy_path_groups: Vec<EnptGroup> = Vec::new();

    for group in kmp.enph.entries.iter() {
        let mut next_groups = Vec::new();
        for next_group in group.next_group {
            if next_group != 0xff {
                next_groups.push(next_group);
            }
        }
        let mut nodes = Vec::new();
        for i in group.start..(group.start + group.group_length) {
            let enemy_point = &kmp.enpt.entries[i as usize];
            nodes.push(enemy_point.clone());
        }
        enemy_path_groups.push(EnptGroup { nodes, next_groups });
    }

    let sphere_mesh_clone = sphere_mesh.clone();

    commands.add(move |world: &mut World| {
        // spawn all the entities, saving the entity IDs into 'entity_groups'
        let mut entity_groups: Vec<EntityGroup> = Vec::new();
        for group in enemy_path_groups {
            let mut entity_group = EntityGroup {
                entities: Vec::new(),
                next_groups: group.next_groups,
            };
            for node in group.nodes {
                let spawned_entity = world.spawn((
                    PbrBundle {
                        mesh: sphere_mesh_clone.clone(),
                        material: enemy_point_material.clone(),
                        transform: Transform::from_translation(node.position),
                        ..default()
                    },
                    KmpNode::new(),
                    EnemyPoint {
                        leniency: node.leniency,
                        setting_1: node.setting_1,
                        setting_2: node.setting_2,
                        setting_3: node.setting_3,
                    },
                    KmpSection,
                    Normalize::new(200., 12., BVec3::TRUE),
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
                    KmpNode::link_nodes_world_access(prev_entity, *entity, world).unwrap();
                    spawn_node_link(
                        cylinder_mesh.clone(),
                        line_material.clone(),
                        cone_mesh.clone(),
                        arrow_material.clone(),
                        world,
                        prev_entity,
                        *entity,
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
                KmpNode::link_nodes_world_access(entity, next_entity, world).unwrap();
                spawn_node_link(
                    cylinder_mesh.clone(),
                    line_material.clone(),
                    cone_mesh.clone(),
                    arrow_material.clone(),
                    world,
                    entity,
                    next_entity,
                );
            }
        }
    });

    // --- ITEM POINTS ---

    let item_point_material = kmp_material_from_color(Color::rgb(0., 0., 1.));

    struct ItptGroup {
        nodes: Vec<Itpt>,
        next_groups: Vec<u8>,
    }

    let mut item_path_groups: Vec<ItptGroup> = Vec::new();

    for group in kmp.itph.entries.iter() {
        let mut next_groups = Vec::new();
        for next_group in group.next_group {
            if next_group != 0xff {
                next_groups.push(next_group);
            }
        }
        let mut nodes = Vec::new();
        for i in group.start..(group.start + group.group_length) {
            let enemy_point = &kmp.itpt.entries[i as usize];
            nodes.push(enemy_point.clone());
        }
        item_path_groups.push(ItptGroup { nodes, next_groups });
    }

    let sphere_mesh_clone = sphere_mesh.clone();

    commands.add(move |world: &mut World| {
        let mut entity_groups: Vec<EntityGroup> = Vec::new();
        for group in item_path_groups {
            let mut entity_group = EntityGroup {
                entities: Vec::new(),
                next_groups: group.next_groups,
            };
            for node in group.nodes {
                let spawned_entity = world.spawn((
                    PbrBundle {
                        mesh: sphere_mesh_clone.clone(),
                        material: item_point_material.clone(),
                        transform: Transform::from_translation(node.position),
                        ..default()
                    },
                    KmpNode::new(),
                    ItemPoint {
                        bullet_bill_control: node.bullet_bill_control,
                        setting_1: node.setting_1,
                        setting_2: node.setting_2,
                    },
                    KmpSection,
                    Normalize::new(200., 12., BVec3::TRUE),
                ));
                entity_group.entities.push(spawned_entity.id());
            }
            entity_groups.push(entity_group);
        }
        for group in entity_groups.iter() {
            let mut prev_entity: Option<Entity> = None;
            // in each group, link the previous node to the current node
            for entity in group.entities.iter() {
                if let Some(prev_entity) = prev_entity {
                    KmpNode::link_nodes_world_access(prev_entity, *entity, world).unwrap();
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
                KmpNode::link_nodes_world_access(entity, next_entity, world).unwrap();
            }
        }
    });

    // --- CHECKPOINTS ---

    // --- OBJECTS ---

    for object in kmp.gobj.entries.iter() {
        commands.spawn((
            PbrBundle {
                mesh: sphere_mesh.clone(),
                material: sphere_material.clone(),
                transform: Transform::from_translation(object.position),
                ..default()
            },
            Object,
            KmpSection,
            Normalize::new(200., 12., BVec3::TRUE),
        ));
    }

    // --- ROUTES ---

    // --- AREAS ---

    // --- CAMREAS ---

    // --- RESPAWN POINTS ---

    // --- CANNON POINTS ---

    // --- FINISH POINTS ---

    // --- STAGE INFO ---
}

#[derive(Component)]
struct KmpNodeLink {
    prev_node: Entity,
    next_node: Entity,
    line_child: Option<Entity>,
}

fn spawn_node_link(
    cylinder_mesh: Handle<Mesh>,
    cylinder_material: Handle<StandardMaterial>,
    cone_mesh: Handle<Mesh>,
    cone_material: Handle<StandardMaterial>,
    world: &mut World,

    prev_node: Entity,
    next_node: Entity,
) {
    let prev_pos = world.get::<Transform>(prev_node).unwrap().translation;
    let next_pos = world.get::<Transform>(next_node).unwrap().translation;

    let mut parent_transform =
        Transform::from_translation(prev_pos.lerp(next_pos, 0.5)).looking_at(next_pos, Vec3::Y);
    parent_transform.rotate_local_x(f32::to_radians(-90.));

    let mut line_transform = Transform::default();
    line_transform.scale.y = prev_pos.distance(next_pos);

    let mut line_child: Option<Entity> = None;

    let parent = world
        // spawn a parent component which contains a transform, and stores the entities of the nodes the node links
        .spawn((
            TransformBundle::from_transform(parent_transform),
            VisibilityBundle::default(),
            KmpNodeLink {
                prev_node,
                next_node,
                line_child: None,
            },
            KmpSection,
        ))
        // spawn the line and arrow as children of this parent component, which will inherit its transform
        .with_children(|parent| {
            line_child = Some(
                parent
                    .spawn((
                        PbrBundle {
                            mesh: cylinder_mesh,
                            material: cylinder_material,
                            transform: line_transform,
                            ..default()
                        },
                        KmpSection,
                        Normalize::new(200., 12., BVec3::new(true, false, true)),
                    ))
                    .id(),
            );
            parent.spawn((
                PbrBundle {
                    mesh: cone_mesh,
                    material: cone_material,
                    ..default()
                },
                KmpSection,
                Normalize::new(200., 12., BVec3::TRUE),
            ));
        })
        .id();
    world.get_mut::<KmpNodeLink>(parent).unwrap().line_child = line_child;
}

fn update_node_links(
    kmp_node_link_query: Query<(Entity, &KmpNodeLink)>,
    mut transform_query: Query<&mut Transform>,
) {
    // go through each node line
    for (entity, kmp_node_link) in kmp_node_link_query.iter() {
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

        // set the transform of the child
        let mut line_transform = transform_query
            .get_mut(kmp_node_link.line_child.unwrap())
            .unwrap();
        *line_transform = new_line_transform;
    }
}
