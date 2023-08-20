use bevy::{prelude::*, window::PrimaryWindow};
use bevy_mod_outline::*;
use bevy_mod_raycast::prelude::*;

use crate::{
    camera::{FlyCam, OrbitCam, TopDownCam},
    kmp_file::Kmp,
    kmp_model::ItptModel,
    ui::AppState,
    undo::{Modify, ModifyAction, UndoStack},
};

pub struct MousePickingPlugin;

impl Plugin for MousePickingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            DefaultRaycastingPlugin::<KmpRaycastSet>::default(),
            DefaultRaycastingPlugin::<KclRaycastSet>::default(),
            OutlinePlugin,
        ))
        .init_resource::<SelectedEntities>()
        .add_systems(
            First,
            update_raycast_with_cursor
                .before(RaycastSystem::BuildRays::<KmpRaycastSet>)
                .before(RaycastSystem::BuildRays::<KclRaycastSet>),
        )
        .add_systems(Update, select_entity);
    }
}

#[derive(Reflect)]
pub struct KmpRaycastSet;
#[derive(Reflect)]
pub struct KclRaycastSet;

// update our raycast source with the current cursor position every frame
fn update_raycast_with_cursor(
    mut kmp_query: Query<&mut RaycastSource<KmpRaycastSet>>,
    mut kcl_query: Query<&mut RaycastSource<KclRaycastSet>>,
    app_state: Res<AppState>,
    window: Query<&Window, With<PrimaryWindow>>,
    modify_action: Res<ModifyAction>,
) {
    let window = window.get_single().unwrap();
    let Some(mouse_pos) = window.cursor_position() else { return };

    let scaled_mouse_pos = scale_mouse_pos(mouse_pos, window, app_state.viewport_rect);

    let mut kmp_pick_source = kmp_query.get_single_mut().unwrap();
    kmp_pick_source.cast_method = RaycastMethod::Screenspace(scaled_mouse_pos);

    let mut kcl_pick_source = kcl_query.get_single_mut().unwrap();
    kcl_pick_source.cast_method =
        RaycastMethod::Screenspace(scaled_mouse_pos + modify_action.mouse_point_offset);
}

fn scale_mouse_pos(mouse_pos: Vec2, window: &Window, viewport_rect: Rect) -> Vec2 {
    // make (0,0) be the top left corner of the viewport
    let mut scaled_mouse_pos = mouse_pos - viewport_rect.min;
    scaled_mouse_pos = scaled_mouse_pos.clamp(Vec2::ZERO, viewport_rect.max);
    scaled_mouse_pos *= window.scale_factor() as f32;
    scaled_mouse_pos
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct SelectedEntities(Vec<Entity>);
impl SelectedEntities {
    fn push(&mut self, entity: Entity, outline_query: &mut Query<&mut OutlineVolume>) {
        outline_query.get_mut(entity).unwrap().visible = true;
        self.0.push(entity);
    }
    fn clear(&mut self, outline_query: &mut Query<&mut OutlineVolume>) {
        for entity in self.0.iter() {
            outline_query.get_mut(*entity).unwrap().visible = false;
        }
        self.0.clear();
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn select_entity(
    mouse_buttons: Res<Input<MouseButton>>,
    keys: Res<Input<KeyCode>>,
    mut selected_entities: ResMut<SelectedEntities>,
    app_state: Res<AppState>,
    mut outline_query: Query<&mut OutlineVolume>,
    transform_query: Query<&Transform>,
    itpt_query: Query<&ItptModel>,
    kmp_query: Query<&RaycastSource<KmpRaycastSet>>,
    kcl_query: Query<&RaycastSource<KclRaycastSet>>,
    mut modify_action: ResMut<ModifyAction>,
    kmp: Option<ResMut<Kmp>>,
    cams: (
        Query<(&Camera, &GlobalTransform), (With<FlyCam>, Without<OrbitCam>, Without<TopDownCam>)>,
        Query<(&Camera, &GlobalTransform), (Without<FlyCam>, With<OrbitCam>, Without<TopDownCam>)>,
        Query<(&Camera, &GlobalTransform), (Without<FlyCam>, Without<OrbitCam>, With<TopDownCam>)>,
    ),
    mut undo_stack: ResMut<UndoStack>,
    window: Query<&Window, With<PrimaryWindow>>,
) {
    if !app_state.mouse_in_viewport {
        return;
    }
    let Some(mut kmp) = kmp else { return };
    let window = window.get_single().unwrap();
    let Some(mouse_pos) = window.cursor_position() else { return };
    let shift_key_down = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let kcl_raycast_source = kcl_query.get_single().unwrap();
    let kmp_raycast_source = kmp_query.get_single().unwrap();

    if mouse_buttons.just_pressed(MouseButton::Left) {
        if let Some((entity, _)) = kmp_raycast_source.get_nearest_intersection() {
            // if we already have something selected but not the shift key down then deselect that thing
            if !selected_entities.is_empty() && !shift_key_down {
                selected_entities.clear(&mut outline_query);
            }
            // push the entity if it is not already in selected_entities
            if !selected_entities.contains(&entity) {
                selected_entities.push(entity, &mut outline_query);
            }
        } else if !shift_key_down {
            // if we just randomly clicked on nothing then clear selected_entities
            selected_entities.clear(&mut outline_query);
        }
    }
    if selected_entities.is_empty() {
        return;
    }

    // get the ITPT indexes of all the selected points
    let mut itpt_indexes = Vec::with_capacity(selected_entities.len());
    for i in 0..selected_entities.len() {
        let itpt_index = itpt_query.get(selected_entities[i]).unwrap().0;
        itpt_indexes.push(itpt_index);
    }

    // if keys.just_pressed(KeyCode::Back) || keys.just_pressed(KeyCode::Delete) {
    //     let mut remove_action_items = Vec::new();
    //     for itpt_index in itpt_indexes.iter() {
    //         remove_action_items.push(Remove::new(
    //             *itpt_index,
    //             kmp.itpt.entries[*itpt_index].clone(),
    //         ));
    //     }
    //     let remove_action = RemoveAction::new(remove_action_items);
    //     undo_stack.push(remove_action);
    //     for itpt_index in itpt_indexes.iter() {
    //         kmp.itpt.entries.remove(*itpt_index);
    //     }
    // }

    if mouse_buttons.pressed(MouseButton::Left) {
        // the point which the mouse is over (main point)
        let main_point = if let Some(main_point_itpt_index) = modify_action.main_point_itpt_index {
            kmp.itpt.entries[main_point_itpt_index].clone()
        } else {
            // find the entity of the point which the mouse is over
            let Some((entity_mouse_over, _)) = kmp_raycast_source.get_nearest_intersection() else { return };
            // find the itpt index of this point
            let main_point_itpt_index = itpt_query.get(entity_mouse_over).unwrap().0;
            kmp.itpt.entries[main_point_itpt_index].clone()
        };

        // get the position differences of all the other selected points relative to the main point
        let mut position_differences = Vec::with_capacity(selected_entities.len());
        for itpt_index in itpt_indexes.iter() {
            let position = kmp.itpt.entries[*itpt_index].position;
            position_differences.push(position - main_point.position);
        }

        // if we are already dragging a point around
        if !modify_action.items.is_empty() {
            if modify_action.mouse_initial_pos != mouse_pos {
                // set the 'after' of each point of the modify action to be the position of the main point + the position differences
                for (i, modify) in modify_action.items.iter_mut().enumerate() {
                    modify.after.position =
                        kmp.itpt.entries[itpt_indexes[i]].position + position_differences[i];
                }
            }
        } else {
            // if we have only started dragging a point around just now
            // create a new modify vec containing the positions of all the selected entities (before and after the same as they haven't been edited yet)
            let mut modify_vec = Vec::with_capacity(selected_entities.len());
            for itpt_index in itpt_indexes.iter() {
                modify_vec.push(Modify::new(
                    *itpt_index,
                    kmp.itpt.entries[*itpt_index].clone(),
                    kmp.itpt.entries[*itpt_index].clone(),
                ))
            }
            // find the entity of the point which the mouse is over
            let Some((entity_mouse_over, intersection)) = kmp_raycast_source.get_nearest_intersection() else { return };
            // find the itpt index of this point
            let main_point_itpt_index = itpt_query.get(entity_mouse_over).unwrap().0;

            let entity_pos = transform_query.get(entity_mouse_over).unwrap().translation;

            // get the currnt active camera
            let (active_cam, active_cam_transform) = {
                let (fly_cam, fly_cam_transform) = cams.0.get_single().unwrap();
                if !fly_cam.is_active {
                    let (orbit_cam, orbit_cam_transform) = cams.1.get_single().unwrap();
                    if !orbit_cam.is_active {
                        cams.2.get_single().unwrap()
                    } else {
                        (orbit_cam, orbit_cam_transform)
                    }
                } else {
                    (fly_cam, fly_cam_transform)
                }
            };

            let entity_screenspace = active_cam
                .world_to_viewport(active_cam_transform, entity_pos)
                .unwrap();

            // get the offset between where the mouse is on the screen and where the entity is on the screen
            // so that when we drag a point, its centre does not snap to the mouse
            let mouse_point_offset =
                entity_screenspace - scale_mouse_pos(mouse_pos, window, app_state.viewport_rect);

            // create a new modify action with all this data
            *modify_action = ModifyAction::new(
                modify_vec,
                main_point_itpt_index,
                mouse_point_offset,
                mouse_pos,
                intersection.distance(),
            );
            // this return is important because otherwise for one frame the position of the point will be set incorrectly below,
            // before next frame the mouse_point_offset will be accounted for when rays are sent out
            return;
        }

        // set the position of the points to the position of the intersection with the collision model + the position differences
        // only do this if the mouse has actually moved
        if modify_action.mouse_initial_pos != mouse_pos {
            if let Some((_, intersection)) = kcl_raycast_source.get_nearest_intersection() {
                for (i, itpt_index) in itpt_indexes.iter().enumerate() {
                    kmp.itpt.entries[*itpt_index].position =
                        intersection.position() + position_differences[i];
                }
            } else {
                // if there is no intersection with the collision model,
                // keep the same distance from the camera as the distance when we first started dragging the point
                if let Some(ray) = kcl_raycast_source.ray {
                    for (i, itpt_index) in itpt_indexes.iter().enumerate() {
                        kmp.itpt.entries[*itpt_index].position = ray
                            .position(modify_action.initial_intersection_distance)
                            + position_differences[i];
                    }
                }
            }
        }
    } else if mouse_buttons.just_released(MouseButton::Left) && !modify_action.items.is_empty() {
        // if we just released the mouse button then push the modify action to the undo stack
        // only if it actually modifies anything - e.g. if we just click on a point and don't modify it then don't push to the undo stack
        if modify_action.modifies() {
            undo_stack.push(modify_action.clone());
        }
        // and reset the modify action to default
        *modify_action = ModifyAction::default();
    }
}
