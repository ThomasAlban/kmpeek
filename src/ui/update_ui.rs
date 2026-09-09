use super::{file_dialog::show_file_dialog, menu_bar::show_menu_bar, tabs::show_dock_area};
use bevy::{camera::visibility::RenderLayers, prelude::*, transform::TransformSystems, window::PrimaryWindow};
use bevy_egui::{EguiContexts, EguiGlobalSettings, EguiPostUpdateSet, EguiPrimaryContextPass, PrimaryEguiContext};
use std::path::PathBuf;

#[derive(SystemSet, Hash, PartialEq, Eq, Clone, Debug)]
pub struct UpdateUiSet;

#[derive(SystemSet, Hash, PartialEq, Eq, Clone, Debug)]
pub enum FileLoadSet {
    Select,
    Load,
}

pub fn update_ui_plugin(app: &mut App) {
    app.configure_sets(
        PostUpdate,
        EguiPostUpdateSet::EndPass.after(TransformSystems::Propagate),
    )
    .configure_sets(Update, (FileLoadSet::Select, FileLoadSet::Load).chain())
    .add_message::<KmpFileSelected>()
    .add_message::<KclFileSelected>()
    .add_systems(Startup, setup_primary_egui_camera)
    .add_systems(
        EguiPrimaryContextPass,
        setup_ui_images.before(UpdateUiSet).run_if(ui_is_ready),
    )
    .add_systems(
        EguiPrimaryContextPass,
        update_ui.in_set(UpdateUiSet).run_if(ui_is_ready),
    );
}

fn setup_primary_egui_camera(mut commands: Commands, mut settings: ResMut<EguiGlobalSettings>) {
    // All editor cameras render to the offscreen viewport image. If bevy_egui
    // automatically selects the first camera, its UI is therefore only 1x1 at startup.
    settings.auto_create_primary_context = false;
    commands.spawn((
        Camera2d,
        Camera {
            // Render the editor scene and its 2D gizmos before sampling the
            // offscreen viewport texture into the window UI.
            order: 2,
            ..default()
        },
        PrimaryEguiContext,
        RenderLayers::none(),
    ));
}

fn ui_is_ready(windows: Query<(), With<PrimaryWindow>>, contexts: Query<(), With<PrimaryEguiContext>>) -> bool {
    !windows.is_empty() && !contexts.is_empty()
}

#[derive(Message, Deref, DerefMut)]
pub struct KmpFileSelected(pub PathBuf);

#[derive(Message, Deref, DerefMut)]
pub struct KclFileSelected(pub PathBuf);

fn setup_ui_images(mut contexts: EguiContexts) {
    egui_extras::install_image_loaders(contexts.ctx_mut().unwrap());
}

fn update_ui(world: &mut World) {
    show_menu_bar(world);
    show_dock_area(world);
    show_file_dialog(world);
    world.flush();
}
