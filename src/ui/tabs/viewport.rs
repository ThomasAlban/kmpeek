use crate::{
    ui::{
        ui_state::{MouseInViewport, ViewportRect},
        update_ui::UiImages,
        viewport::ViewportImage,
    },
    viewer::transform::gizmo::ShowGizmo,
};

use super::UiSubSection;
use bevy::{
    ecs::system::SystemParam, math::vec2, prelude::*, render::render_resource::Extent3d,
    window::PrimaryWindow,
};
use bevy_egui::egui::{self, generate_loader_id, ImageButton};

#[derive(SystemParam)]
struct ViewportParams<'w, 's> {
    window: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    image_assets: ResMut<'w, Assets<Image>>,
    viewport: ResMut<'w, ViewportImage>,
}

#[derive(SystemParam)]
pub struct ShowViewportTab<'w, 's> {
    p: ParamSet<'w, 's, (ViewportParams<'w, 's>, ShowGizmo<'w, 's>)>,
    mouse_in_viewport: ResMut<'w, MouseInViewport>,
    viewport_rect: ResMut<'w, ViewportRect>,
    // ui_images: Res<'w, UiImages>,
}
impl UiSubSection for ShowViewportTab<'_, '_> {
    fn show(&mut self, ui: &mut egui::Ui) {
        let mut p = self.p.p0();
        let viewport_image = p.image_assets.get_mut(p.viewport.handle.id()).unwrap();
        // let viewport_tex_id = p.contexts.image_id(&p.viewport).unwrap();
        let window = p.window.get_single().unwrap();

        let rect = ui.min_rect();

        let viewport_size = vec2(rect.width(), rect.height());

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
        let rect = ui.max_rect();

        self.mouse_in_viewport.0 = ui.ui_contains_pointer();
        self.viewport_rect.0 = Rect::new(rect.min.x, rect.min.y, rect.max.x, rect.max.y);
        self.p.p1().show(ui);

        // ui.allocate_ui_at_rect(ui.max_rect(), |ui| {
        //     let image = egui::include_image!("../../../assets/icons/translate.png");
        //     ui.add(ImageButton::new(image));
        // });
    }
}
