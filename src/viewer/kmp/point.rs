use super::{meshes_materials::KmpMeshesMaterials, FromKmp, GetPointMaterialSection, KmpError, KmpSelectablePoint};
use crate::{
    ui::settings::AppSettings,
    util::kmp_file::{KmpFile, KmpGetSection, KmpPositionPoint, KmpRotationPoint},
    viewer::{
        edit::{
            transform_gizmo::GizmoTransformable,
            tweak::{SnapTo, Tweakable},
        },
        normalize::{Normalize, NormalizeInheritParent},
    },
};
use bevy::{ecs::system::Command, math::vec3, prelude::*};
use bevy_mod_outline::{OutlineBundle, OutlineVolume};
use std::sync::Arc;

pub fn spawn_point_section<
    T: KmpGetSection + KmpPositionPoint + KmpRotationPoint + Send + Sync + 'static + Clone,
    U: Component + FromKmp<T> + Clone + GetPointMaterialSection,
>(
    commands: &mut Commands,
    kmp: Arc<KmpFile>,
    kmp_errors: &mut Vec<KmpError>,
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
        let entity = PointSpawner::new(U::from_kmp(node, kmp_errors, i))
            .pos(position)
            .rot(rotation)
            .visible(false)
            .spawn_command(commands);
        entities.push(entity);
    }
    entities
}

pub struct PointSpawner<U> {
    position: Vec3,
    rotation: Quat,
    kmp_component: U,
    visible: bool,
    e: Option<Entity>,
}
impl<U: Component + Clone + GetPointMaterialSection> PointSpawner<U> {
    pub fn new(kmp_component: U) -> Self {
        Self {
            position: Vec3::default(),
            rotation: Quat::default(),
            kmp_component,
            visible: true,
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
        let meshes = meshes_materials.meshes.clone();
        let materials = U::get_materials(&meshes_materials.materials).clone();
        let outline = world.get_resource::<AppSettings>().unwrap().kmp_model.outline.clone();

        let mut entity = match self.e {
            Some(e) => world.entity_mut(e),
            None => world.spawn_empty(),
        };

        entity.insert((
            PbrBundle {
                mesh: meshes.sphere.clone(),
                material: materials.point.clone(),
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
            Tweakable(SnapTo::Kcl),
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
        entity.with_children(|parent| {
            let line_length = 750.;
            let mut line_transform = Transform::from_scale(vec3(1., line_length, 1.));
            line_transform.translation.z = line_length / 2.;
            line_transform.rotate_x(90_f32.to_radians());
            parent.spawn((
                PbrBundle {
                    mesh: meshes.cylinder.clone(),
                    material: materials.line.clone(),
                    transform: line_transform,
                    ..default()
                },
                NormalizeInheritParent,
            ));

            let mut arrow_transform = Transform::from_translation(vec3(0., 0., line_length));
            arrow_transform.rotate_x(90_f32.to_radians());
            parent.spawn((
                PbrBundle {
                    mesh: meshes.cone.clone(),
                    material: materials.arrow.clone(),
                    transform: arrow_transform,
                    ..default()
                },
                NormalizeInheritParent,
            ));

            let up_arrow_transform =
                Transform::from_translation(vec3(0., line_length * 0.75, 0.)).with_scale(vec3(1., 2., 1.));
            parent.spawn((
                PbrBundle {
                    mesh: meshes.cone.clone(),
                    material: materials.up_arrow.clone(),
                    transform: up_arrow_transform,
                    ..default()
                },
                NormalizeInheritParent,
            ));
        });
        entity.id()
    }
}
pub struct AddRespawnPointPreview(pub Entity);
impl Command for AddRespawnPointPreview {
    fn apply(self, world: &mut World) {
        let meshes_materials = world.resource::<KmpMeshesMaterials>();
        let mesh = meshes_materials.meshes.sphere.clone();
        let material = meshes_materials.materials.respawn_points.line.clone();

        world.entity_mut(self.0).with_children(|parent| {
            // spawn respawn position previews
            let y = 700.;
            let mut z = -600.;
            while z <= 0. {
                let mut x = -450.;
                while x <= 450. {
                    parent.spawn({
                        PbrBundle {
                            mesh: mesh.clone(),
                            material: material.clone(),
                            transform: Transform::from_translation(vec3(x, y, z)).with_scale(Vec3::splat(0.5)),
                            ..default()
                        }
                    });
                    x += 300.;
                }
                z += 300.;
            }
        });
    }
}
