use crate::{
    ui::viewport::ViewportInfo,
    util::{ui_viewport_to_ndc, RaycastFromCam},
    viewer::{
        camera::Gizmo2dCam,
        kmp::{
            checkpoints::{CheckpointLeft, CheckpointRespawnLink},
            components::{KmpSelectablePoint, RespawnPoint, RoutePoint},
            routes::{GetRouteStart, RouteLink},
        },
    },
};
use bevy::{ecs::system::SystemState, prelude::*, utils::HashMap};
use bevy_mod_raycast::prelude::Raycast;
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

        for e in pts_to_be_linked {
            world.entity_mut(e).insert(RouteLink(route_start_e));
        }
    }
}
impl CreateLink for RespawnPoint {
    fn create_link(world: &mut World, respawn_e: Entity, cps_to_be_linked: Vec<Entity>) {
        for cp in cps_to_be_linked {
            world.entity_mut(cp).insert(CheckpointRespawnLink(respawn_e));
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
    q_camera: Query<(&mut Camera, &GlobalTransform), Without<Gizmo2dCam>>,
    viewport_info: Res<ViewportInfo>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    q_window: Query<&Window>,
    q_route_pt: Query<Entity, With<T>>,
    mut raycast: Raycast,
    q_every_other_pt: Query<Entity, (With<KmpSelectablePoint>, Without<T>)>,
) {
    let Some(res) = res else { return };

    if res.is_added() {
        // we only just went into link selection mode so we need to set everything up
        for (e, v) in q_visibility.iter() {
            e_v_map.insert(e, *v);
        }
        for e in q_every_other_pt.iter() {
            *q_visibility.get_mut(e).unwrap().1 = Visibility::Hidden;
        }
        for e in q_route_pt.iter() {
            *q_visibility.get_mut(e).unwrap().1 = Visibility::Visible;
        }
    }

    let mut reset_visibilities = || {
        for (e, v) in e_v_map.iter() {
            let (_, mut v_mut) = q_visibility.get_mut(*e).unwrap();
            *v_mut = *v;
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

    let Some(mouse_pos) = q_window.single().cursor_position() else {
        reset_visibilities();
        return;
    };

    // get the active camera
    let cam = q_camera.iter().find(|cam| cam.0.is_active).unwrap();

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

    commands.add(move |world: &mut World| {
        T::create_link(world, intersection_e, entities);

        for (e, v) in e_v_map.iter() {
            let mut v_mut = world.query::<&mut Visibility>().get_mut(world, *e).unwrap();
            *v_mut = *v;
        }
    });
}
