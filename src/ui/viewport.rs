use bevy::{
    prelude::*,
    render::render_resource::{Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages},
};
use bevy_egui::{egui::TextureId, EguiUserTextures};

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub struct SetupViewportSet;

pub struct ViewportPlugin;
impl Plugin for ViewportPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Startup,
            // this makes sure all the 'Commands' are completed before moving onto other startup systems
            // so that other startup systems can make use of the Viewport image handle
            (setup_viewport, apply_deferred).chain().in_set(SetupViewportSet),
        );
    }
}

// stores the image which the camera renders to, so that we can display a viewport inside a tab
#[derive(Resource)]
pub struct ViewportImage {
    pub handle: Handle<Image>,
    pub tex_id: TextureId,
}

fn setup_viewport(
    mut commands: Commands,
    mut egui_user_textures: ResMut<EguiUserTextures>,
    mut images: ResMut<Assets<Image>>,
) {
    // this is the texture that will be rendered to
    let image: Image = Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            // initialised as a 1x1 texture (the default extent 3d). This will be immediately updated in the update viewport ui function
            // when we know how large the image should be.
            // can't set this to 0x0 as otherwise it works fine on macos but crashes on windows
            size: Extent3d::default(),
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        ..default()
    };

    // create a handle to the image
    let handle = images.add(image);
    let tex_id = egui_user_textures.add_image(handle.clone());

    commands.insert_resource(ViewportImage { handle, tex_id });
}
