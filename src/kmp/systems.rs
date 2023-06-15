use super::resources::Kmp;
use bevy::prelude::*;

pub fn spawn_model(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    kmp: Res<Kmp>,
) {
    let sphere = meshes.add(
        shape::UVSphere {
            radius: 100.,
            ..default()
        }
        .into(),
    );
    let material = materials.add(Color::rgb(0.3, 0.5, 0.3).into());

    for point in kmp.gobj.entries.iter() {
        commands.spawn((PbrBundle {
            mesh: sphere.clone(),
            material: material.clone(),
            transform: Transform::from_translation(point.position),
            ..Default::default()
        },));
    }
}
