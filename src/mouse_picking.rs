use bevy::{math::vec2, prelude::*, window::PrimaryWindow};
use bevy_mod_raycast::{
    DefaultPluginState, DefaultRaycastingPlugin, RaycastMethod, RaycastSource, RaycastSystem,
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
            );
    }
}

#[derive(Reflect)]
pub struct RaycastSet;

fn setup(mut commands: Commands) {
    commands.insert_resource(DefaultPluginState::<RaycastSet>::default().with_debug_cursor());
}

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

    let mut scaled_mouse_pos = vec2_scale(
        mouse_pos,
        viewport_rect.min,
        viewport_rect.max,
        window_rect.min,
        window_rect.max,
    );

    scaled_mouse_pos = scaled_mouse_pos.clamp(Vec2::ZERO, viewport_rect.max);

    println!("\n\nmouse pos: {mouse_pos}\nscaled: {scaled_mouse_pos}");

    //grab the most recent cursor event if it exists
    for mut pick_source in &mut query {
        pick_source.cast_method = RaycastMethod::Screenspace(scaled_mouse_pos);
    }
}

fn vec2_scale(x: Vec2, old_min: Vec2, old_max: Vec2, new_min: Vec2, new_max: Vec2) -> Vec2 {
    let old_range = old_max - old_min;
    let new_range = new_max - new_min;

    new_min + (x - old_min) * new_range / old_range
}
