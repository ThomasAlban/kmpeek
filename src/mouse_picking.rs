use bevy::prelude::*;
use bevy_mod_raycast::{
    DefaultPluginState, DefaultRaycastingPlugin, RaycastMethod, RaycastSource, RaycastSystem,
};

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
) {
    // grab the most recent cursor event if it exists
    let Some(cursor_moved) = cursor.iter().last() else { return };
    for mut pick_source in &mut query {
        pick_source.cast_method = RaycastMethod::Screenspace(cursor_moved.position);
    }
}
