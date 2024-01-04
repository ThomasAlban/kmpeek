use crate::ui::viewport::ViewportImage;

use super::super::app_state::AppState;
use bevy::{
    ecs::system::SystemParam, math::vec2, prelude::*, render::render_resource::Extent3d,
    window::PrimaryWindow,
};
use bevy_egui::egui;

#[derive(SystemParam)]
pub struct ViewportParams<'w, 's> {
    app_state: ResMut<'w, AppState>,
    window: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    image_assets: ResMut<'w, Assets<Image>>,
    viewport: ResMut<'w, ViewportImage>,
}

pub fn show_viewport_tab(ui: &mut egui::Ui, p: &mut ViewportParams) {
    let viewport_image = p.image_assets.get_mut(p.viewport.handle.id()).unwrap();
    // let viewport_tex_id = p.contexts.image_id(&p.viewport).unwrap();
    let window = p.window.get_single().unwrap();

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
        p.viewport.tex_id,
        viewport_size.to_array(),
    ));

    p.app_state.mouse_in_viewport = ui.ui_contains_pointer();
    let rect = ui.max_rect();
    p.app_state.viewport_rect = Rect::new(rect.min.x, rect.min.y, rect.max.x, rect.max.y);
}

// // function called inside dock_tree.rs to render the viewport
// pub fn render_viewport(
//     ui: &mut egui::Ui,
//     viewport_image: &mut Image,
//     window: &Window,
//     viewport_tex_id: TextureId,
//     app_state: &mut AppState,
// ) {
//     let viewport_size = vec2(ui.available_width(), ui.available_height());

//     // resize the viewport if needed
//     if viewport_image.size() != (viewport_size.as_uvec2() * window.scale_factor() as u32) {
//         let size = Extent3d {
//             width: viewport_size.x as u32 * window.scale_factor() as u32,
//             height: viewport_size.y as u32 * window.scale_factor() as u32,
//             ..default()
//         };
//         viewport_image.resize(size);
//     }

//     // show the viewport image
//     ui.image(egui::load::SizedTexture::new(
//         viewport_tex_id,
//         viewport_size.to_array(),
//     ));

//     app_state.mouse_in_viewport = ui.ui_contains_pointer();
//     let rect = ui.max_rect();
//     app_state.viewport_rect = Rect::new(rect.min.x, rect.min.y, rect.max.x, rect.max.y);
// }
