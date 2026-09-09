use crate::{
    ui::viewport::ViewportInfo,
    util::{ui_viewport_to_ndc, RaycastFromCam},
    viewer::{
        camera::EditorCamera,
        kmp::{
            checkpoints::CheckpointRespawnLink,
            components::{Checkpoint, KmpSelectablePoint, RespawnPoint, RoutePoint},
            routes::{GetRouteStart, RouteLink, RouteLinkedEntities},
        },
    },
};
use bevy::{ecs::system::SystemState, platform::collections::HashMap, prelude::*};
use std::marker::PhantomData;

use super::select::SelectSet;

pub fn link_select_mode_plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            update_link_selection_mode::<RoutePoint>,
            update_link_selection_mode::<RespawnPoint>,
        )
            .after(SelectSet),
    );
}

/// If this resource exists in the world, it means we are in link select mode and are currently
/// linking the type of point which is the generic type
#[derive(Resource, Deref, DerefMut)]
pub struct LinkSelectMode<T: Component>(#[deref] pub Vec<Entity>, PhantomData<T>);
impl<T: Component> LinkSelectMode<T> {
    pub fn new(e: impl IntoIterator<Item = Entity>) -> Self {
        Self(e.into_iter().collect(), PhantomData)
    }
}

trait CreateLink {
    fn create_link(world: &mut World, clicked_entity: Entity, entities_to_be_linked: Vec<Entity>);
}

impl CreateLink for RoutePoint {
    fn create_link(world: &mut World, route_e: Entity, pts_to_be_linked: Vec<Entity>) {
        let mut ss = SystemState::<GetRouteStart>::new(world);
        let get_route_start = ss.get_mut(world);

        let route_start_e = get_route_start.get_entity(route_e);
        ss.apply(world);

        if world.get::<RoutePoint>(route_e).is_none() || world.get::<RouteLinkedEntities>(route_start_e).is_none() {
            return;
        }

        for e in pts_to_be_linked {
            if let Ok(mut entity) = world.get_entity_mut(e) {
                entity.insert(RouteLink(route_start_e));
            }
        }
    }
}
impl CreateLink for RespawnPoint {
    fn create_link(world: &mut World, respawn_e: Entity, cps_to_be_linked: Vec<Entity>) {
        if world.get::<RespawnPoint>(respawn_e).is_none() {
            return;
        }
        for cp in cps_to_be_linked {
            if world.get::<Checkpoint>(cp).is_some() {
                world.entity_mut(cp).insert(CheckpointRespawnLink(respawn_e));
            }
        }
    }
}

fn update_link_selection_mode<T: Component + CreateLink>(
    res: Option<Res<LinkSelectMode<T>>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut q_visibility: Query<(Entity, &mut Visibility)>,
    // saves the visibility state of everything before we went into route selection mode
    mut e_v_map: Local<HashMap<Entity, Visibility>>,
    mut commands: Commands,
    q_camera: Query<(&mut Camera, &GlobalTransform), With<EditorCamera>>,
    viewport_info: Res<ViewportInfo>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    q_window: Query<&Window>,
    q_route_pt: Query<Entity, With<T>>,
    mut raycast: MeshRayCast,
    q_every_other_pt: Query<Entity, (With<KmpSelectablePoint>, Without<T>)>,
) {
    let Some(res) = res else { return };

    if res.is_added() {
        // we only just went into link selection mode so we need to set everything up
        for (e, v) in q_visibility.iter() {
            e_v_map.insert(e, *v);
        }
        for e in q_every_other_pt.iter() {
            if let Ok((_, mut visibility)) = q_visibility.get_mut(e) {
                *visibility = Visibility::Hidden;
            }
        }
        for e in q_route_pt.iter() {
            if let Ok((_, mut visibility)) = q_visibility.get_mut(e) {
                *visibility = Visibility::Visible;
            }
        }
    }

    let mut reset_visibilities = || {
        for (e, v) in e_v_map.iter() {
            if let Ok((_, mut v_mut)) = q_visibility.get_mut(*e) {
                *v_mut = *v;
            }
        }
    };

    if keys.just_pressed(KeyCode::Escape) {
        commands.remove_resource::<LinkSelectMode<T>>();
        reset_visibilities();
        return;
    }

    if !mouse_buttons.just_pressed(MouseButton::Left) {
        return;
    }

    commands.remove_resource::<LinkSelectMode<T>>();

    let Some(mouse_pos) = q_window.single().ok().and_then(|x| x.cursor_position()) else {
        reset_visibilities();
        return;
    };

    // get the active camera
    let Some(cam) = q_camera.iter().find(|cam| cam.0.is_active) else {
        reset_visibilities();
        return;
    };

    let mouse_pos_ndc = ui_viewport_to_ndc(mouse_pos, viewport_info.viewport_rect);

    let intersections = RaycastFromCam::new(cam, mouse_pos_ndc, &mut raycast)
        .filter(&|e| q_route_pt.contains(e))
        .cast();
    let Some(intersection_e) = intersections.first().map(|x| x.0) else {
        reset_visibilities();
        return;
    };
    let entities = res.0.clone();
    let e_v_map = e_v_map.clone();

    commands.queue(move |world: &mut World| {
        T::create_link(world, intersection_e, entities);

        for (e, v) in e_v_map.iter() {
            if let Some(mut v_mut) = world.get_mut::<Visibility>(*e) {
                *v_mut = *v;
            }
        }
    });
}
