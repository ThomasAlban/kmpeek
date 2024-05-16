use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Resource, Serialize, Deserialize)]
pub struct KmpModelSettings {
    pub normalize: bool,
    pub point_scale: f32,
    pub color: KmpModelColors,
    pub outline: OutlineSettings,
    pub checkpoint_height: f32,
}
impl Default for KmpModelSettings {
    fn default() -> Self {
        KmpModelSettings {
            normalize: true,
            point_scale: 1.,
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
    pub checkpoints: CheckpointColour,
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
                point: Color::rgb(0., 0., 0.5),
                line: Color::rgba(0.4, 0.4, 1., 0.9),
                arrow: Color::rgb(0., 0., 0.5),
                up_arrow: Color::rgba(0., 0., 0.7, 0.9),
            },
            enemy_paths: PathColor {
                point: Color::rgb(1., 0., 0.),
                line: Color::rgb(1., 0.5, 0.),
                arrow: Color::rgb(1., 1., 0.),
            },
            item_paths: PathColor {
                point: Color::rgb(0., 0.6, 0.),
                line: Color::rgb(0., 1., 0.),
                arrow: Color::rgb(0., 0.6, 0.),
            },
            checkpoints: CheckpointColour {
                normal: Color::rgb(0., 0.55, 0.85),
                key: Color::rgb(1., 0., 0.7),
                lap_count: Color::rgb(1., 0.45, 0.8),
                line: Color::rgb(0.2, 0.75, 0.9),
                arrow: Color::rgb(0.45, 0.8, 0.9),
            },
            objects: PointColor {
                point: Color::rgb(0.8, 0., 0.8),
                line: Color::rgba(1., 0.4, 1., 0.9),
                arrow: Color::rgb(0.8, 0., 0.8),
                up_arrow: Color::rgba(1., 0., 1., 0.9),
            },
            routes: PathColor {
                point: Color::rgb(0., 0.75, 0.75),
                line: Color::rgb(0.3, 1., 1.),
                arrow: Color::rgb(0., 0.6, 0.6),
            },
            areas: PointColor {
                point: Color::rgb(1., 0.5, 0.),
                line: Color::rgb(1., 0.8, 0.),
                arrow: Color::rgb(1., 0.2, 0.),
                up_arrow: Color::rgba(1., 0.8, 0., 0.9),
            },
            cameras: PointColor {
                point: Color::rgb(0.6, 0., 1.),
                line: Color::rgba(0.7, 0.25, 1., 0.9),
                arrow: Color::rgb(0.6, 0., 1.),
                up_arrow: Color::rgba(0.7, 0.25, 1., 0.9),
            },
            respawn_points: PointColor {
                point: Color::rgb(0.5, 0.5, 0.),
                line: Color::rgba(0.9, 0.9, 0., 0.8),
                arrow: Color::rgb(0.75, 0.75, 0.1),
                up_arrow: Color::rgba(0.5, 0.5, 0., 0.9),
            },
            cannon_points: PointColor {
                point: Color::rgb(1., 0.2, 0.),
                line: Color::rgba(1., 0.7, 0.6, 0.8),
                arrow: Color::rgb(0.8, 0.2, 0.),
                up_arrow: Color::rgba(0.8, 0.2, 0., 0.9),
            },
            battle_finish_points: PointColor {
                point: Color::rgb(0.15, 0.55, 0.55),
                line: Color::rgba(0.65, 0.9, 0.9, 0.9),
                arrow: Color::rgb(0.2, 0.7, 0.7),
                up_arrow: Color::rgb(0.2, 0.7, 0.7),
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
pub struct CheckpointColour {
    pub normal: Color,
    pub key: Color,
    pub lap_count: Color,
    pub line: Color,
    pub arrow: Color,
}

#[derive(Serialize, Deserialize, Reflect, Clone)]
pub struct OutlineSettings {
    pub color: Color,
    pub width: f32,
}
impl Default for OutlineSettings {
    fn default() -> Self {
        Self {
            color: Color::rgba(1.0, 1.0, 1.0, 0.3),
            width: 7.0,
        }
    }
}
