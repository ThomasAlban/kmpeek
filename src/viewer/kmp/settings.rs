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
#[derive(Serialize, Deserialize)]
pub struct KmpModelSectionSettings {
    pub start_points: KmpSectionSettings<Color>,
    pub enemy_paths: KmpSectionSettings<PathColor>,
    pub item_paths: KmpSectionSettings<PathColor>,
    pub objects: KmpSectionSettings<Color>,
    pub areas: KmpSectionSettings<()>,
    pub cameras: KmpSectionSettings<()>,
    pub respawn_points: KmpSectionSettings<()>,
    pub cannon_points: KmpSectionSettings<()>,
    pub finish_points: KmpSectionSettings<()>,
}
impl Default for KmpModelSectionSettings {
    fn default() -> Self {
        Self {
            start_points: KmpSectionSettings::new(Color::rgb(0., 0., 0.5)),
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
            objects: KmpSectionSettings::new(Color::rgb(1., 0., 1.)),
            areas: KmpSectionSettings::new(()),
            cameras: KmpSectionSettings::new(()),
            respawn_points: KmpSectionSettings::new(()),
            cannon_points: KmpSectionSettings::new(()),
            finish_points: KmpSectionSettings::new(()),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct KmpSectionSettings<T> {
    pub visible: bool,
    pub color: T,
}
impl<T> KmpSectionSettings<T> {
    fn new(color: T) -> Self {
        Self {
            visible: false,
            color,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct PathColor {
    pub point: Color,
    pub line: Color,
    pub arrow: Color,
}
