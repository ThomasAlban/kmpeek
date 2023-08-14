use bevy::{math::vec2, prelude::*, window::PrimaryWindow};
use bevy_mod_raycast::{
    print_intersections, DefaultRaycastingPlugin, RaycastMethod, RaycastPluginState, RaycastSource,
    RaycastSystem,
};

use crate::ui::AppState;

pub struct MousePickingPlugin;

impl Plugin for MousePickingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DefaultRaycastingPlugin::<RaycastSet>::default())
            .add_systems(Startup, setup)
            .add_systems(
                First,
                update_raycast_with_cursor.before(RaycastSystem::BuildRays::<RaycastSet>),
            )
            .add_systems(Update, print_intersections::<RaycastSet>);
    }
}

fn setup(mut commands: Commands) {
    commands.insert_resource(RaycastPluginState::<RaycastSet>::default().with_debug_cursor());
}

#[derive(Reflect)]
pub struct RaycastSet;

// update our raycast source with the current cursor position every frame
fn update_raycast_with_cursor(
    mut cursor: EventReader<CursorMoved>,
    mut query: Query<&mut RaycastSource<RaycastSet>>,
    app_state: Res<AppState>,
    window: Query<&Window, With<PrimaryWindow>>,
) {
    let window = window
        .get_single()
        .expect("Could not get primary window in update raycast with cursor");
    let Some(cursor_moved) = cursor.iter().last() else { return };

    let mouse_pos: Vec2 = cursor_moved.position;
    let window_rect: Rect = Rect::from_corners(Vec2::ZERO, vec2(window.width(), window.height()));
    let viewport_rect = app_state.viewport_rect;

    // ratio between viewport size and window size
    let ratio = viewport_rect.size() / window_rect.size();

    // scale the mouse pos so that top left of the window becomes top left of the viewport
    // and bottom right of the window becomes bottom right of the viewport
    let mut scaled_mouse_pos = window_rect.min
        + (mouse_pos - viewport_rect.min) * (window_rect.size() / viewport_rect.size());

    scaled_mouse_pos *= ratio;
    scaled_mouse_pos = scaled_mouse_pos.clamp(Vec2::ZERO, viewport_rect.max);
    scaled_mouse_pos *= window.scale_factor() as f32;

    //grab the most recent cursor event if it exists
    for mut pick_source in &mut query {
        pick_source.cast_method = RaycastMethod::Screenspace(scaled_mouse_pos);
    }
}
