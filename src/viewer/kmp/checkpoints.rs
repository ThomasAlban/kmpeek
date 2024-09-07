use super::{
    calc_cp_arrow_transform, calc_line_transform,
    meshes_materials::{CheckpointMaterials, KmpMeshes},
    ordering::{NextOrderID, OrderId},
    path::{get_kmp_data_and_component_groups, link_entity_groups, EntityGroup, KmpPathNode},
    Checkpoint, CheckpointKind, CheckpointMarker, KmpFile, KmpSectionIdEntityMap, KmpSelectablePoint, PathOverallStart,
    RespawnPoint, TransformEditOptions,
};
use crate::{
    ui::settings::AppSettings,
    util::try_despawn,
    viewer::{
        edit::{
            select::Selected,
            transform_gizmo::GizmoTransformable,
            tweak::{SnapTo, Tweakable},
        },
        normalize::{Normalize, NormalizeInheritParent},
    },
};
use bevy::{
    ecs::{
        entity::{EntityHashMap, EntityHashSet},
        system::SystemParam,
    },
    math::vec3,
    prelude::*,
    transform::TransformSystem,
};
use bevy_mod_outline::{OutlineBundle, OutlineVolume};
use bon::builder;

pub fn checkpoint_plugin(app: &mut App) {
    app.init_resource::<CheckpointHeight>()
        .add_systems(
            Update,
            (
                set_checkpoint_right_visibility,
                update_checkpoint_lines_arrows,
                update_checkpoint_planes,
                update_checkpoint_colors,
            ),
        )
        .add_systems(
            PostUpdate,
            set_checkpoint_node_height.after(TransformSystem::TransformPropagate),
        )
        // .add_systems(
        //     Update,
        //     |q1: Query<(), (With<CpArrowParent>, With<Normalize>)>,
        //      q2: Query<(), (With<CpArrowChild>, With<NormalizeInheritParent>)>| {
        //         dbg!(q1.iter().len());
        //         dbg!(q2.iter().len());
        //     },
        // )
        .observe(on_remove_cp_left)
        .observe(on_remove_cp_right);
}

#[derive(Component)]
pub struct CpArrowParent;
#[derive(Component)]
pub struct CpArrowChild;

#[derive(Component, Clone, PartialEq, Debug)]
pub struct CheckpointLeft {
    pub right: Entity,
    pub line: Entity,
    pub plane: Entity,
    pub arrow: Entity,
}

impl Default for CheckpointLeft {
    fn default() -> Self {
        Self {
            right: Entity::PLACEHOLDER,
            line: Entity::PLACEHOLDER,
            plane: Entity::PLACEHOLDER,
            arrow: Entity::PLACEHOLDER,
        }
    }
}
#[derive(Component, Clone, PartialEq)]
pub struct CheckpointRight {
    pub left: Entity,
    pub line: Entity,
    pub plane: Entity,
}

impl Default for CheckpointRight {
    fn default() -> Self {
        Self {
            left: Entity::PLACEHOLDER,
            line: Entity::PLACEHOLDER,
            plane: Entity::PLACEHOLDER,
        }
    }
}
#[derive(Component)]
pub struct CheckpointLine {
    pub left: Entity,
    pub right: Entity,
    pub arrow: Entity,
}
#[derive(Component)]
pub struct CheckpointPlane {
    pub left: Entity,
    pub right: Entity,
}

#[derive(Component, PartialEq, Clone, Copy, Deref, DerefMut)]
pub struct CheckpointRespawnLink(pub Entity);

fn calc_cp_plane_transform(left: Vec2, right: Vec2, height: f32) -> Transform {
    // lerp btw left and right pos with half the height as y
    let pos = left.lerp(right, 0.5).extend(height / 2.).xzy();
    let dir = (left - right).perp().normalize().extend(height).xzy();

    Transform::from_translation(pos)
        .looking_to(dir, Vec3::Y)
        .with_scale(vec3(left.distance(right), 1., pos.y * 2.))
}

const DEFAULT_CP_HEIGHT: f32 = 15000.;

#[derive(Resource, Deref, DerefMut)]
pub struct CheckpointHeight(pub f32);

impl Default for CheckpointHeight {
    fn default() -> Self {
        Self(DEFAULT_CP_HEIGHT)
    }
}

fn on_remove_cp_left(
    trigger: Trigger<OnRemove, CheckpointLeft>,
    q_cp_left: Query<&CheckpointLeft>,
    mut commands: Commands,
) {
    let cp_left = q_cp_left.get(trigger.entity()).unwrap();
    let cp_right = cp_left.right;

    try_despawn(&mut commands, cp_right);
    try_despawn(&mut commands, cp_left.line);
    try_despawn(&mut commands, cp_left.plane);
    try_despawn(&mut commands, cp_left.arrow);
}

fn on_remove_cp_right(
    trigger: Trigger<OnRemove, CheckpointRight>,
    q_cp_right: Query<&CheckpointRight>,
    mut commands: Commands,
) {
    let cp_right = q_cp_right.get(trigger.entity()).unwrap();
    let cp_left = cp_right.left;

    try_despawn(&mut commands, cp_left);
}

#[builder]
pub fn checkpoint_spawner(
    world: &mut World,
    cp: Checkpoint,
    #[builder(default)] pos: (Vec2, Vec2),
    visible: Option<bool>,
    #[builder(default = DEFAULT_CP_HEIGHT)] height: f32,
    order_id: Option<u32>,
    right_e: Option<Entity>,
) -> (Entity, Entity) {
    let (left_pos, right_pos) = (pos.0, pos.1);
    let left_transform = Transform::from_xyz(left_pos.x, height, left_pos.y);
    let right_transform = Transform::from_xyz(right_pos.x, height, right_pos.y);
    let left_tr = left_transform.translation;
    let right_tr = right_transform.translation;

    let line_transform = calc_line_transform(left_tr, right_tr);

    let meshes = world.resource::<KmpMeshes>();
    let (sphere_mesh, cylinder_mesh, cone_mesh, plane_mesh) = (
        meshes.sphere.clone(),
        meshes.cylinder.clone(),
        meshes.cone.clone(),
        meshes.plane.clone(),
    );
    let cp_materials = world.resource::<CheckpointMaterials>();
    let (material, material_plane) = match cp.kind {
        CheckpointKind::Normal => (cp_materials.normal.clone(), cp_materials.normal_plane.clone()),
        CheckpointKind::Key(_) => (cp_materials.key.clone(), cp_materials.key_plane.clone()),
        CheckpointKind::LapCount => (cp_materials.lap_count.clone(), cp_materials.lap_count_plane.clone()),
    };

    let outline = world.resource::<AppSettings>().kmp_model.outline;

    // either gets the order id, or gets it from the NextOrderID (which will increment it for next time)
    let order_id = order_id.unwrap_or_else(|| world.resource::<NextOrderID<Checkpoint>>().get());

    let visibility = if visible.unwrap_or(true) {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };

    let left_e = world.spawn_empty().id();
    let right_e = right_e.unwrap_or_else(|| world.spawn_empty().id());

    let line_e = world.spawn_empty().id();
    let arrow_e = world.spawn_empty().id();
    let plane_e = world.spawn_empty().id();

    let cp_bundle = || {
        (
            KmpSelectablePoint,
            Tweakable(SnapTo::CheckpointPlane),
            TransformEditOptions::new(true, true),
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
            KmpPathNode::default(),
            CheckpointMarker,
        )
    };

    // spawn the left of the checkpoint
    world.entity_mut(left_e).insert((
        PbrBundle {
            mesh: sphere_mesh.clone(),
            material: material.clone(),
            transform: left_transform,
            visibility,
            ..default()
        },
        cp.clone(),
        CheckpointLeft {
            right: right_e,
            line: line_e,
            plane: plane_e,
            arrow: arrow_e,
        },
        OrderId(order_id),
        cp_bundle(),
    ));

    // spawn the right of the checkpoint
    world.entity_mut(right_e).insert((
        PbrBundle {
            mesh: sphere_mesh,
            material: material.clone(),
            transform: right_transform,
            visibility,
            ..default()
        },
        CheckpointRight {
            left: left_e,
            line: line_e,
            plane: plane_e,
        },
        cp_bundle(),
    ));

    // spawn the line
    world.get_entity_mut(line_e).unwrap().insert((
        PbrBundle {
            mesh: cylinder_mesh,
            material: material.clone(),
            transform: line_transform,
            visibility,
            ..default()
        },
        Normalize::new(200., 30., BVec3::new(true, false, true)),
        CheckpointLine {
            left: left_e,
            right: right_e,
            arrow: arrow_e,
        },
    ));

    // spawn the arrow
    let arrow_parent_transform = calc_cp_arrow_transform(left_tr, right_tr);
    // basically got these values from trial and error
    let arrow_child_transform = Transform::from_translation(vec3(0., 75., 0.)).with_scale(vec3(0.4, 1., 1.));
    world
        .get_entity_mut(arrow_e)
        .unwrap()
        .insert((
            SpatialBundle {
                visibility: Visibility::Visible,
                transform: arrow_parent_transform,
                ..default()
            },
            CpArrowParent,
            Normalize::new(200., 30., BVec3::TRUE),
        ))
        .with_children(|parent| {
            parent.spawn((
                PbrBundle {
                    mesh: cone_mesh,
                    material,
                    transform: arrow_child_transform,
                    ..default()
                },
                CpArrowChild,
                NormalizeInheritParent,
            ));
        });

    // spawn the plane
    let transform = calc_cp_plane_transform(left_pos, right_pos, height);
    world.entity_mut(plane_e).insert((
        PbrBundle {
            mesh: plane_mesh,
            material: material_plane,
            transform,
            visibility,
            ..default()
        },
        CheckpointPlane {
            left: left_e,
            right: right_e,
        },
    ));

    (left_e, right_e)
}

pub fn spawn_checkpoint_section(world: &mut World, kmp: &KmpFile) {
    let kmp_groups = get_kmp_data_and_component_groups::<Checkpoint>(kmp, world);

    let mut left_entity_groups: Vec<EntityGroup> = Vec::with_capacity(kmp_groups.len());
    let mut right_entity_groups = left_entity_groups.clone();
    let mut acc = 0;
    for (i, (data_group, component_group)) in kmp_groups.iter().enumerate() {
        let mut left_entity_group = EntityGroup {
            entities: Vec::with_capacity(data_group.nodes.len()),
            next_groups: data_group.next_groups.clone(),
        };
        let mut right_entity_group = left_entity_group.clone();
        for (j, node) in data_group.nodes.iter().enumerate() {
            let kmp_component = component_group[j].clone();
            let (left, right) = checkpoint_spawner()
                .cp(kmp_component)
                .pos((node.cp_left.into(), node.cp_right.into()))
                .visible(false)
                .height(**world.resource::<CheckpointHeight>())
                .order_id(acc as u32)
                .world(world)
                .call();
            if i == 0 && j == 0 {
                world.entity_mut(left).insert(PathOverallStart);
            }
            let maybe_respawn_e = world
                .resource::<KmpSectionIdEntityMap<RespawnPoint>>()
                .get(&(node.respawn_pos as u32))
                .copied();
            if let Some(respawn_e) = maybe_respawn_e {
                world.entity_mut(left).insert(CheckpointRespawnLink(respawn_e));
            }
            left_entity_group.entities.push(left);
            right_entity_group.entities.push(right);
            acc += 1;
        }
        left_entity_groups.push(left_entity_group);
        right_entity_groups.push(right_entity_group);
    }
    link_entity_groups(world, left_entity_groups);
    link_entity_groups(world, right_entity_groups);
}

fn set_checkpoint_right_visibility(
    q_cp_left: Query<(Ref<Visibility>, &CheckpointLeft)>,
    mut q_visibility: Query<&mut Visibility, Without<CheckpointLeft>>,
) {
    for (left_vis, cp_left) in q_cp_left.iter() {
        if !left_vis.is_changed() {
            continue;
        }
        let Ok(mut right_vis) = q_visibility.get_mut(cp_left.right) else {
            continue;
        };
        *right_vis = *left_vis;
    }
}

fn set_checkpoint_node_height(
    mut q_cp: Query<&mut Transform, Or<(With<Checkpoint>, With<CheckpointRight>)>>,
    cp_height: Res<CheckpointHeight>,
) {
    for mut cp in q_cp.iter_mut() {
        if cp.is_changed() {
            cp.translation.y = cp_height.0;
        }
    }
}

fn update_checkpoint_colors(
    q_cp_left: Query<(Ref<Checkpoint>, &CheckpointLeft, Entity)>,
    mut q_std_mat: Query<&mut Handle<StandardMaterial>>,
    q_children: Query<&Children>,
    materials: Res<CheckpointMaterials>,
) {
    for (cp, cp_left, cp_e) in q_cp_left.iter() {
        if !cp.is_changed() {
            continue;
        }

        let point_material = match cp.kind {
            CheckpointKind::Normal => materials.normal.clone(),
            CheckpointKind::Key(_) => materials.key.clone(),
            CheckpointKind::LapCount => materials.lap_count.clone(),
        };
        let plane_material = match cp.kind {
            CheckpointKind::Normal => materials.normal_plane.clone(),
            CheckpointKind::Key(_) => materials.key_plane.clone(),
            CheckpointKind::LapCount => materials.lap_count_plane.clone(),
        };

        let arrow = q_children.get(cp_left.arrow).unwrap().first().unwrap();

        *q_std_mat.get_mut(cp_e).unwrap() = point_material.clone();
        *q_std_mat.get_mut(cp_left.right).unwrap() = point_material.clone();
        *q_std_mat.get_mut(cp_left.line).unwrap() = point_material.clone();
        *q_std_mat.get_mut(*arrow).unwrap() = point_material.clone();
        *q_std_mat.get_mut(cp_left.plane).unwrap() = plane_material;
    }
}

fn update_checkpoint_lines_arrows(
    mut q_cp_line: Query<(&CheckpointLine, &mut Transform, &mut Visibility)>,
    mut q_cp_part: Query<(&mut Transform, &mut Visibility), Without<CheckpointLine>>,
) {
    for (line, mut line_trans, mut line_vis) in q_cp_line.iter_mut() {
        let Ok([(l_trans, l_vis), (r_trans, _), (mut a_trans, mut a_vis)]) =
            q_cp_part.get_many_mut([line.left, line.right, line.arrow])
        else {
            continue;
        };
        // set the visibility
        line_vis.set_if_neq(*l_vis);
        a_vis.set_if_neq(*l_vis);
        if !l_trans.is_changed() && !r_trans.is_changed() {
            continue;
        }
        *line_trans = calc_line_transform(l_trans.translation, r_trans.translation);
        *a_trans = calc_cp_arrow_transform(l_trans.translation, r_trans.translation);
    }
}
// the same as above but for checkpoint planes instead of lines
fn update_checkpoint_planes(
    mut q_cp_plane: Query<(&CheckpointPlane, &mut Transform, &mut Visibility)>,
    q_cp_node: Query<(Ref<Transform>, &Visibility), Without<CheckpointPlane>>,
    cp_height: Res<CheckpointHeight>,
) {
    for (plane, mut plane_trans, mut plane_vis) in q_cp_plane.iter_mut() {
        let Ok([(l_trans, l_vis), (r_trans, _)]) = q_cp_node.get_many([plane.left, plane.right]) else {
            continue;
        };
        plane_vis.set_if_neq(*l_vis);
        if !l_trans.is_changed() && !r_trans.is_changed() {
            continue;
        }
        let new_plane_trans = calc_cp_plane_transform(l_trans.translation.xz(), r_trans.translation.xz(), cp_height.0);
        *plane_trans = new_plane_trans;
    }
}

#[derive(SystemParam)]
pub struct GetSelectedCheckpoints<'w, 's> {
    q_cp_left: Query<'w, 's, (&'static mut Checkpoint, Entity, Has<Selected>)>,
    q_cp_right: Query<'w, 's, &'static mut CheckpointRight, With<Selected>>,
}
impl GetSelectedCheckpoints<'_, '_> {
    pub fn get(&mut self) -> EntityHashMap<Mut<Checkpoint>> {
        let cp_left_of_right: EntityHashSet = self.q_cp_right.iter().map(|x| x.left).collect();
        let mut cps: EntityHashMap<Mut<Checkpoint>> = EntityHashMap::default();
        for (cp_l, e, selected) in self.q_cp_left.iter_mut() {
            if selected || cp_left_of_right.contains(&e) {
                cps.insert(e, cp_l);
            }
        }
        cps
    }
    pub fn get_entities(&self) -> EntityHashSet {
        let cp_left_of_right: EntityHashSet = self.q_cp_right.iter().map(|x| x.left).collect();
        let mut cps = EntityHashSet::default();
        for (_, e, selected) in self.q_cp_left.iter() {
            if selected || cp_left_of_right.contains(&e) {
                cps.insert(e);
            }
        }
        cps
    }
}

/// Utility for getting both checkpoint nodes when we only have the Entity ID of one of them, and don't know whether the one we have is a left or a right
pub fn get_both_cp_nodes(world: &mut World, e: Entity) -> (Entity, Entity) {
    let left = if world.entity(e).contains::<Checkpoint>() {
        e
    } else {
        world.entity(e).get::<CheckpointRight>().unwrap().left
    };
    let right = if world.entity(e).contains::<CheckpointRight>() {
        e
    } else {
        world.entity(e).get::<CheckpointLeft>().unwrap().right
    };
    (left, right)
}
