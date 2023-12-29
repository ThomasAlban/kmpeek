use bevy::{
    math::vec2,
    prelude::*,
    render::render_resource::{
        Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    },
};
use bevy_egui::{
    egui::{self, TextureId},
    EguiUserTextures,
};

use super::app_state::AppState;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub struct SetupViewportSet;

pub struct ViewportPlugin;
impl Plugin for ViewportPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Startup,
            // this makes sure all the 'Commands' are completed before moving onto other startup systems
            // so that other startup systems can make use of the Viewport image handle
            (setup_viewport, apply_deferred)
                .chain()
                .in_set(SetupViewportSet),
        );
    }
}

// stores the image which the camera renders to, so that we can display a viewport inside a tab
#[derive(Deref, Resource)]
pub struct ViewportImage(Handle<Image>);

fn setup_viewport(
    mut commands: Commands,
    mut egui_user_textures: ResMut<EguiUserTextures>,
    mut images: ResMut<Assets<Image>>,
) {
    // this is the texture that will be rendered to
    let image: Image = Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size: Extent3d {
                width: 0,
                height: 0,
                ..default()
            },
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        ..default()
    };

    // create a handle to the image
    let image_handle = images.add(image);
    egui_user_textures.add_image(image_handle.clone());

    commands.insert_resource(ViewportImage(image_handle));
}

// function called inside dock_tree.rs to render the viewport
pub fn render_viewport(
    ui: &mut egui::Ui,
    viewport_image: &mut Image,
    window: &Window,
    viewport_tex_id: TextureId,
    app_state: &mut AppState,
) {
    let viewport_size = vec2(ui.available_width(), ui.available_height());

    // resize the viewport if needed
    if viewport_image.size() != (viewport_size.as_uvec2() * window.scale_factor() as u32) {
        let size = Extent3d {
            width: viewport_size.x as u32 * window.scale_factor() as u32,
            height: viewport_size.y as u32 * window.scale_factor() as u32,
            ..default()
        };
        viewport_image.resize(size);
    }

    // show the viewport image
    ui.image(egui::load::SizedTexture::new(
        viewport_tex_id,
        viewport_size.to_array(),
    ));

    app_state.mouse_in_viewport = ui.ui_contains_pointer();
    let rect = ui.max_rect();
    app_state.viewport_rect = Rect::new(rect.min.x, rect.min.y, rect.max.x, rect.max.y);
}
