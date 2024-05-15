use super::{
    meshes_materials::KmpMeshesMaterials,
    path::{get_kmp_data_and_component_groups, link_entity_groups, EntityGroup, KmpPathNode},
    CheckpointLeft, CheckpointLine, CheckpointRight, Ckpt, HideRotation, KmpError, KmpFile, KmpSelectablePoint,
    PathOverallStart,
};
use crate::{
    ui::settings::AppSettings,
    viewer::{
        edit::{
            transform_gizmo::GizmoTransformable,
            tweak::{SnapTo, Tweakable},
        },
        normalize::Normalize,
    },
};
use bevy::{math::vec3, prelude::*, utils::HashMap};
use bevy_mod_outline::{OutlineBundle, OutlineVolume};
use std::sync::Arc;

pub struct CheckpointPlugin;
impl Plugin for CheckpointPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CheckpointHeight>().add_systems(
            Update,
            (
                set_checkpoint_right_visibility,
                set_checkpoint_node_height,
                update_checkpoint_lines,
            ),
        );
    }
}

#[derive(Resource)]
pub struct CheckpointHeight(pub f32);

impl Default for CheckpointHeight {
    fn default() -> Self {
        Self(10000.)
    }
}

pub struct CheckpointSpawner {
    kmp_component: CheckpointLeft,
    left_pos: Vec2,
    right_pos: Vec2,
    height: f32,
    visible: bool,
    left_e: Option<Entity>,
    right_e: Option<Entity>,
}
impl CheckpointSpawner {
    pub fn new(kmp_component: CheckpointLeft) -> Self {
        Self {
            kmp_component,
            left_pos: Vec2::default(),
            right_pos: Vec2::default(),
            height: 10000.,
            visible: true,
            left_e: None,
            right_e: None,
        }
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

    pub fn _spawn_command(mut self, commands: &mut Commands) -> (Entity, Entity) {
        let left = self.left_e.unwrap_or_else(|| commands.spawn_empty().id());
        let right = self.right_e.unwrap_or_else(|| commands.spawn_empty().id());
        self.left_e = Some(left);
        self.right_e = Some(right);

        commands.add(move |world: &mut World| {
            self.spawn(world);
        });
        (left, right)
    }

    pub fn spawn(self, world: &mut World) -> (Entity, Entity) {
        let meshes_materials = world.resource::<KmpMeshesMaterials>();
        let mesh = meshes_materials.meshes.sphere.clone();
        let material = meshes_materials.materials.checkpoints.point.clone();
        let outline = world.get_resource::<AppSettings>().unwrap().kmp_model.outline.clone();

        let left = self.left_e.unwrap_or_else(|| world.spawn_empty().id());
        let right = self.right_e.unwrap_or_else(|| world.spawn_empty().id());

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
            CheckpointLeft {
                right,
                ..self.kmp_component.clone()
            },
            KmpSelectablePoint,
            Tweakable(SnapTo::CheckpointPlane),
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
            KmpPathNode::default(),
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
            CheckpointRight { left },
            KmpSelectablePoint,
            Tweakable(SnapTo::CheckpointPlane),
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
            KmpPathNode::default(),
        ));

        (left, right)
    }
}

pub fn spawn_checkpoint_section(
    commands: &mut Commands,
    kmp: Arc<KmpFile>,
    kmp_errors: &mut Vec<KmpError>,
    height: f32,
) {
    let kmp_groups = get_kmp_data_and_component_groups::<Ckpt, CheckpointLeft>(kmp, kmp_errors);

    commands.add(move |world: &mut World| {
        let mut left_entity_groups: Vec<EntityGroup> = Vec::with_capacity(kmp_groups.len());
        let mut right_entity_groups = left_entity_groups.clone();
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
                    .spawn(world);
                if i == 0 && j == 0 {
                    world.entity_mut(left).insert(PathOverallStart);
                    world.entity_mut(right).insert(PathOverallStart);
                }
                left_entity_group.entities.push(left);
                right_entity_group.entities.push(right);
            }
            left_entity_groups.push(left_entity_group);
            right_entity_groups.push(right_entity_group);
        }
        link_entity_groups(world, left_entity_groups);
        link_entity_groups(world, right_entity_groups);
    });
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
    mut q_cp: Query<&mut Transform, Or<(With<CheckpointLeft>, With<CheckpointRight>)>>,
    cp_height: Res<CheckpointHeight>,
) {
    for mut cp in q_cp.iter_mut() {
        if cp.is_changed() {
            cp.translation.y = cp_height.0;
        }
    }
}

fn update_checkpoint_lines(
    mut commands: Commands,
    q_cp_left: Query<
        (&CheckpointLeft, Ref<Transform>, Entity, &Visibility),
        (Without<CheckpointRight>, Without<CheckpointLine>),
    >,
    q_cp_right: Query<Ref<Transform>, (With<CheckpointRight>, Without<CheckpointLeft>, Without<CheckpointLine>)>,
    mut q_cp_line: Query<
        (&mut Transform, Entity, &CheckpointLine, &mut Visibility),
        (Without<CheckpointLeft>, Without<CheckpointRight>),
    >,
) {
    // list of all left nodes of checkpoints we are yet to deal with
    // if we get to the end and there's still stuff in this list, we know we have new checkpoints to add a line for
    let mut cps_to_link: HashMap<Entity, (CheckpointLeft, (Transform, bool), Visibility)> = HashMap::new();
    for (cp_left, trans, entity, visibility) in q_cp_left.iter() {
        cps_to_link.insert(entity, (cp_left.clone(), (*trans, trans.is_changed()), *visibility));
    }

    for (mut line_trans, line_entity, cp_line, mut line_visibility) in q_cp_line.iter_mut() {
        let Some((cp_left, (left_trans, left_trans_is_changed), visibility)) = cps_to_link.remove(&cp_line.left) else {
            // if the left cp node doesn't exist, it has been deleted, so delete this checkpoint line
            commands.entity(line_entity).despawn();
            continue;
        };

        *line_visibility = visibility;

        let Ok(right_trans) = q_cp_right.get(cp_left.right) else {
            // if the right node doesn't exist, we may be in the process of deleting a checkpoint so also remove the line
            commands.entity(line_entity).despawn();
            continue;
        };
        if !left_trans_is_changed && !right_trans.is_changed() {
            continue;
        }

        let l_tr = left_trans.translation;
        let r_tr = right_trans.translation;

        let mut new_line_transform = Transform::from_translation(l_tr.lerp(r_tr, 0.5)).looking_at(r_tr, Vec3::Y);
        new_line_transform.rotate_local_x(f32::to_radians(-90.));
        new_line_transform.scale.y = l_tr.distance(r_tr);
        *line_trans = new_line_transform;
    }

    // for any not linked, we need to spawn a new line
    for (left_entity, (cp_left, (left_trans, _), visibility)) in cps_to_link.iter() {
        let left_entity = *left_entity;
        let cp_left = cp_left.clone();
        let visibility = *visibility;
        let l_tr = left_trans.translation;
        dbg!(cp_left.right);

        let r_tr = q_cp_right.get(cp_left.right).unwrap().translation;
        let mut transform = Transform::from_translation(l_tr.lerp(r_tr, 0.5)).looking_at(r_tr, Vec3::Y);
        transform.rotate_local_x(f32::to_radians(-90.));
        transform.scale.y = l_tr.distance(r_tr);

        commands.add(move |world: &mut World| {
            let meshes_materials = world.resource::<KmpMeshesMaterials>();

            world.spawn((
                PbrBundle {
                    mesh: meshes_materials.meshes.cylinder.clone(),
                    material: meshes_materials.materials.checkpoints.join_line.clone(),
                    transform,
                    visibility,
                    ..default()
                },
                Normalize::new(200., 30., BVec3::new(true, false, true)),
                CheckpointLine { left: left_entity },
            ));
        });
    }
}
