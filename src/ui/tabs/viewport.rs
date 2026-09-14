use crate::{
    ui::{
        settings::AppSettings,
        util::{button_triggered_popup, image_selectable_value, Icons},
        viewport::{ViewportImage, ViewportInfo},
    },
    util::ToEguiRect,
    viewer::{
        camera::{CameraMode, CameraModeChanged},
        edit::{
            link_select_mode::LinkSelectMode,
            select::{SelectBox, SelectPainter},
            transform_gizmo::{show_transform_gizmo, TransformGizmoState},
            EditorMode,
        },
        kmp::components::{RespawnPoint, RoutePoint},
    },
};
use bevy::{
    ecs::system::SystemState, math::vec2, prelude::*, render::render_resource::Extent3d, window::RequestRedraw,
};
use bevy_egui::egui::{
    self, Color32, CornerRadius, Margin, PopupAnchor, Response, Sense, Stroke, StrokeKind, Tooltip, Ui, UiBuilder,
};
use transform_gizmo::{config::TransformPivotPoint, GizmoOrientation};

pub fn show_viewport_tab(ui: &mut Ui, world: &mut World) {
    let Ok(window) = world.query::<&Window>().single(world) else {
        return;
    };

    let window_sf = window.scale_factor();

    let mut ss = SystemState::<(
        Res<ViewportImage>,
        ResMut<Assets<Image>>,
        ResMut<ViewportInfo>,
        MessageWriter<RequestRedraw>,
    )>::new(world);
    let Ok((viewport, mut image_assets, mut viewport_info, mut redraw)) = ss.get_mut(world) else {
        return;
    };

    let viewport_top_left = vec2(ui.next_widget_position().x, ui.next_widget_position().y);
    // Cap the texture dimensions because unexpectedly large images can exhaust
    // integrated GPUs, especially on high-DPI displays.
    let viewport_size = (vec2(ui.max_rect().max.x, ui.max_rect().max.y) - viewport_top_left)
        .max(Vec2::ONE)
        .min(Vec2::splat(2000.));
    let viewport_rect = Rect::from_corners(viewport_top_left, viewport_top_left + viewport_size);
    let egui_viewport_rect = viewport_rect.to_egui_rect();

    const MAX_TEXTURE_DIMENSION: u32 = 4096;
    let mut physical_size = (viewport_size * window_sf).round().as_uvec2().max(UVec2::ONE);
    let largest_dimension = physical_size.max_element();
    if largest_dimension > MAX_TEXTURE_DIMENSION {
        let scale = MAX_TEXTURE_DIMENSION as f32 / largest_dimension as f32;
        physical_size = (physical_size.as_vec2() * scale).round().as_uvec2().max(UVec2::ONE);
    }

    // Accessing an asset mutably marks it as modified, which causes Bevy to
    // reprepare the GPU image. Only do that when the texture actually changed size.
    let Some(current_size) = image_assets.get(viewport.handle.id()).map(Image::size) else {
        return;
    };
    if current_size != physical_size {
        let size = Extent3d {
            width: physical_size.x,
            height: physical_size.y,
            ..default()
        };
        if let Some(mut viewport_image) = image_assets.get_mut(viewport.handle.id()) {
            viewport_image.resize(size);
            // The resized target is rendered later in this frame. Request one more
            // frame so egui can display those contents in reactive desktop mode.
            redraw.write(RequestRedraw);
        }
    }

    // show the viewport image
    ui.scope_builder(UiBuilder::new().max_rect(egui_viewport_rect), |ui| {
        // Register the viewport background first so it consumes otherwise-unused
        // clicks and drags. The gizmo and overlay controls are registered later
        // and therefore remain the topmost interactions at the pointer.
        ui.add(
            egui::Image::new(egui::load::SizedTexture::new(
                viewport.tex_id,
                viewport_rect.size().to_array(),
            ))
            .sense(Sense::click_and_drag()),
        );
    });

    viewport_info.mouse_in_viewport = ui.rect_contains_pointer(egui_viewport_rect);
    viewport_info.viewport_rect = viewport_rect;

    ui.scope_builder(UiBuilder::new().max_rect(egui_viewport_rect), |ui| {
        ui.set_clip_rect(egui_viewport_rect);
        show_transform_gizmo(ui, egui_viewport_rect, world);
    });

    show_selection_overlay(ui, world);

    let responses = show_overlayed_ui(ui, world);

    world.resource_mut::<ViewportInfo>().mouse_on_overlayed_ui = responses.iter().any(|x| x.contains_pointer());

    // show the route hover label if needed
    if world.contains_resource::<LinkSelectMode<RoutePoint>>() {
        Tooltip::always_open(ui.ctx().clone(), ui.layer_id(), ui.next_auto_id(), PopupAnchor::Pointer).show(|ui| {
            ui.label("Select a Route (ESC to cancel)");
        });
    } else if world.contains_resource::<LinkSelectMode<RespawnPoint>>() {
        Tooltip::always_open(ui.ctx().clone(), ui.layer_id(), ui.next_auto_id(), PopupAnchor::Pointer).show(|ui| {
            ui.label("Select a Respawn (ESC to cancel)");
        });
    }
}

fn show_selection_overlay(ui: &mut Ui, world: &mut World) {
    let vp_rect = world.resource::<ViewportInfo>().viewport_rect.to_egui_rect();
    ui.scope_builder(UiBuilder::new().max_rect(vp_rect), |ui| {
        ui.set_clip_rect(vp_rect);
        let painter = ui.painter();
        if let Some(select_box) = world.resource::<SelectBox>().0 {
            let select_box = select_box.to_egui_rect();
            painter.rect(
                select_box,
                CornerRadius::from(2.),
                Color32::from_rgba_unmultiplied(200, 200, 200, 15),
                Stroke {
                    width: 1.,
                    color: Color32::GRAY,
                },
                StrokeKind::Inside,
            );
        }

        if *world.resource::<EditorMode>() == EditorMode::SelectPainter {
            let radius = world
                .resource::<SelectPainter>()
                .radius
                .clamp(SelectPainter::MIN_RADIUS, SelectPainter::MAX_RADIUS);
            if let Some(pointer_pos) = ui
                .input(|input| input.pointer.hover_pos())
                .filter(|pos| vp_rect.contains(*pos))
            {
                painter.circle(
                    pointer_pos,
                    radius,
                    Color32::from_rgba_unmultiplied(200, 200, 200, 12),
                    Stroke::new(1.5, Color32::LIGHT_GRAY),
                );
            }
        }
    });
}

fn show_overlayed_ui(ui: &mut Ui, world: &mut World) -> Vec<Response> {
    let vp_rect = world.resource::<ViewportInfo>().viewport_rect.to_egui_rect();

    let mut responses = Vec::new();
    // viewport overlayed ui
    ui.scope_builder(UiBuilder::new().max_rect(vp_rect), |ui| {
        ui.style_mut().spacing.item_spacing = egui::Vec2::splat(5.);

        egui::Frame::new().inner_margin(Margin::same(5)).show(ui, |ui| {
            // popups for things such as gizmo options, camera options, etc
            ui.horizontal(|ui| {
                let gizmo_options_btn = ui.button("Gizmo Options");
                responses.push(gizmo_options_btn.clone());
                let r = button_triggered_popup(ui, "gizmo_options_popup", gizmo_options_btn, |ui| {
                    ui.style_mut().spacing.button_padding = egui::Vec2::ZERO;
                    let size = 25.;
                    ui.label("Pivot:");
                    let mut transform_gizmo = world.resource_mut::<TransformGizmoState>();
                    ui.horizontal(|ui| {
                        let pivot = &mut transform_gizmo.config.pivot_point;
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
                        let orientation = &mut transform_gizmo.config.orientation;
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
                        ui.checkbox(&mut transform_gizmo.group_targets, "Group targets")
                            .on_hover_text_at_pointer(
                                "Use a single gizmo for all targets, rather than individual gizmos",
                            )
                    });
                });
                if let Some(r) = r {
                    responses.push(r);
                }

                let painter_btn = ui.button("Painter Options");
                responses.push(painter_btn.clone());
                let r = button_triggered_popup(ui, "painter_options_popup", painter_btn, |ui| {
                    let mut painter = world.resource_mut::<SelectPainter>();
                    ui.add(
                        egui::Slider::new(
                            &mut painter.radius,
                            SelectPainter::MIN_RADIUS..=SelectPainter::MAX_RADIUS,
                        )
                        .text("Painter Radius")
                        .suffix(" px"),
                    );
                });
                if let Some(r) = r {
                    responses.push(r);
                }

                let camera_mode = &mut world.resource_mut::<AppSettings>().camera.mode;
                let mut ev_camera_mode_change = None;
                let camera_btn = ui.button(format!("Camera: {}", camera_mode));
                responses.push(camera_btn.clone());
                let r = button_triggered_popup(ui, "camera_button_popup", camera_btn, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Camera Mode:");
                        if ui.selectable_value(camera_mode, CameraMode::Fly, "Fly").clicked() {
                            ev_camera_mode_change = Some(CameraModeChanged(CameraMode::Fly));
                        }
                        if ui.selectable_value(camera_mode, CameraMode::Orbit, "Orbit").clicked() {
                            ev_camera_mode_change = Some(CameraModeChanged(CameraMode::Orbit));
                        }
                        if ui
                            .selectable_value(camera_mode, CameraMode::TopDown, "Top Down")
                            .clicked()
                        {
                            ev_camera_mode_change = Some(CameraModeChanged(CameraMode::TopDown));
                        }
                    });
                });
                if let Some(ev_camera_mode_change) = ev_camera_mode_change {
                    world.write_message(ev_camera_mode_change);
                }
                if let Some(r) = r {
                    responses.push(r);
                }
            });
            // Blender-style mutually exclusive editor tools.
            let vertical_res = ui
                .vertical(|ui| {
                    ui.style_mut().spacing.button_padding = egui::Vec2::ZERO;
                    let size = 35.;
                    let mode = &mut *world.resource_mut::<EditorMode>();
                    image_selectable_value(ui, mode, EditorMode::Default, Icons::select_box(ui.ctx(), size), size)
                        .on_hover_text_at_pointer("Select and drag points, or box-select from empty space");
                    image_selectable_value(
                        ui,
                        mode,
                        EditorMode::SelectPainter,
                        Icons::select_painter(ui.ctx(), size),
                        size,
                    )
                    .on_hover_text_at_pointer("Paint-select points within the brush radius");
                    image_selectable_value(ui, mode, EditorMode::Translate, Icons::translate(ui.ctx(), size), size)
                        .on_hover_text_at_pointer("Translate gizmo");
                    image_selectable_value(ui, mode, EditorMode::Rotate, Icons::rotate(ui.ctx(), size), size)
                        .on_hover_text_at_pointer("Rotate gizmo");
                    image_selectable_value(ui, mode, EditorMode::Scale, Icons::scale(ui.ctx(), size), size)
                        .on_hover_text_at_pointer("Scale point spacing with grouped multiple selection");
                    image_selectable_value(ui, mode, EditorMode::Transform, Icons::transform(ui.ctx(), size), size)
                        .on_hover_text_at_pointer("Combined translate, rotate, and scale gizmo");
                })
                .response;
            responses.push(vertical_res);
        });
    });
    responses
}
