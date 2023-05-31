use bevy::prelude::*;

// this is a component attached to every part of the KCL model so that we know which bit it is when querying
#[derive(Component)]
pub struct KCLModelSection(pub usize);
