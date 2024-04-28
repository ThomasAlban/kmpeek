use super::{
    meshes_materials::{CheckpointMaterials, KmpMeshes},
    path::spawn_path_section,
    settings::OutlineSettings,
    CheckpointLeft, CheckpointRight, Ckpt, KmpError, KmpFile, KmpSelectablePoint,
};
use crate::viewer::{
    edit::tweak::{SnapTo, Tweakable},
    normalize::Normalize,
};
use bevy::{math::vec3, prelude::*};
use bevy_mod_outline::{OutlineBundle, OutlineVolume};
use std::sync::Arc;

pub struct CheckpointPlugin;
impl Plugin for CheckpointPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CheckpointHeight>()
            .add_systems(Update, checkpoint_visibility_propagate);
    }
}

#[derive(Resource)]
pub struct CheckpointHeight(pub f32);
impl Default for CheckpointHeight {
    fn default() -> Self {
        Self(10000.)
    }
}

pub struct CheckpointSpawner<'a> {
    meshes: &'a KmpMeshes,
    materials: &'a CheckpointMaterials,
    kmp_component: CheckpointLeft,
    left_pos: Vec2,
    right_pos: Vec2,
    height: f32,
    outline: &'a OutlineSettings,
    visible: bool,
}
impl<'a> CheckpointSpawner<'a> {
    pub fn new(
        meshes: &'a KmpMeshes,
        materials: &'a CheckpointMaterials,
        outline: &'a OutlineSettings,
        kmp_component: CheckpointLeft,
    ) -> Self {
        Self {
            meshes,
            materials,
            kmp_component,
            left_pos: Vec2::default(),
            right_pos: Vec2::default(),
            height: 10000.,
            outline,
            visible: true,
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
    fn get_point_bundles(
        &self,
    ) -> (
        (
            MaterialMeshBundle<StandardMaterial>,
            CheckpointLeft,
            KmpSelectablePoint,
            Tweakable,
            Normalize,
            OutlineBundle,
        ),
        (
            MaterialMeshBundle<StandardMaterial>,
            CheckpointRight,
            KmpSelectablePoint,
            Tweakable,
            Normalize,
            OutlineBundle,
        ),
    ) {
        (
            (
                PbrBundle {
                    mesh: self.meshes.sphere.clone(),
                    material: self.materials.point.clone(),
                    transform: Transform::from_translation(vec3(self.left_pos.x, self.height, self.left_pos.y)),
                    visibility: if self.visible {
                        Visibility::Visible
                    } else {
                        Visibility::Hidden
                    },
                    ..default()
                },
                self.kmp_component.clone(),
                KmpSelectablePoint,
                Tweakable(SnapTo::CheckpointPlane),
                Normalize::new(200., 30., BVec3::TRUE),
                OutlineBundle {
                    outline: OutlineVolume {
                        visible: false,
                        colour: self.outline.color,
                        width: self.outline.width,
                    },
                    ..default()
                },
            ),
            (
                PbrBundle {
                    mesh: self.meshes.sphere.clone(),
                    material: self.materials.point.clone(),
                    transform: Transform::from_translation(vec3(self.right_pos.x, self.height, self.right_pos.y)),
                    visibility: if self.visible {
                        Visibility::Visible
                    } else {
                        Visibility::Hidden
                    },
                    ..default()
                },
                CheckpointRight {
                    left: Entity::PLACEHOLDER,
                },
                KmpSelectablePoint,
                Tweakable(SnapTo::CheckpointPlane),
                Normalize::new(200., 30., BVec3::TRUE),
                OutlineBundle {
                    outline: OutlineVolume {
                        visible: false,
                        colour: self.outline.color,
                        width: self.outline.width,
                    },
                    ..default()
                },
            ),
        )
    }
    pub fn set_left_right_entities(world: &mut World, left_entity: Entity, right_entity: Entity) {
        let mut left = world.get_mut::<CheckpointLeft>(left_entity).unwrap();
        left.right = right_entity;
        let mut right = world.get_mut::<CheckpointRight>(right_entity).unwrap();
        right.left = left_entity;
    }
    pub fn spawn_world(&self, world: &mut World) -> Entity {
        let left = world.spawn(self.get_point_bundles().0).id();
        let right = world.spawn(self.get_point_bundles().1).id();
        Self::set_left_right_entities(world, left, right);
        left
    }
}

pub fn spawn_checkpoint_section(
    commands: &mut Commands,
    kmp: Arc<KmpFile>,
    kmp_errors: &mut Vec<KmpError>,
    meshes: KmpMeshes,
    materials: CheckpointMaterials,
    outline: OutlineSettings,
    height: f32,
) {
    spawn_path_section::<Ckpt, CheckpointLeft>(commands, kmp, kmp_errors, move |node, kmp_component, world| {
        CheckpointSpawner::new(&meshes, &materials, &outline, kmp_component)
            .pos(node.cp_left.into(), node.cp_right.into())
            .visible(false)
            .height(height)
            .spawn_world(world)
    });
}

pub fn checkpoint_visibility_propagate(
    q_cp_left: Query<(&Visibility, &CheckpointLeft)>,
    mut q_cp_right: Query<&mut Visibility, (With<CheckpointRight>, Without<CheckpointLeft>)>,
) {
    for (left_vis, cp_left) in q_cp_left.iter() {
        let mut right_vis = q_cp_right.get_mut(cp_left.right).unwrap();
        *right_vis = *left_vis;
    }
}
