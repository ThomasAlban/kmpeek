use bevy::{prelude::*, window::PrimaryWindow};
use bevy_mod_outline::*;
use bevy_mod_raycast::prelude::*;

use crate::{
    camera::FlyCam,
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
    let window = window
        .get_single()
        .expect("Could not get primary window in update raycast with cursor");
    let Some(mouse_pos) = window.cursor_position() else { return };

    let scaled_mouse_pos = scale_mouse_pos(mouse_pos, window, app_state.viewport_rect);

    //grab the most recent cursor event if it exists
    for mut pick_source in &mut kmp_query {
        pick_source.cast_method = RaycastMethod::Screenspace(scaled_mouse_pos);
    }

    let offset = modify_action.mouse_screen_offset;
    for mut pick_source in &mut kcl_query {
        pick_source.cast_method = RaycastMethod::Screenspace(scaled_mouse_pos + offset);
    }
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
        outline_query
            .get_mut(entity)
            .expect("element of selected_entities did not have OutlineVolume component")
            .visible = true;
        self.0.push(entity);
    }
    fn clear(&mut self, outline_query: &mut Query<&mut OutlineVolume>) {
        for entity in self.0.iter() {
            outline_query
                .get_mut(*entity)
                .expect("element of selected_entities did not have OutlineVolume component")
                .visible = false;
        }
        self.0.clear();
    }
}

#[allow(clippy::too_many_arguments)]
fn select_entity(
    kmp_query: Query<&RaycastSource<KmpRaycastSet>>,
    mouse_buttons: Res<Input<MouseButton>>,
    keys: Res<Input<KeyCode>>,
    mut selected_entities: ResMut<SelectedEntities>,
    app_state: Res<AppState>,
    mut outline_query: Query<&mut OutlineVolume>,
    transform_query: Query<&Transform>,
    itpt_query: Query<&ItptModel>,
    kcl_query: Query<&RaycastSource<KclRaycastSet>>,
    mut modify_action: ResMut<ModifyAction>,
    kmp: Option<ResMut<Kmp>>,
    mut undo_stack: ResMut<UndoStack>,
    cam: Query<(&Camera, &GlobalTransform), With<FlyCam>>,
    window: Query<&Window, With<PrimaryWindow>>,
) {
    if !app_state.mouse_in_viewport {
        return;
    }
    let Some(mut kmp) = kmp else { return };
    let shift_key_down = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let kcl_raycast_source = kcl_query.get_single().unwrap();
    let kmp_raycast_source = kmp_query.get_single().unwrap();
    let window = window.get_single().unwrap();

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

        // get the ITPT indexes of all the selected points
        let mut itpt_indexes = Vec::new();
        for i in 0..selected_entities.len() {
            let itpt_index = itpt_query
                .get(selected_entities[i])
                .expect("element of selected_entities did not have a itpt model component")
                .0;
            itpt_indexes.push(itpt_index);
        }

        // get the position differences of all the other selected points relative to the main point
        let mut position_differences = Vec::new();
        for itpt_index in itpt_indexes.iter() {
            let position = kmp.itpt.entries[*itpt_index].position;
            position_differences.push(position - main_point.position);
        }

        // if we are already dragging a point around
        if !modify_action.items.is_empty() {
            // set the 'after' of each point of the modify action to be the position of the main point + the position differences
            for (i, modify) in modify_action.items.iter_mut().enumerate() {
                modify.after.position =
                    kmp.itpt.entries[itpt_indexes[i]].position + position_differences[i];
            }
        } else {
            // if we have only started dragging a point around just now
            // create a new modify vec containing the positions of all the selected entities (before and after the same as they haven't been edited yet)
            let mut modify_vec = Vec::new();
            for itpt_index in itpt_indexes.iter() {
                modify_vec.push(Modify::new(
                    *itpt_index,
                    kmp.itpt.entries[*itpt_index].clone(),
                    kmp.itpt.entries[*itpt_index].clone(),
                ))
            }
            // find the entity of the point which the mouse is over
            let Some((entity_mouse_over, _)) = kmp_raycast_source.get_nearest_intersection() else { return };
            // find the itpt index of this point
            let main_point_itpt_index = itpt_query.get(entity_mouse_over).unwrap().0;

            let entity_pos = transform_query.get(entity_mouse_over).unwrap().translation;
            let (cam, cam_transform) = cam.get_single().unwrap();
            let entity_screenspace = cam.world_to_viewport(cam_transform, entity_pos).unwrap();
            let mouse_screen_offset = entity_screenspace
                - scale_mouse_pos(
                    window.cursor_position().unwrap(),
                    window,
                    app_state.viewport_rect,
                );

            // create a new modify action with all this data
            *modify_action =
                ModifyAction::new(modify_vec, main_point_itpt_index, mouse_screen_offset);
            return;
        }

        // set the position of the points to the position of the intersection with the collision model + the position differences
        if let Some((_, intersection)) = kcl_raycast_source.get_nearest_intersection() {
            for (i, itpt_index) in itpt_indexes.iter().enumerate() {
                kmp.itpt.entries[*itpt_index].position =
                    intersection.position() + position_differences[i];
            }
        } else {
            // if there is no intersection with the collision model, do something I haven't figured out yet
        }
    } else if mouse_buttons.just_released(MouseButton::Left) && !modify_action.items.is_empty() {
        // if we just released the mouse button then push the modify action to the undo stack
        undo_stack.push(modify_action.clone());
        // and reset the modify action to default
        *modify_action = ModifyAction::default();
    }
}
