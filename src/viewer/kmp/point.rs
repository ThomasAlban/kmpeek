use super::{
    meshes_materials::{KmpMeshes, PointMaterials},
    settings::OutlineSettings,
    FromKmp, KmpError, KmpSelectablePoint,
};
use crate::{
    util::kmp_file::{KmpFile, KmpGetSection, KmpPositionPoint, KmpRotationPoint},
    viewer::normalize::{Normalize, NormalizeInheritParent},
};
use bevy::{math::vec3, prelude::*};
use bevy_mod_outline::{OutlineBundle, OutlineVolume};
use std::sync::Arc;

pub fn spawn_point_section<
    T: KmpGetSection + KmpPositionPoint + KmpRotationPoint + Send + 'static + Clone,
    U: Component + FromKmp<T> + Clone,
>(
    commands: &mut Commands,
    kmp: Arc<KmpFile>,
    kmp_errors: &mut Vec<KmpError>,
    meshes: KmpMeshes,
    materials: PointMaterials,
    outline: OutlineSettings,
) -> Vec<Entity> {
    let node_entries = &T::get_section(kmp.as_ref()).entries;
    let mut entities = Vec::with_capacity(node_entries.len());

    for (i, node) in node_entries.iter().enumerate() {
        let position: Vec3 = node.get_position().into();
        let euler_rot: Vec3 = node.get_rotation().into();
        let rotation = Quat::from_euler(
            EulerRot::XYZ,
            euler_rot.x.to_radians(),
            euler_rot.y.to_radians(),
            euler_rot.z.to_radians(),
        );
        let entity = PointSpawner::new(&meshes, &materials, &outline, U::from_kmp(node, kmp_errors, i))
            .pos(position)
            .rot(rotation)
            .visible(false)
            .spawn_command(commands);

        entities.push(entity);
    }
    entities
}

pub struct PointSpawner<'a, U> {
    meshes: &'a KmpMeshes,
    materials: &'a PointMaterials,
    position: Vec3,
    rotation: Quat,
    kmp_component: U,
    outline: &'a OutlineSettings,
    visible: bool,
}
impl<'a, U: Component + Clone> PointSpawner<'a, U> {
    pub fn new(
        meshes: &'a KmpMeshes,
        materials: &'a PointMaterials,
        outline: &'a OutlineSettings,
        kmp_component: U,
    ) -> Self {
        Self {
            meshes,
            materials,
            position: Vec3::default(),
            rotation: Quat::default(),
            kmp_component,
            outline,
            visible: true,
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
    fn get_parent_bundle(
        &self,
    ) -> (
        bevy::prelude::MaterialMeshBundle<bevy::prelude::StandardMaterial>,
        U,
        KmpSelectablePoint,
        Normalize,
        bevy_mod_outline::OutlineBundle,
    ) {
        (
            PbrBundle {
                mesh: self.meshes.sphere.clone(),
                material: self.materials.point.clone(),
                transform: Transform::from_translation(self.position).with_rotation(self.rotation),
                visibility: if self.visible {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                },
                ..default()
            },
            self.kmp_component.clone(),
            KmpSelectablePoint,
            Normalize::new(200., 30., BVec3::TRUE),
            OutlineBundle {
                outline: OutlineVolume {
                    visible: false,
                    colour: self.outline.color,
                    width: self.outline.width,
                },
                ..default()
            },
        )
    }
    fn get_children_bundles(&self) -> Vec<(MaterialMeshBundle<StandardMaterial>, NormalizeInheritParent)> {
        let mut bundles = Vec::new();

        let line_length = 750.;
        let mut line_transform = Transform::from_scale(vec3(1., line_length, 1.));
        line_transform.translation.z = line_length / 2.;
        line_transform.rotate_x(90_f32.to_radians());
        bundles.push((
            PbrBundle {
                mesh: self.meshes.cylinder.clone(),
                material: self.materials.line.clone(),
                transform: line_transform,
                ..default()
            },
            NormalizeInheritParent,
        ));

        let mut arrow_transform = Transform::from_translation(vec3(0., 0., line_length));
        arrow_transform.rotate_x(90_f32.to_radians());
        bundles.push((
            PbrBundle {
                mesh: self.meshes.cone.clone(),
                material: self.materials.arrow.clone(),
                transform: arrow_transform,
                ..default()
            },
            NormalizeInheritParent,
        ));

        let up_arrow_transform =
            Transform::from_translation(vec3(0., line_length * 0.75, 0.)).with_scale(vec3(1., 2., 1.));
        bundles.push((
            PbrBundle {
                mesh: self.meshes.cone.clone(),
                material: self.materials.up_arrow.clone(),
                transform: up_arrow_transform,
                ..default()
            },
            NormalizeInheritParent,
        ));
        bundles
    }

    pub fn spawn_command(&self, commands: &mut Commands) -> Entity {
        commands
            .spawn(self.get_parent_bundle())
            .with_children(|parent| {
                for child_bundle in self.get_children_bundles() {
                    parent.spawn(child_bundle);
                }
            })
            .id()
    }
    pub fn _spawn_world(&self, world: &mut World) -> Entity {
        world
            .spawn(self.get_parent_bundle())
            .with_children(|parent| {
                for child_bundle in self.get_children_bundles() {
                    parent.spawn(child_bundle);
                }
            })
            .id()
    }
}

pub fn add_respawn_point_preview(
    parent: Entity,
    commands: &mut Commands,
    meshes: &KmpMeshes,
    materials: &PointMaterials,
) {
    let mut children = Vec::with_capacity(12);
    // spawn respawn position previews
    let y = 700.;
    let mut z = -600.;
    while z <= 0. {
        let mut x = -450.;
        while x <= 450. {
            children.push(
                commands
                    .spawn({
                        PbrBundle {
                            mesh: meshes.sphere.clone(),
                            material: materials.line.clone(),
                            transform: Transform::from_translation(vec3(x, y, z)).with_scale(Vec3::splat(0.5)),
                            ..default()
                        }
                    })
                    .id(),
            );
            x += 300.;
        }
        z += 300.;
    }
    commands.entity(parent).push_children(&children);
}

// pub fn show_area_cub
