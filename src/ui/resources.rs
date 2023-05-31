use bevy::prelude::*;

#[derive(Resource)]
pub struct UIOptions {
    pub customise_kcl_open: bool,

    pub show_walls: bool,
    pub show_invisible_walls: bool,
    pub show_death_barriers: bool,
    pub show_effects_triggers: bool,
    // pub lap_count_buf: String,
}

impl Default for UIOptions {
    fn default() -> Self {
        Self {
            customise_kcl_open: false,
            show_walls: true,
            show_invisible_walls: true,
            show_death_barriers: true,
            show_effects_triggers: true,
        }
    }
}
