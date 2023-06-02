use bevy::prelude::*;

// components used in queries when you want a particular camera

#[derive(Component)]
pub struct FlyCam;

#[derive(Component)]
pub struct OrbitCam {
    pub focus: Vec3,
    pub radius: f32,
    pub upside_down: bool,
}
impl Default for OrbitCam {
    fn default() -> Self {
        Self {
            focus: Vec3::ZERO,
            radius: 10000.,
            upside_down: false,
        }
    }
}

#[derive(Component)]
pub struct TopDownCam;
