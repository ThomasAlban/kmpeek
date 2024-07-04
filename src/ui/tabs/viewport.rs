use super::UiSubSection;
use crate::{
    ui::{
        settings::AppSettings,
        util::{button_triggered_popup, image_selectable_value, Icons},
        viewport::{ViewportImage, ViewportInfo},
    },
    util::ToEguiRect,
    viewer::{
        camera::{CameraMode, CameraModeChanged},
        edit::{select::SelectBox, EditMode},
    },
};
use bevy::{ecs::system::SystemParam, math::vec2, prelude::*, render::render_resource::Extent3d};
use bevy_egui::egui::{self, Color32, Margin, Response, Rounding, Sense, Stroke, Ui};
use transform_gizmo_bevy::{config::TransformPivotPoint, GizmoOptions, GizmoOrientation};

#[derive(SystemParam)]
struct ViewportParams<'w, 's> {
    q_window: Query<'w, 's, &'static Window>,
    image_assets: ResMut<'w, Assets<Image>>,
    viewport: ResMut<'w, ViewportImage>,
    viewport_info: ResMut<'w, ViewportInfo>,
    edit_mode: ResMut<'w, EditMode>,
    select_box: Res<'w, SelectBox>,
    settings: ResMut<'w, AppSettings>,
    ev_camera_mode_changed: EventWriter<'w, CameraModeChanged>,
    gizmo_options: ResMut<'w, GizmoOptions>,
}

#[derive(SystemParam)]
pub struct ShowViewportTab<'w, 's> {
    p: ViewportParams<'w, 's>,
}
impl UiSubSection for ShowViewportTab<'_, '_> {
    fn show(&mut self, ui: &mut egui::Ui) {
        let window = self.p.q_window.get_single().unwrap();
        let window_sf = window.scale_factor();

        let viewport_image = self.p.image_assets.get_mut(self.p.viewport.handle.id()).unwrap();

        let viewport_top_left = vec2(ui.next_widget_position().x, ui.next_widget_position().y);
        // make sure we don't go above 2000 because otherwise igpus may run out of memory especially when multiplying by a window scale factor above 1
        // this fixes a weird error I experienced on windows where for one frame the viewport image size would be strangely large and crash the igpu
        let viewport_bottom_right = vec2(ui.max_rect().max.x, ui.max_rect().max.y).min(Vec2::splat(2000.));

        let viewport_rect = Rect::from_corners(viewport_top_left, viewport_bottom_right);
        let egui_viewport_rect = viewport_rect.to_egui_rect();

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

        ui.allocate_ui_at_rect(egui_viewport_rect, |ui| {
            // make the image sense clicks and drags, so that any events that aren't consumed by buttons above it are consumed by this
            // so we don't start dragging around the window when trying to select stuff etc
            ui.add(
                egui::Image::new(egui::load::SizedTexture::new(
                    self.p.viewport.tex_id,
                    viewport_rect.size().to_array(),
                ))
                .sense(Sense::click_and_drag()),
            )
        });

        self.p.viewport_info.mouse_in_viewport = ui.rect_contains_pointer(egui_viewport_rect);

        self.p.viewport_info.viewport_rect = viewport_rect;

        self.show_select_box(ui, egui_viewport_rect);

        let responses = self.show_overlayed_ui(ui, egui_viewport_rect);

        self.p.viewport_info.mouse_on_overlayed_ui = responses.iter().any(|x| x.contains_pointer());
    }
}

impl ShowViewportTab<'_, '_> {
    fn show_overlayed_ui(&mut self, ui: &mut Ui, viewport_rect: egui::Rect) -> Vec<Response> {
        let mut responses = Vec::new();
        // viewport overlayed ui
        ui.allocate_ui_at_rect(viewport_rect, |ui| {
            ui.style_mut().spacing.item_spacing = egui::Vec2::splat(5.);

            egui::Frame::none().inner_margin(Margin::same(5.)).show(ui, |ui| {
                // popups for things such as gizmo options, camera options, etc
                ui.horizontal(|ui| {
                    let gizmo_options_btn = ui.button("Gizmo Options");
                    responses.push(gizmo_options_btn.clone());
                    let r = button_triggered_popup(ui, "gizmo_options_popup", gizmo_options_btn, |ui| {
                        ui.style_mut().spacing.button_padding = egui::Vec2::ZERO;
                        let size = 25.;
                        ui.label("Pivot:");
                        ui.horizontal(|ui| {
                            let pivot = &mut self.p.gizmo_options.pivot_point;
                            image_selectable_value(
                                ui,
                                pivot,
                                TransformPivotPoint::MedianPoint,
                                Icons::pivot_median(ui.ctx(), size),
                                size,
                            )
                            .on_hover_text_at_pointer("Median point");
                            image_selectable_value(
                                ui,
                                pivot,
                                TransformPivotPoint::IndividualOrigins,
                                Icons::pivot_individual(ui.ctx(), size),
                                size,
                            )
                            .on_hover_text_at_pointer("Individual origins");
                        });

                        ui.label("Orientation:");
                        ui.horizontal(|ui| {
                            let orientation = &mut self.p.gizmo_options.gizmo_orientation;
                            image_selectable_value(
                                ui,
                                orientation,
                                GizmoOrientation::Global,
                                Icons::orient_global(ui.ctx(), size),
                                size,
                            )
                            .on_hover_text_at_pointer("Global orientation");
                            image_selectable_value(
                                ui,
                                orientation,
                                GizmoOrientation::Local,
                                Icons::orient_local(ui.ctx(), size),
                                size,
                            )
                            .on_hover_text_at_pointer("Local orientation");
                        });
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut self.p.gizmo_options.group_targets, "Group targets")
                                .on_hover_text_at_pointer(
                                    "Use a single gizmo for all targets, rather than individual gizmos",
                                )
                        });
                    });
                    if let Some(r) = r {
                        responses.push(r);
                    }

                    let camera_mode = &mut self.p.settings.camera.mode;
                    let camera_btn = ui.button(format!("Camera: {}", camera_mode));
                    responses.push(camera_btn.clone());
                    let r = button_triggered_popup(ui, "camera_button_popup", camera_btn, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Camera Mode:");
                            if ui.selectable_value(camera_mode, CameraMode::Fly, "Fly").clicked() {
                                self.p.ev_camera_mode_changed.send(CameraModeChanged(CameraMode::Fly));
                            }
                            if ui.selectable_value(camera_mode, CameraMode::Orbit, "Orbit").clicked() {
                                self.p.ev_camera_mode_changed.send(CameraModeChanged(CameraMode::Orbit));
                            }
                            if ui
                                .selectable_value(camera_mode, CameraMode::TopDown, "Top Down")
                                .clicked()
                            {
                                self.p
                                    .ev_camera_mode_changed
                                    .send(CameraModeChanged(CameraMode::TopDown));
                            }
                        });
                    });
                    if let Some(r) = r {
                        responses.push(r);
                    }
                });
                // cursor/gizmo mode
                let vertical_res = ui
                    .vertical(|ui| {
                        ui.style_mut().spacing.button_padding = egui::Vec2::ZERO;
                        let mode = &mut *self.p.edit_mode;
                        let size = 35.;

                        image_selectable_value(ui, mode, EditMode::Tweak, Icons::tweak(ui.ctx(), size), size)
                            .on_hover_text_at_pointer("Drag points around freely");
                        image_selectable_value(ui, mode, EditMode::SelectBox, Icons::select_box(ui.ctx(), size), size)
                            .on_hover_text_at_pointer("Select points with a selection box");
                        image_selectable_value(ui, mode, EditMode::Translate, Icons::translate(ui.ctx(), size), size)
                            .on_hover_text_at_pointer("Translate points with a gizmo");
                        image_selectable_value(ui, mode, EditMode::Rotate, Icons::rotate(ui.ctx(), size), size)
                            .on_hover_text_at_pointer("Rotate points with a gizmo");
                    })
                    .response;
                responses.push(vertical_res);
            });
        });
        responses
    }

    fn show_select_box(&mut self, ui: &mut Ui, viewport_rect: egui::Rect) {
        ui.allocate_ui_at_rect(viewport_rect, |ui| {
            ui.set_clip_rect(viewport_rect);
            let painter = ui.painter();
            if let Some(select_box) = self.p.select_box.0 {
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
