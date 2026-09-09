use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub const DEFAULT_GIZMO_SIZE: f32 = 100.0;
pub const MIN_GIZMO_SIZE: f32 = 50.0;
pub const MAX_GIZMO_SIZE: f32 = 200.0;
pub const DEFAULT_GIZMO_LINE_WIDTH: f32 = 6.0;
pub const MIN_GIZMO_LINE_WIDTH: f32 = 1.0;
pub const MAX_GIZMO_LINE_WIDTH: f32 = 16.0;

fn default_gizmo_size() -> f32 {
    DEFAULT_GIZMO_SIZE
}

fn default_gizmo_line_width() -> f32 {
    DEFAULT_GIZMO_LINE_WIDTH
}

#[derive(Resource, Serialize, Deserialize)]
pub struct KmpModelSettings {
    //pub normalize: bool,
    pub point_scale: f32,
    #[serde(default = "default_gizmo_size")]
    pub gizmo_size: f32,
    #[serde(default = "default_gizmo_line_width")]
    pub gizmo_line_width: f32,
    pub color: KmpModelColors,
    pub outline: OutlineSettings,
    pub checkpoint_height: f32,
}
impl Default for KmpModelSettings {
    fn default() -> Self {
        KmpModelSettings {
            //normalize: true,
            point_scale: 1.,
            gizmo_size: DEFAULT_GIZMO_SIZE,
            gizmo_line_width: DEFAULT_GIZMO_LINE_WIDTH,
            color: KmpModelColors::default(),
            outline: OutlineSettings::default(),
            checkpoint_height: 10000.,
        }
    }
}

#[derive(Serialize, Deserialize, Reflect)]
pub struct KmpModelColors {
    pub start_points: PointColor,
    pub enemy_paths: PathColor,
    pub item_paths: PathColor,
    pub checkpoints: CheckpointColor,
    pub respawn_points: PointColor,
    pub objects: PointColor,
    pub routes: PathColor,
    pub areas: PointColor,
    pub cameras: PointColor,
    pub cannon_points: PointColor,
    pub battle_finish_points: PointColor,
}
impl Default for KmpModelColors {
    fn default() -> Self {
        Self {
            start_points: PointColor {
                point: Color::srgb(0., 0., 0.5),
                line: Color::srgba(0.4, 0.4, 1., 0.9),
                arrow: Color::srgb(0., 0., 0.5),
                up_arrow: Color::srgba(0., 0., 0.7, 0.9),
            },
            enemy_paths: PathColor {
                point: Color::srgb(1., 0., 0.),
                line: Color::srgb(1., 0.5, 0.),
                arrow: Color::srgb(1., 1., 0.),
            },
            item_paths: PathColor {
                point: Color::srgb(0., 0.6, 0.),
                line: Color::srgb(0., 1., 0.),
                arrow: Color::srgb(0., 0.6, 0.),
            },
            checkpoints: CheckpointColor {
                normal: Color::srgb(0., 0.55, 0.85),
                key: Color::srgb(1., 0., 0.7),
                lap_count: Color::srgb(1., 0.45, 0.8),
                line: Color::srgb(0.2, 0.75, 0.9),
                arrow: Color::srgb(0.45, 0.8, 0.9),
            },
            objects: PointColor {
                point: Color::srgb(0.8, 0., 0.8),
                line: Color::srgba(1., 0.4, 1., 0.9),
                arrow: Color::srgb(0.8, 0., 0.8),
                up_arrow: Color::srgba(1., 0., 1., 0.9),
            },
            routes: PathColor {
                point: Color::srgb(0., 0.75, 0.75),
                line: Color::srgb(0.3, 1., 1.),
                arrow: Color::srgb(0., 0.6, 0.6),
            },
            areas: PointColor {
                point: Color::srgb(1., 0.5, 0.),
                line: Color::srgb(1., 0.8, 0.),
                arrow: Color::srgb(1., 0.2, 0.),
                up_arrow: Color::srgba(1., 0.8, 0., 0.9),
            },
            cameras: PointColor {
                point: Color::srgb(0.6, 0., 1.),
                line: Color::srgba(0.7, 0.25, 1., 0.9),
                arrow: Color::srgb(0.6, 0., 1.),
                up_arrow: Color::srgba(0.7, 0.25, 1., 0.9),
            },
            respawn_points: PointColor {
                point: Color::srgb(0.5, 0.5, 0.),
                line: Color::srgba(0.9, 0.9, 0., 0.8),
                arrow: Color::srgb(0.75, 0.75, 0.1),
                up_arrow: Color::srgba(0.5, 0.5, 0., 0.9),
            },
            cannon_points: PointColor {
                point: Color::srgb(1., 0.2, 0.),
                line: Color::srgba(1., 0.7, 0.6, 0.8),
                arrow: Color::srgb(0.8, 0.2, 0.),
                up_arrow: Color::srgba(0.8, 0.2, 0., 0.9),
            },
            battle_finish_points: PointColor {
                point: Color::srgb(0.15, 0.55, 0.55),
                line: Color::srgba(0.65, 0.9, 0.9, 0.9),
                arrow: Color::srgb(0.2, 0.7, 0.7),
                up_arrow: Color::srgb(0.2, 0.7, 0.7),
            },
        }
    }
}

#[derive(Serialize, Deserialize, Reflect)]
pub struct PathColor {
    pub point: Color,
    pub line: Color,
    pub arrow: Color,
}

#[derive(Serialize, Deserialize, Reflect)]
pub struct PointColor {
    pub point: Color,
    pub line: Color,
    pub arrow: Color,
    pub up_arrow: Color,
}

#[derive(Serialize, Deserialize, Reflect)]
pub struct CheckpointColor {
    pub normal: Color,
    pub key: Color,
    pub lap_count: Color,
    pub line: Color,
    pub arrow: Color,
}

#[derive(Serialize, Deserialize, Reflect, Clone, Copy)]
pub struct OutlineSettings {
    pub color: Color,
    pub width: f32,
}
impl Default for OutlineSettings {
    fn default() -> Self {
        Self {
            color: Color::srgba(1.0, 1.0, 1.0, 0.3),
            width: 7.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_gizmo_visual_settings_use_defaults() {
        let mut serialized = serde_json::to_value(KmpModelSettings::default()).unwrap();
        let object = serialized.as_object_mut().unwrap();
        object.remove("gizmo_size");
        object.remove("gizmo_line_width");

        let settings: KmpModelSettings = serde_json::from_value(serialized).unwrap();

        assert_eq!(settings.gizmo_size, DEFAULT_GIZMO_SIZE);
        assert_eq!(settings.gizmo_line_width, DEFAULT_GIZMO_LINE_WIDTH);
    }
}
