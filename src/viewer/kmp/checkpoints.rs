use super::{
    calc_cp_arrow_transform, calc_line_transform,
    meshes_materials::{CheckpointMaterials, KmpMeshes},
    ordering::{NextOrderID, OrderID},
    path::{get_kmp_data_and_component_groups, link_entity_groups, EntityGroup, KmpPathNode},
    Checkpoint, CheckpointKind, CheckpointLine, CheckpointPlane, CheckpointRight, Ckpt, KmpError, KmpFile,
    KmpSelectablePoint, PathOverallStart, TransformEditOptions,
};
use crate::{
    ui::settings::AppSettings,
    util::BoolToVisibility,
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
    ecs::entity::{EntityHashMap, EntityHashSet},
    math::vec3,
    prelude::*,
    transform::TransformSystem,
};
use bevy_mod_outline::{OutlineBundle, OutlineVolume};
use std::sync::Arc;

pub struct CheckpointPlugin;
impl Plugin for CheckpointPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CheckpointHeight>()
            .add_systems(
                Update,
                (
                    set_checkpoint_right_visibility,
                    update_checkpoint_lines,
                    update_checkpoint_planes,
                    update_checkpoint_colors,
                ),
            )
            .add_systems(
                PostUpdate,
                set_checkpoint_node_height.after(TransformSystem::TransformPropagate),
            );
    }
}

fn calc_cp_plane_transform(left: Vec2, right: Vec2, height: f32) -> Transform {
    // lerp btw left and right pos with half the height as y
    let pos = left.lerp(right, 0.5).extend(height / 2.).xzy();
    let dir = (left - right).perp().normalize().extend(height).xzy();

    Transform::from_translation(pos)
        .looking_to(dir, Vec3::Y)
        .with_scale(vec3(left.distance(right) / 2., 1., pos.y))
}

const DEFAULT_CP_HEIGHT: f32 = 15000.;

#[derive(Resource)]
pub struct CheckpointHeight(pub f32);

impl Default for CheckpointHeight {
    fn default() -> Self {
        Self(DEFAULT_CP_HEIGHT)
    }
}

pub struct CheckpointSpawner {
    kmp_component: Checkpoint,
    left_pos: Vec2,
    right_pos: Vec2,
    height: f32,
    visible: bool,
    order_id: Option<u32>,
    left_e: Option<Entity>,
    right_e: Option<Entity>,
}
impl CheckpointSpawner {
    pub fn new(kmp_component: Checkpoint) -> Self {
        Self {
            kmp_component,
            left_pos: Vec2::default(),
            right_pos: Vec2::default(),
            height: DEFAULT_CP_HEIGHT,
            visible: true,
            order_id: None,
            left_e: None,
            right_e: None,
        }
    }
    pub fn single_3d_pos(mut self, pos: Vec3) -> Self {
        self.left_pos = pos.xz();
        self.right_pos = pos.xz();
        self.height = pos.y;
        self
    }
    pub fn pos(mut self, left: Vec2, right: Vec2) -> Self {
        self.left_pos = left;
        self.right_pos = right;
        self
    }
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }
    pub fn order_id(mut self, id: u32) -> Self {
        self.order_id = Some(id);
        self
    }
    pub fn left_transform(&self) -> Transform {
        Transform::from_xyz(self.left_pos.x, self.height, self.left_pos.y)
    }
    pub fn right_transform(&self) -> Transform {
        Transform::from_xyz(self.right_pos.x, self.height, self.right_pos.y)
    }

    fn spawn_arrow(&self, world: &mut World, entity: Entity) {
        let l_tr = self.left_transform().translation;
        let r_tr = self.right_transform().translation;
        let parent_transform = calc_cp_arrow_transform(l_tr, r_tr);

        // basically got these values from trial and error
        let child_transform = Transform::from_translation(vec3(0., 75., 0.)).with_scale(vec3(0.4, 1., 1.));

        let mesh = world.resource::<KmpMeshes>().cone.clone();
        let cp_materials = world.resource::<CheckpointMaterials>();
        let material = match self.kmp_component.kind {
            CheckpointKind::Normal => cp_materials.normal.clone(),
            CheckpointKind::Key => cp_materials.key.clone(),
            CheckpointKind::LapCount => cp_materials.lap_count.clone(),
        };

        world
            .get_entity_mut(entity)
            .unwrap()
            .insert((
                SpatialBundle {
                    visibility: self.visible.to_visibility(),
                    transform: parent_transform,
                    ..default()
                },
                Normalize::new(200., 30., BVec3::TRUE),
            ))
            .with_children(|parent| {
                parent.spawn((
                    PbrBundle {
                        mesh,
                        material,
                        transform: child_transform,
                        ..default()
                    },
                    NormalizeInheritParent,
                ));
            });
    }

    fn spawn_line(&self, world: &mut World, entity: Entity, arrow_entity: Entity) {
        let l_tr = self.left_transform().translation;
        let r_tr = self.right_transform().translation;
        let line_transform = calc_line_transform(l_tr, r_tr);

        let mesh = world.resource::<KmpMeshes>().cylinder.clone();
        let cp_materials = world.resource::<CheckpointMaterials>();
        let material = match self.kmp_component.kind {
            CheckpointKind::Normal => cp_materials.normal.clone(),
            CheckpointKind::Key => cp_materials.key.clone(),
            CheckpointKind::LapCount => cp_materials.lap_count.clone(),
        };
        world.get_entity_mut(entity).unwrap().insert((
            PbrBundle {
                mesh,
                material,
                transform: line_transform,
                visibility: self.visible.to_visibility(),
                ..default()
            },
            Normalize::new(200., 30., BVec3::new(true, false, true)),
            CheckpointLine {
                left: self.left_e.unwrap(),
                right: self.right_e.unwrap(),
                arrow: arrow_entity,
            },
        ));
    }

    fn spawn_plane(&self, world: &mut World, entity: Entity) {
        let mesh = world.resource::<KmpMeshes>().plane.clone();
        let cp_materials = world.resource::<CheckpointMaterials>();
        let material = match self.kmp_component.kind {
            CheckpointKind::Normal => cp_materials.normal_plane.clone(),
            CheckpointKind::Key => cp_materials.key_plane.clone(),
            CheckpointKind::LapCount => cp_materials.lap_count_plane.clone(),
        };
        let transform = calc_cp_plane_transform(self.left_pos, self.right_pos, self.height);

        world.entity_mut(entity).insert((
            PbrBundle {
                mesh,
                material,
                transform,
                visibility: Visibility::Visible,
                ..default()
            },
            CheckpointPlane {
                left: self.left_e.unwrap(),
                right: self.right_e.unwrap(),
            },
        ));
    }

    pub fn spawn_command(mut self, commands: &mut Commands) -> (Entity, Entity) {
        let left = self.left_e.unwrap_or_else(|| commands.spawn_empty().id());
        let right = self.right_e.unwrap_or_else(|| commands.spawn_empty().id());
        self.left_e = Some(left);
        self.right_e = Some(right);

        commands.add(move |world: &mut World| {
            self.spawn(world);
        });
        (left, right)
    }

    pub fn spawn(mut self, world: &mut World) -> (Entity, Entity) {
        let mesh = world.resource::<KmpMeshes>().sphere.clone();
        let cp_materials = world.resource::<CheckpointMaterials>();
        let material = match self.kmp_component.kind {
            CheckpointKind::Normal => cp_materials.normal.clone(),
            CheckpointKind::Key => cp_materials.key.clone(),
            CheckpointKind::LapCount => cp_materials.lap_count.clone(),
        };
        let outline = world.resource::<AppSettings>().kmp_model.outline.clone();

        // either gets the order id, or gets it from the NextOrderID (which will increment it for next time)
        let order_id = self
            .order_id
            .unwrap_or_else(|| world.resource::<NextOrderID<Checkpoint>>().get());

        let left = self.left_e.unwrap_or_else(|| world.spawn_empty().id());
        let right = self.right_e.unwrap_or_else(|| world.spawn_empty().id());
        self.left_e = Some(left);
        self.right_e = Some(right);

        let line = world.spawn_empty().id();
        let arrow = world.spawn_empty().id();
        let plane = world.spawn_empty().id();

        let cp_bundle = || {
            (
                KmpSelectablePoint,
                Tweakable(SnapTo::CheckpointPlane),
                TransformEditOptions {
                    hide_rotation: true,
                    hide_y_translation: true,
                },
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
            )
        };

        world.entity_mut(left).insert((
            PbrBundle {
                mesh: mesh.clone(),
                material: material.clone(),
                transform: Transform::from_translation(vec3(self.left_pos.x, self.height, self.left_pos.y)),
                visibility: if self.visible {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                },
                ..default()
            },
            Checkpoint {
                right,
                line,
                plane,
                arrow,
                ..self.kmp_component.clone()
            },
            OrderID(order_id),
            cp_bundle(),
        ));

        world.entity_mut(right).insert((
            PbrBundle {
                mesh,
                material,
                transform: Transform::from_translation(vec3(self.right_pos.x, self.height, self.right_pos.y)),
                visibility: if self.visible {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                },
                ..default()
            },
            CheckpointRight { left, line, plane },
            cp_bundle(),
        ));

        self.spawn_line(world, line, arrow);
        self.spawn_arrow(world, arrow);
        self.spawn_plane(world, plane);

        (left, right)
    }
}

pub fn spawn_checkpoint_section(
    commands: &mut Commands,
    kmp: Arc<KmpFile>,
    kmp_errors: &mut Vec<KmpError>,
    height: f32,
) {
    let kmp_groups = get_kmp_data_and_component_groups::<Ckpt, Checkpoint>(kmp, kmp_errors);

    commands.add(move |world: &mut World| {
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
                let (left, right) = CheckpointSpawner::new(kmp_component)
                    .pos(node.cp_left.into(), node.cp_right.into())
                    .visible(false)
                    .height(height)
                    .order_id(acc)
                    .spawn(world);
                if i == 0 && j == 0 {
                    world.entity_mut(left).insert(PathOverallStart);
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
    });
}

fn set_checkpoint_right_visibility(
    q_cp_left: Query<(Ref<Visibility>, &Checkpoint)>,
    mut q_visibility: Query<&mut Visibility, Without<Checkpoint>>,
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
    q_cp_left: Query<(Ref<Checkpoint>, Entity)>,
    mut q_std_mat: Query<&mut Handle<StandardMaterial>>,
    q_children: Query<&Children>,
    materials: Res<CheckpointMaterials>,
) {
    for (cp, cp_left_e) in q_cp_left.iter() {
        if !cp.is_changed() {
            continue;
        }

        let point_material = match cp.kind {
            CheckpointKind::Normal => materials.normal.clone(),
            CheckpointKind::Key => materials.key.clone(),
            CheckpointKind::LapCount => materials.lap_count.clone(),
        };
        let plane_material = match cp.kind {
            CheckpointKind::Normal => materials.normal_plane.clone(),
            CheckpointKind::Key => materials.key_plane.clone(),
            CheckpointKind::LapCount => materials.lap_count_plane.clone(),
        };

        let arrow = q_children.get(cp.arrow).unwrap().first().unwrap();

        *q_std_mat.get_mut(cp_left_e).unwrap() = point_material.clone();
        *q_std_mat.get_mut(cp.right).unwrap() = point_material.clone();
        *q_std_mat.get_mut(cp.line).unwrap() = point_material.clone();
        *q_std_mat.get_mut(*arrow).unwrap() = point_material.clone();
        *q_std_mat.get_mut(cp.plane).unwrap() = plane_material;
    }
}

fn update_checkpoint_lines(
    mut commands: Commands,
    mut q_cp_line: Query<(&CheckpointLine, &mut Transform, Entity, &mut Visibility)>,
    mut q_cp_part: Query<(&mut Transform, &mut Visibility), Without<CheckpointLine>>,
) {
    for (line, mut line_trans, line_e, mut line_vis) in q_cp_line.iter_mut() {
        let Ok([(l_trans, l_vis), (r_trans, _), (mut a_trans, mut a_vis)]) =
            q_cp_part.get_many_mut([line.left, line.right, line.arrow])
        else {
            // despawn the line and the arrow if either of the nodes doesn't exist
            if let Some(line) = commands.get_entity(line_e) {
                line.despawn_recursive();
            }
            if let Some(arrow) = commands.get_entity(line.arrow) {
                arrow.despawn_recursive();
            }
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
    mut commands: Commands,
    mut q_cp_plane: Query<(&CheckpointPlane, &mut Transform, Entity, &mut Visibility)>,
    q_cp_node: Query<(Ref<Transform>, &Visibility), Without<CheckpointPlane>>,
    cp_height: Res<CheckpointHeight>,
) {
    for (plane, mut plane_trans, plane_e, mut plane_vis) in q_cp_plane.iter_mut() {
        let Ok([(l_trans, l_vis), (r_trans, _)]) = q_cp_node.get_many([plane.left, plane.right]) else {
            commands.entity(plane_e).despawn();
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

pub fn get_selected_cp_lefts<'a>(
    q_cp_left: &'a mut Query<(&mut Checkpoint, Entity, Has<Selected>)>,
    q_cp_right: &'a mut Query<&mut CheckpointRight, With<Selected>>,
) -> impl Iterator<Item = (Entity, Mut<'a, Checkpoint>)> {
    let cp_left_of_right: EntityHashSet = q_cp_right.iter().map(|x| x.left).collect();
    let mut cps: EntityHashMap<Mut<Checkpoint>> = EntityHashMap::default();
    for (cp_l, e, selected) in q_cp_left.iter_mut() {
        if selected || cp_left_of_right.contains(&e) {
            cps.insert(e, cp_l);
        }
    }
    cps.into_iter()
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
        world.entity(e).get::<Checkpoint>().unwrap().right
    };
    (left, right)
}
