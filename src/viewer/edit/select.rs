use super::area_gizmo::AreaGizmoOptions;
use super::create_delete::JustCreatedPoint;
use super::link_select_mode::LinkSelectMode;
use super::transform_gizmo::TransformGizmoState;
use super::EditorMode;
use crate::ui::keybinds::{Modifier, ModifiersPressed};
use crate::ui::update_ui::UpdateUiSet;
use crate::ui::viewport::ViewportInfo;
use crate::util::{ui_viewport_to_ndc, world_to_ui_viewport, RaycastFromCam};
use crate::viewer::camera::EditorCamera;
use crate::viewer::kmp::components::{KmpSelectablePoint, RespawnPoint, RoutePoint};
use crate::viewer::kmp::sections::KmpEditMode;
use bevy::prelude::*;
use bevy_mod_outline::*;

#[derive(SystemSet, Debug, PartialEq, Eq, Hash, Clone)]
pub struct SelectSet;

pub fn select_plugin(app: &mut App) {
    app.init_resource::<SelectBox>()
        .init_resource::<SelectPainter>()
        .add_systems(
            Update,
            (select, select_box, select_painter, select_all)
                .chain()
                .in_set(SelectSet),
        )
        .add_systems(Update, update_outlines.after(SelectSet))
        .add_systems(
            Update,
            (deselect_if_not_visible, deselect_on_mode_change.after(UpdateUiSet)),
        );
}

#[derive(Component, Default)]
pub struct Selected;

#[derive(Resource, Clone, Copy, Debug)]
pub struct SelectPainter {
    pub radius: f32,
}

impl Default for SelectPainter {
    fn default() -> Self {
        Self { radius: 24.0 }
    }
}

impl SelectPainter {
    pub const MIN_RADIUS: f32 = 2.0;
    pub const MAX_RADIUS: f32 = 200.0;
}

fn select(
    viewport_info: Res<ViewportInfo>,
    q_window: Query<&Window>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    q_camera: Query<(&mut Camera, &GlobalTransform), With<EditorCamera>>,
    transform_gizmo: Res<TransformGizmoState>,
    mut raycast: MeshRayCast,
    q_kmp_section: Query<&KmpSelectablePoint>,
    mut commands: Commands,
    area_gizmo_opts: Res<AreaGizmoOptions>,
    q_selected: Query<Entity, With<Selected>>,
    mut ev_just_created_point: MessageReader<JustCreatedPoint>,

    route_selection_mode: Option<Res<LinkSelectMode<RoutePoint>>>,
    respawn_selection_mode: Option<Res<LinkSelectMode<RespawnPoint>>>,
) {
    if !viewport_info.mouse_in_viewport
        || viewport_info.mouse_on_overlayed_ui
        || !mouse_buttons.just_pressed(MouseButton::Left)
        || (ev_just_created_point.is_empty() && (keys.pressed(KeyCode::AltLeft)) || keys.pressed(KeyCode::AltRight))
        || area_gizmo_opts.mouse_hovering
        || transform_gizmo.is_focused
        || route_selection_mode.is_some()
        || respawn_selection_mode.is_some()
    {
        return;
    }

    let Some(mouse_pos) = q_window.single().ok().and_then(|x| x.cursor_position()) else {
        return;
    };

    let shift_key_down = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);

    // get the active camera
    let Some(cam) = q_camera.iter().find(|cam| cam.0.is_active) else {
        return;
    };

    let mouse_pos_ndc = ui_viewport_to_ndc(mouse_pos, viewport_info.viewport_rect);

    let intersections = RaycastFromCam::new(cam, mouse_pos_ndc, &mut raycast)
        .filter(&|e| q_kmp_section.contains(e))
        .cast();
    let intersection = intersections.first();

    // deselect everything if we already have something selected but don't have the shift key down
    if intersection.is_some() && !shift_key_down {
        for selected in q_selected.iter() {
            commands.entity(selected).remove::<Selected>();
        }
    }
    // select the entity
    if let Some((to_select, _)) = intersection {
        commands.entity(*to_select).insert(Selected);
    } else if !shift_key_down {
        // if we just randomly clicked on nothing then deselect everything
        for selected in q_selected.iter() {
            commands.entity(selected).remove::<Selected>();
        }
    }
    for created_point in ev_just_created_point.read() {
        commands.entity(created_point.0).insert(Selected);
    }
}

fn select_all(
    mut commands: Commands,
    q_selectable: Query<(Entity, &Visibility), With<KmpSelectablePoint>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if !keys.keybind_pressed([Modifier::Ctrl], [KeyCode::KeyA]) {
        return;
    }

    for (e, visibility) in q_selectable.iter() {
        if *visibility == Visibility::Visible {
            commands.entity(e).insert(Selected);
        }
    }
}

fn deselect_if_not_visible(
    mut commands: Commands,
    q_selected: Query<(Entity, &Visibility), With<Selected>>,
    route_select_mode: Option<Res<LinkSelectMode<RoutePoint>>>,
    respawn_select_mode: Option<Res<LinkSelectMode<RespawnPoint>>>,
) {
    if route_select_mode.is_some() || respawn_select_mode.is_some() {
        return;
    }
    for (e, visibility) in q_selected.iter() {
        if visibility != Visibility::Visible {
            commands.entity(e).remove::<Selected>();
        }
    }
}

fn deselect_on_mode_change(mode: Res<KmpEditMode>, mut commands: Commands, q_selected: Query<Entity, With<Selected>>) {
    if !mode.is_changed() {
        return;
    }
    for e in q_selected.iter() {
        commands.entity(e).remove::<Selected>();
    }
}

#[derive(Resource, Default)]
pub struct SelectBox(pub Option<Rect>);
impl SelectBox {
    /// How much we have to move the mouse before we actually start making a select box
    const LENIENCY_BEFORE_SELECT: f32 = 3.;
}

// this handles working out the select box rectangle and actually selecting stuff (the visuals for the box are handled in the UI section)
fn select_box(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    q_window: Query<&Window>,
    editor_mode: Res<EditorMode>,
    viewport_info: Res<ViewportInfo>,
    area_gizmo_opts: Res<AreaGizmoOptions>,
    route_selection_mode: Option<Res<LinkSelectMode<RoutePoint>>>,
    respawn_selection_mode: Option<Res<LinkSelectMode<RespawnPoint>>>,
    q_selectable: Query<(&Transform, Entity, &Visibility, Has<Selected>), With<KmpSelectablePoint>>,
    q_camera: Query<(&Camera, &GlobalTransform), With<EditorCamera>>,
    mut raycast: MeshRayCast,
    mut commands: Commands,
    mut select_box: ResMut<SelectBox>,
    mut initial_mouse_pos: Local<Vec2>,
    mut selecting: Local<bool>,
) {
    if *editor_mode != EditorMode::Default {
        *select_box = SelectBox::default();
        *selecting = false;
        return;
    }

    let Ok(window) = q_window.single() else { return };
    let Some(mouse_pos) = window.cursor_position() else {
        return;
    };

    if mouse_buttons.just_pressed(MouseButton::Left) {
        let started_on_point = q_camera.iter().find(|camera| camera.0.is_active).is_some_and(|camera| {
            let mouse_pos_ndc = ui_viewport_to_ndc(mouse_pos, viewport_info.viewport_rect);
            RaycastFromCam::new(camera, mouse_pos_ndc, &mut raycast)
                .filter(&|entity| q_selectable.contains(entity))
                .cast()
                .first()
                .is_some()
        });
        *selecting = viewport_info.mouse_in_viewport
            && !viewport_info.mouse_on_overlayed_ui
            && !area_gizmo_opts.mouse_hovering
            && !started_on_point
            && route_selection_mode.is_none()
            && respawn_selection_mode.is_none();
        if *selecting {
            *initial_mouse_pos = mouse_pos;
        } else {
            *select_box = SelectBox::default();
        }
    }

    if *selecting
        && mouse_buttons.pressed(MouseButton::Left)
        && initial_mouse_pos.distance(mouse_pos) > SelectBox::LENIENCY_BEFORE_SELECT
    {
        // Cancel this gesture if the pointer leaves the viewport.
        if !viewport_info.mouse_in_viewport {
            *select_box = SelectBox::default();
            *selecting = false;
            return;
        }

        // set the select box with the initial mouse pos and the current mouse pos as the 2 corners
        *select_box = SelectBox(Some(Rect::from_corners(*initial_mouse_pos, mouse_pos)));
    }

    // when we release the mouse button, we actually select stuff
    if mouse_buttons.just_released(MouseButton::Left) {
        let was_selecting = *selecting;
        *selecting = false;
        if !was_selecting {
            *select_box = SelectBox::default();
            return;
        }
        let Some(select_rect) = select_box.0 else {
            return;
        };
        // get the active camera
        let Some(cam) = q_camera.iter().find(|cam| cam.0.is_active) else {
            return;
        };

        // select stuff
        for selectable in q_selectable.iter() {
            if selectable.2 != Visibility::Visible || selectable.3 {
                continue;
            }
            let Some(viewport_pos) = world_to_ui_viewport(cam, viewport_info.viewport_rect, selectable.0.translation)
            else {
                continue;
            };
            if select_rect.contains(viewport_pos) {
                commands.entity(selectable.1).insert(Selected);
            }
        }
        // reset the select box after we've selected stuff
        *select_box = SelectBox::default();
    }
}

fn select_painter(
    editor_mode: Res<EditorMode>,
    painter: Res<SelectPainter>,
    viewport_info: Res<ViewportInfo>,
    q_window: Query<&Window>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    q_camera: Query<(&Camera, &GlobalTransform), With<EditorCamera>>,
    q_selectable: Query<(&Transform, Entity, &Visibility), With<KmpSelectablePoint>>,
    mut commands: Commands,
    mut painting: Local<bool>,
    mut previous_mouse_pos: Local<Option<Vec2>>,
    route_selection_mode: Option<Res<LinkSelectMode<RoutePoint>>>,
    respawn_selection_mode: Option<Res<LinkSelectMode<RespawnPoint>>>,
) {
    if mouse_buttons.just_released(MouseButton::Left) || *editor_mode != EditorMode::SelectPainter {
        *painting = false;
        *previous_mouse_pos = None;
    }

    if *editor_mode != EditorMode::SelectPainter {
        return;
    }

    if mouse_buttons.just_pressed(MouseButton::Left) {
        *painting = viewport_info.mouse_in_viewport
            && !viewport_info.mouse_on_overlayed_ui
            && route_selection_mode.is_none()
            && respawn_selection_mode.is_none();
    }

    if !*painting || !mouse_buttons.pressed(MouseButton::Left) || !viewport_info.mouse_in_viewport {
        return;
    }

    let Some(mouse_pos) = q_window.single().ok().and_then(Window::cursor_position) else {
        return;
    };
    let Some(camera) = q_camera.iter().find(|camera| camera.0.is_active) else {
        return;
    };
    let stroke_start = previous_mouse_pos.unwrap_or(mouse_pos);
    let radius = painter
        .radius
        .clamp(SelectPainter::MIN_RADIUS, SelectPainter::MAX_RADIUS);

    for (transform, entity, visibility) in q_selectable.iter() {
        if *visibility != Visibility::Visible {
            continue;
        }
        let Some(point_pos) = world_to_ui_viewport(camera, viewport_info.viewport_rect, transform.translation) else {
            continue;
        };
        if distance_to_segment(point_pos, stroke_start, mouse_pos) <= radius {
            commands.entity(entity).insert(Selected);
        }
    }

    *previous_mouse_pos = Some(mouse_pos);
}

fn distance_to_segment(point: Vec2, start: Vec2, end: Vec2) -> f32 {
    let segment = end - start;
    let length_squared = segment.length_squared();
    if length_squared == 0.0 {
        return point.distance(start);
    }
    let t = ((point - start).dot(segment) / length_squared).clamp(0.0, 1.0);
    point.distance(start + segment * t)
}

// put outlines on any entities which are selected, and remove them if they aren't selected
fn update_outlines(
    q_entities: Query<(Entity, Has<Selected>, &Visibility), With<KmpSelectablePoint>>,
    mut q_outline: Query<&mut OutlineVolume>,
) {
    for (entity, is_selected, visibility) in q_entities.iter() {
        let Ok(mut outline) = q_outline.get_mut(entity) else {
            continue;
        };
        outline.visible = is_selected;
        if visibility != Visibility::Visible {
            outline.visible = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn painter_sweep_hits_between_frames() {
        let start = Vec2::new(0.0, 0.0);
        let end = Vec2::new(100.0, 0.0);

        assert_eq!(distance_to_segment(Vec2::new(50.0, 0.0), start, end), 0.0);
        assert_eq!(distance_to_segment(Vec2::new(50.0, 12.0), start, end), 12.0);
        assert_eq!(distance_to_segment(Vec2::new(120.0, 0.0), start, end), 20.0);
    }

    #[test]
    fn painter_radius_has_safe_defaults() {
        let painter = SelectPainter::default();
        assert!(painter.radius >= SelectPainter::MIN_RADIUS);
        assert!(painter.radius <= SelectPainter::MAX_RADIUS);
    }
}
