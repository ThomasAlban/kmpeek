use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Resource, Serialize, Deserialize)]
pub struct KmpModelSettings {
    pub normalize: bool,
    pub point_scale: f32,
    pub sections: KmpModelSectionSettings,
}
impl Default for KmpModelSettings {
    fn default() -> Self {
        KmpModelSettings {
            normalize: true,
            point_scale: 1.,
            sections: KmpModelSectionSettings::default(),
        }
    }
}

// stores whether each section is visible, and the relevant colors for each section
#[derive(Serialize, Deserialize, Reflect)]
pub struct KmpModelSectionSettings {
    pub start_points: KmpSectionSettings<PointColor>,
    pub enemy_paths: KmpSectionSettings<PathColor>,
    pub item_paths: KmpSectionSettings<PathColor>,
    pub checkpoints: KmpSectionSettings<()>,
    pub respawn_points: KmpSectionSettings<()>,
    pub objects: KmpSectionSettings<PointColor>,
    pub routes: KmpSectionSettings<PathColor>,
    pub areas: KmpSectionSettings<PointColor>,
    pub cameras: KmpSectionSettings<PointColor>,
    pub cannon_points: KmpSectionSettings<()>,
    pub battle_finish_points: KmpSectionSettings<()>,
}
impl Default for KmpModelSectionSettings {
    fn default() -> Self {
        Self {
            start_points: KmpSectionSettings::new(PointColor {
                point: Color::rgb(0., 0., 0.5),
                line: Color::rgba(0.4, 0.4, 1., 0.9),
                arrow: Color::rgb(0., 0., 0.5),
                up_arrow: Color::rgba(0., 0., 0.7, 0.9),
            }),
            enemy_paths: KmpSectionSettings::new(PathColor {
                point: Color::rgb(1., 0., 0.),
                line: Color::rgb(1., 0.5, 0.),
                arrow: Color::rgb(1., 1., 0.),
            }),
            item_paths: KmpSectionSettings::new(PathColor {
                point: Color::rgb(0., 0.6, 0.),
                line: Color::rgb(0., 1., 0.),
                arrow: Color::rgb(0., 0.6, 0.),
            }),
            checkpoints: KmpSectionSettings::new(()),
            objects: KmpSectionSettings::new(PointColor {
                point: Color::rgb(0.8, 0., 0.8),
                line: Color::rgba(1., 0.4, 1., 0.9),
                arrow: Color::rgb(0.8, 0., 0.8),
                up_arrow: Color::rgba(1., 0., 1., 0.9),
            }),
            routes: KmpSectionSettings::new(PathColor {
                point: Color::rgb(0., 0.75, 0.75),
                line: Color::rgb(0.3, 1., 1.),
                arrow: Color::rgb(0., 0.6, 0.6),
            }),
            areas: KmpSectionSettings::new(PointColor {
                point: Color::rgb(1., 0.5, 0.),
                line: Color::rgb(1., 0.8, 0.),
                arrow: Color::rgb(1., 0.2, 0.),
                up_arrow: Color::rgba(1., 0.8, 0., 0.9),
            }),
            cameras: KmpSectionSettings::new(PointColor {
                point: Color::rgb(0.6, 0., 1.),
                line: Color::rgba(0.7, 0.25, 1., 0.9),
                arrow: Color::rgb(0.6, 0., 1.),
                up_arrow: Color::rgba(0.7, 0.25, 1., 0.9),
            }),
            respawn_points: KmpSectionSettings::new(()),
            cannon_points: KmpSectionSettings::new(()),
            battle_finish_points: KmpSectionSettings::new(()),
        }
    }
}

#[derive(Serialize, Deserialize, Reflect)]
pub struct KmpSectionSettings<T: Reflect> {
    pub visible: bool,
    pub color: T,
}
impl<T: Reflect> KmpSectionSettings<T> {
    fn new(color: T) -> Self {
        Self {
            visible: false,
            color,
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
