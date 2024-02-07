use super::UiSubSection;
use crate::{
    ui::{
        settings::AppSettings,
        ui_state::{MouseInViewport, ViewportRect},
        util::{button_triggered_popup, image_selectable_value, Icons},
        viewport::ViewportImage,
    },
    viewer::{
        camera::{CameraMode, CameraModeChanged},
        edit::{
            gizmo::{GizmoOptions, GizmoOrigin, ShowGizmo},
            select::SelectBox,
            EditMode,
        },
    },
};
use bevy::{
    ecs::system::SystemParam, math::vec2, prelude::*, render::render_resource::Extent3d,
    window::PrimaryWindow,
};
use bevy_egui::egui::{self, Color32, Margin, Pos2, Rounding, Stroke, Ui};
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
    settings: ResMut<'w, AppSettings>,
    ev_camera_mode_changed: EventWriter<'w, CameraModeChanged>,
}

#[derive(SystemParam)]
pub struct ShowViewportTab<'w, 's> {
    p: ParamSet<'w, 's, (ViewportParams<'w, 's>, ShowGizmo<'w, 's>)>,
}
impl UiSubSection for ShowViewportTab<'_, '_> {
    fn show(&mut self, ui: &mut egui::Ui) {
        let mut p = self.p.p0();
        let window = p.q_window.get_single().unwrap();
        let window_sf = window.scale_factor() as f32;

        let viewport_image = p.image_assets.get_mut(p.viewport.handle.id()).unwrap();
        // let viewport_tex_id = p.contexts.image_id(&p.viewport).unwrap();
        // let window = p.q_window.get_single().unwrap();

        let viewport_top_left = vec2(ui.next_widget_position().x, ui.next_widget_position().y);
        // make sure we don't go above 2000 because otherwise igpus may run out of memory especially when multiplying by a window scale factor above 1
        // this fixes a weird error I experienced on windows where for one frame the viewport image size would be strangely large and crash the igpu
        let viewport_bottom_right =
            vec2(ui.max_rect().max.x, ui.max_rect().max.y).min(Vec2::splat(2000.));

        let viewport_rect = Rect::from_corners(viewport_top_left, viewport_bottom_right);
        // exact same thing as above but in egui's format
        let egui_viewport_rect = egui::Rect::from_min_max(
            Pos2 {
                x: viewport_rect.min.x,
                y: viewport_rect.min.y,
            },
            Pos2 {
                x: viewport_rect.max.x,
                y: viewport_rect.max.y,
            },
        );

        // resize the viewport if needed
        if viewport_image.size() != (viewport_rect.size().as_uvec2() * window_sf as u32) {
            let size = Extent3d {
                width: viewport_rect.size().x as u32 * window_sf as u32,
                height: viewport_rect.size().y as u32 * window_sf as u32,
                ..default()
            };
            viewport_image.resize(size);
        }

        // show the viewport image
        ui.image(egui::load::SizedTexture::new(
            p.viewport.tex_id,
            viewport_rect.size().to_array(),
        ));

        p.mouse_in_viewport.0 = ui.rect_contains_pointer(egui_viewport_rect);

        p.viewport_rect.0 = viewport_rect;

        // gizmo
        self.p.p1().show(ui);

        self.show_select_box(ui, egui_viewport_rect);

        self.show_overlayed_ui(ui, egui_viewport_rect);
    }
}

impl ShowViewportTab<'_, '_> {
    fn show_overlayed_ui(&mut self, ui: &mut Ui, viewport_rect: egui::Rect) {
        let mut p = self.p.p0();
        // viewport overlayed ui
        ui.allocate_ui_at_rect(viewport_rect, |ui| {
            ui.style_mut().spacing.item_spacing = egui::Vec2::splat(5.);

            egui::Frame::none()
                .inner_margin(Margin::same(5.))
                .show(ui, |ui| {
                    // popups for things such as gizmo options, camera options, etc
                    ui.horizontal(|ui| {
                        let gizmo_options_btn = ui.button("Gizmo Options");
                        button_triggered_popup(ui, "gizmo_options_popup", gizmo_options_btn, |ui| {
                            ui.style_mut().spacing.button_padding = egui::Vec2::ZERO;
                            let size = 25.;
                            ui.label("Origin:");
                            ui.horizontal(|ui| {
                                let origin = &mut p.gizmo_options.gizmo_origin;

                                image_selectable_value(
                                    ui,
                                    origin,
                                    GizmoOrigin::Mean,
                                    Icons::origin_mean(ui.ctx(), size),
                                    size,
                                )
                                .on_hover_text_at_pointer("Takes the mean average of each point's position, and places the gizmo there");
                                image_selectable_value(
                                    ui,
                                    origin,
                                    GizmoOrigin::FirstSelected,
                                    Icons::origin_first_selected(ui.ctx(), size),
                                    size,
                                )
                                .on_hover_text_at_pointer("Places the gizmo at the first selected point");
                                image_selectable_value(
                                    ui,
                                    origin,
                                    GizmoOrigin::Individual,
                                    Icons::origin_individual(ui.ctx(), size),
                                    size,
                                )
                                .on_hover_text_at_pointer("Places the gizmo at the first selected point, with each point's origin as its own");
                            });

                            ui.label("Orientation");
                            ui.horizontal(|ui| {
                                let orient = &mut p.gizmo_options.gizmo_orientation;
                                image_selectable_value(
                                    ui,
                                    orient,
                                    GizmoOrientation::Global,
                                    Icons::orient_global(ui.ctx(), size),
                                    size,
                                )
                                .on_hover_text_at_pointer("Orient the gizmo to the global space");
                                image_selectable_value(
                                    ui,
                                    orient,
                                    GizmoOrientation::Local,
                                    Icons::orient_local(ui.ctx(), size),
                                    size,
                                )
                                .on_hover_text_at_pointer("Orient the gizmo to the selected point");
                            });
                        });

                        let camera_mode = &mut p.settings.camera.mode;
                        let camera_btn = ui.button(format!("Camera: {}", camera_mode));
                        button_triggered_popup(ui, "camera_button_popup", camera_btn, |ui| {
                            ui.horizontal(|ui| {
                                ui.label("Camera Mode:");
                                if ui
                                    .selectable_value(camera_mode, CameraMode::Fly, "Fly")
                                    .clicked()
                                {
                                    p.ev_camera_mode_changed
                                        .send(CameraModeChanged(CameraMode::Fly));
                                }
                                if ui
                                    .selectable_value(camera_mode, CameraMode::Orbit, "Orbit")
                                    .clicked()
                                {
                                    p.ev_camera_mode_changed
                                        .send(CameraModeChanged(CameraMode::Orbit));
                                }
                                if ui
                                    .selectable_value(camera_mode, CameraMode::TopDown, "Top Down")
                                    .clicked()
                                {
                                    p.ev_camera_mode_changed
                                        .send(CameraModeChanged(CameraMode::TopDown));
                                }
                            });
                        });
                    });
                    // cursor/gizmo mode
                    ui.vertical(|ui| {
                        ui.style_mut().spacing.button_padding = egui::Vec2::ZERO;
                        let mode = &mut *p.edit_mode;
                        let size = 35.;

                        image_selectable_value(
                            ui,
                            mode,
                            EditMode::Tweak,
                            Icons::tweak(ui.ctx(), size),
                            size,
                        )
                        .on_hover_text_at_pointer("Drag points around freely");
                        image_selectable_value(
                            ui,
                            mode,
                            EditMode::SelectBox,
                            Icons::select_box(ui.ctx(), size),
                            size,
                        )
                        .on_hover_text_at_pointer("Select points with a selection box");
                        image_selectable_value(
                            ui,
                            mode,
                            EditMode::Translate,
                            Icons::translate(ui.ctx(), size),
                            size,
                        )
                        .on_hover_text_at_pointer("Translate points with a gizmo");
                        image_selectable_value(
                            ui,
                            mode,
                            EditMode::Rotate,
                            Icons::rotate(ui.ctx(), size),
                            size,
                        )
                        .on_hover_text_at_pointer("Rotate points with a gizmo");
                    });


                });

        });
    }

    fn show_select_box(&mut self, ui: &mut Ui, viewport_rect: egui::Rect) {
        let p = self.p.p0();
        ui.allocate_ui_at_rect(viewport_rect, |ui| {
            ui.set_clip_rect(viewport_rect);
            let painter = ui.painter();
            if let Some(select_box) = p.select_box.unscaled {
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
    }
}
