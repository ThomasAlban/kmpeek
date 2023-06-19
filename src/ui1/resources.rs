use bevy::prelude::*;

#[derive(Resource)]
pub struct AppState {
    pub customise_kcl_open: bool,
    pub camera_settings_open: bool,

    pub show_walls: bool,
    pub show_invisible_walls: bool,
    pub show_death_barriers: bool,
    pub show_effects_triggers: bool,

    pub lap_count_buf: String,
    pub speed_mod_buf: String,

    pub look_sensitivity_buf: String,
    pub speed_buf: String,
    pub speed_boost_buf: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            customise_kcl_open: false,
            camera_settings_open: false,

            show_walls: true,
            show_invisible_walls: true,
            show_death_barriers: true,
            show_effects_triggers: true,

            lap_count_buf: String::from("3"),
            speed_mod_buf: String::from("0.0"),

            look_sensitivity_buf: String::from("1.0"),
            speed_buf: String::from("1.0"),
            speed_boost_buf: String::from("3.0"),
        }
    }
}
