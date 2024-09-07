use super::{
    meshes_materials::{KmpMeshes, PointMaterials},
    ordering::{NextOrderID, OrderId},
    routes::RouteLink,
    KmpComponent, KmpSectionEntityIdMap, KmpSectionIdEntityMap, KmpSelectablePoint, MaybeRouteId, RespawnPoint,
    RoutePoint, Section, Spawn, Spawner,
};
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
use bevy::{ecs::world::Command, math::vec3, prelude::*};
use bevy_mod_outline::{OutlineBundle, OutlineVolume};

pub fn spawn_point_section<T: KmpComponent + Spawn>(world: &mut World, kmp: &KmpFile) -> KmpSectionIdEntityMap<T>
where
    T::KmpFormat: KmpGetSection + KmpPositionPoint + KmpRotationPoint + MaybeRouteId,
{
    let node_entries = &**T::KmpFormat::get_section(kmp);
    let mut id_entity_map = KmpSectionIdEntityMap::default();

    for (i, node) in node_entries.iter().enumerate() {
        let maybe_route_id = node.get_route_id();
        let maybe_route = maybe_route_id
            .and_then(|x| world.resource::<KmpSectionIdEntityMap<RoutePoint>>().get(&(x as u32)))
            .copied();

        let entity = Spawner::builder()
            .component(T::from_kmp(node, world))
            .pos(node.get_position())
            .rot(node.get_rotation())
            .visible(false)
            .order_id(i as u32)
            .maybe_route(maybe_route)
            .build()
            .spawn(world);

        id_entity_map.insert(i as u32, entity);
    }
    id_entity_map
}

pub fn spawn_point<T: Spawn + Component + Clone>(spawner: Spawner<T>, world: &mut World) -> Entity {
    let meshes = world.resource::<KmpMeshes>().clone();
    let materials = world.resource::<PointMaterials<T>>().clone();
    let outline = world.get_resource::<AppSettings>().unwrap().kmp_model.outline;

    // either gets the order id, or gets it from the NextOrderID (which will increment it for next time)
    let order_id = spawner
        .order_id
        .unwrap_or_else(|| world.resource::<NextOrderID<T>>().get());

    let mut entity = match spawner.e {
        Some(e) => world.entity_mut(e),
        None => world.spawn_empty(),
    };

    entity.insert((
        PbrBundle {
            mesh: meshes.sphere.clone(),
            material: materials.point.clone(),
            transform: spawner.get_transform(),
            visibility: if spawner.visible {
                Visibility::Visible
            } else {
                Visibility::Hidden
            },
            ..default()
        },
        spawner.component,
        KmpSelectablePoint,
        Tweakable(SnapTo::Kcl),
        GizmoTransformable,
        OrderId(order_id),
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
    if let Some(route_e) = spawner.route {
        entity.insert(RouteLink(route_e));
    }
    entity.id()
}

pub struct AddRespawnPointPreview(pub Entity);
impl Command for AddRespawnPointPreview {
    fn apply(self, world: &mut World) {
        let mesh = world.resource::<KmpMeshes>().sphere.clone();
        let material = world.resource::<PointMaterials<RespawnPoint>>().line.clone();

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

pub fn save_point_section<T: KmpComponent>(world: &mut World) -> (Section<T::KmpFormat>, KmpSectionEntityIdMap<T>) {
    let mut q = world.query::<(&T, &Transform, Entity, &OrderId)>();
    let components: Vec<_> = q
        .iter(world)
        .sort::<&OrderId>()
        .map(|(c, t, e, _)| (c.clone(), *t, e))
        .collect();
    let mut kmp_points = Vec::with_capacity(components.len());
    for (component, transform, e) in components.iter() {
        kmp_points.push(component.to_kmp(*transform, world, *e));
    }
    (
        Section::new(kmp_points),
        KmpSectionEntityIdMap::<T>::new(
            components
                .iter()
                .map(|x| x.2)
                .enumerate()
                .map(|x| (x.1, x.0 as u8))
                .collect(),
        ),
    )
}
