use crate::{
    ui::{
        ui_state::{MouseInViewport, ViewportRect},
        util::image_selectable_value,
        viewport::ViewportImage,
    },
    viewer::transform::{
        gizmo::{GizmoOptions, GizmoOrigin, ShowGizmo},
        select::SelectBox,
        EditMode,
    },
};

use super::UiSubSection;
use bevy::{
    ecs::system::SystemParam, math::vec2, prelude::*, render::render_resource::Extent3d,
    window::PrimaryWindow,
};
use bevy_egui::egui::{self, include_image, Color32, Margin, Rounding, Stroke};
use egui_gizmo::GizmoOrientation;

#[derive(SystemParam)]
struct ViewportParams<'w, 's> {
    q_window: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    image_assets: ResMut<'w, Assets<Image>>,
    viewport: ResMut<'w, ViewportImage>,
    mouse_in_viewport: ResMut<'w, MouseInViewport>,
    viewport_rect: ResMut<'w, ViewportRect>,
    edit_mode: ResMut<'w, EditMode>,
    gizmo_options: ResMut<'w, GizmoOptions>,
    select_box: Res<'w, SelectBox>,
}

#[derive(SystemParam)]
pub struct ShowViewportTab<'w, 's> {
    p: ParamSet<'w, 's, (ViewportParams<'w, 's>, ShowGizmo<'w, 's>)>,
}
impl UiSubSection for ShowViewportTab<'_, '_> {
    fn show(&mut self, ui: &mut egui::Ui) {
        let mut p = self.p.p0();
        let viewport_image = p.image_assets.get_mut(p.viewport.handle.id()).unwrap();
        // let viewport_tex_id = p.contexts.image_id(&p.viewport).unwrap();
        let window = p.q_window.get_single().unwrap();

        let rect = ui.min_rect();

        // set the viewport size to how much ui space there is, but making sure we don't go above 2000 because otherwise igpus may run out of memory
        // especially when multiplying by a window scale factor above 1
        // this fixes a weird error I experienced on windows where for one frame the viewport image size would be strangely large and crash the igpu
        let viewport_size = vec2(rect.width(), rect.height()).min(Vec2::splat(2000.));

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

        self.p.p0().mouse_in_viewport.0 = ui.ui_contains_pointer();
        self.p.p0().viewport_rect.0 = Rect::new(rect.min.x, rect.min.y, rect.max.x, rect.max.y);

        // gizmo
        self.p.p1().show(ui);

        // select box
        ui.allocate_ui_at_rect(ui.max_rect(), |ui| {
            ui.set_clip_rect(ui.max_rect());
            let painter = ui.painter();
            if let Some(select_box) = self.p.p0().select_box.unscaled {
                let select_box = egui::Rect {
                    min: egui::Pos2 {
                        x: select_box.min.x,
                        y: select_box.min.y,
                    },
                    max: egui::Pos2 {
                        x: select_box.max.x,
                        y: select_box.max.y,
                    },
                };
                painter.rect(
                    select_box,
                    Rounding::from(2.),
                    Color32::from_rgba_unmultiplied(200, 200, 200, 15),
                    Stroke {
                        width: 1.,
                        color: Color32::GRAY,
                    },
                );
            }
        });

        // viewport overlayed ui
        ui.allocate_ui_at_rect(ui.max_rect(), |ui| {
            ui.style_mut().spacing.item_spacing = egui::Vec2::splat(5.);

            egui::Frame::none()
                .inner_margin(Margin::same(5.))
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.style_mut().spacing.button_padding = egui::Vec2::ZERO;
                        ui.style_mut().interaction.tooltip_delay = 10.;

                        ui.collapsing(
                            egui::RichText::new("Gizmo Options").color(egui::Color32::WHITE),
                            |ui| {
                                ui.label(egui::RichText::new("Origin").color(egui::Color32::WHITE));
                                ui.horizontal(|ui| {
                                    let origin = &mut self.p.p0().gizmo_options.gizmo_origin;
                                    let size = 25.;

                                    image_selectable_value(
                                        ui,
                                        size,
                                        origin,
                                        GizmoOrigin::Mean,
                                        include_image!("../../../assets/icons/origin_mean.svg"),
                                    );
                                    // .on_hover_text("Takes the mean average of each point's position, and places the gizmo there");
                                    image_selectable_value(
                                        ui,
                                        size,
                                        origin,
                                        GizmoOrigin::FirstSelected,
                                        include_image!(
                                            "../../../assets/icons/origin_first_selected.svg"
                                        ),
                                    );
                                    // .on_hover_text("Positions the gizmo at the first selected point");
                                    image_selectable_value(
                                        ui,
                                        size,
                                        origin,
                                        GizmoOrigin::Individual,
                                        include_image!(
                                            "../../../assets/icons/origin_individual.svg"
                                        ),
                                    );
                                    // .on_hover_text("Positions the gizmo at the first selected point, with each point's origin as its own");
                                });

                                ui.label(
                                    egui::RichText::new("Orientation").color(egui::Color32::WHITE),
                                );
                                ui.horizontal(|ui| {
                                    let orient = &mut self.p.p0().gizmo_options.gizmo_orientation;
                                    let size = 25.;
                                    image_selectable_value(
                                        ui,
                                        size,
                                        orient,
                                        GizmoOrientation::Global,
                                        include_image!("../../../assets/icons/orient_global.svg"),
                                    );
                                    // .on_hover_text("Orient the gizmo to the global space");
                                    image_selectable_value(
                                        ui,
                                        size,
                                        orient,
                                        GizmoOrientation::Local,
                                        include_image!("../../../assets/icons/orient_local.svg"),
                                    );
                                    // .on_hover_text("Orient the gizmo to the selected point");
                                });
                            },
                        );
                        let mode = &mut *self.p.p0().edit_mode;
                        let size = 35.;
                        image_selectable_value(
                            ui,
                            size,
                            mode,
                            EditMode::Tweak,
                            include_image!("../../../assets/icons/tweak.svg"),
                        );
                        // .on_hover_text("Drag points around freely");
                        image_selectable_value(
                            ui,
                            size,
                            mode,
                            EditMode::SelectBox,
                            include_image!("../../../assets/icons/select_box.svg"),
                        );
                        // .on_hover_text("Drag points around freely");
                        image_selectable_value(
                            ui,
                            size,
                            mode,
                            EditMode::Translate,
                            include_image!("../../../assets/icons/translate.svg"),
                        );
                        // .on_hover_text("Translate points with a gizmo");
                        image_selectable_value(
                            ui,
                            size,
                            mode,
                            EditMode::Rotate,
                            include_image!("../../../assets/icons/rotate.svg"),
                        );
                        // .on_hover_text("Rotate points with a gizmo");
                    });
                });
        });
    }
}
