use crate::{
    ui::{
        settings::AppSettings,
        util::{button_triggered_popup, image_selectable_value, Icons},
        viewport::{ViewportImage, ViewportInfo},
    },
    util::ToEguiRect,
    viewer::{
        camera::{CameraMode, CameraModeChanged},
        edit::{link_select_mode::LinkSelectMode, select::SelectBox, EditMode},
        kmp::components::{RespawnPoint, RoutePoint},
    },
};
use bevy::{
    ecs::system::SystemState,
    math::vec2,
    prelude::*,
    render::render_resource::Extent3d,
    window::RequestRedraw,
};
use bevy_egui::egui::{
    self, Color32, CornerRadius, Margin, PopupAnchor, Response, Sense, Stroke, StrokeKind, Tooltip, Ui, UiBuilder,
};
use transform_gizmo_bevy::{config::TransformPivotPoint, GizmoOptions, GizmoOrientation};

pub fn show_viewport_tab(ui: &mut Ui, world: &mut World) {
    let window = world.query::<&Window>().single(world).unwrap();

    let window_sf = window.scale_factor();

    let mut ss = SystemState::<(
        Res<ViewportImage>,
        ResMut<Assets<Image>>,
        ResMut<ViewportInfo>,
        MessageWriter<RequestRedraw>,
    )>::new(world);
    let (viewport, mut image_assets, mut viewport_info, mut redraw) = ss.get_mut(world);

    let viewport_top_left = vec2(ui.next_widget_position().x, ui.next_widget_position().y);
    // Cap the texture dimensions because unexpectedly large images can exhaust
    // integrated GPUs, especially on high-DPI displays.
    let viewport_size = (vec2(ui.max_rect().max.x, ui.max_rect().max.y) - viewport_top_left)
        .max(Vec2::ONE)
        .min(Vec2::splat(2000.));
    let viewport_rect = Rect::from_corners(viewport_top_left, viewport_top_left + viewport_size);
    let egui_viewport_rect = viewport_rect.to_egui_rect();

    let physical_size = (viewport_size * window_sf).round().as_uvec2().max(UVec2::ONE);

    // Accessing an asset mutably marks it as modified, which causes Bevy to
    // reprepare the GPU image. Only do that when the texture actually changed size.
    let current_size = image_assets.get(viewport.handle.id()).unwrap().size();
    if current_size != physical_size {
        let size = Extent3d {
            width: physical_size.x,
            height: physical_size.y,
            ..default()
        };
        image_assets.get_mut(viewport.handle.id()).unwrap().resize(size);
        // The resized target is rendered later in this frame. Request one more
        // frame so egui can display those contents in reactive desktop mode.
        redraw.write(RequestRedraw);
    }

    // show the viewport image
    ui.scope_builder(UiBuilder::new().max_rect(egui_viewport_rect), |ui| {
        // make the image sense clicks and drags, so that any events that aren't consumed by buttons above it are consumed by this
        // so we don't start dragging around the window when trying to select stuff etc
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

    show_select_box(ui, world);

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

fn show_select_box(ui: &mut Ui, world: &mut World) {
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
    });
}

fn show_overlayed_ui(ui: &mut Ui, world: &mut World) -> Vec<Response> {
    let vp_rect = world.resource::<ViewportInfo>().viewport_rect.to_egui_rect();
    // let ss = SystemState::<(
    //     Res<ViewportInfo>,
    //     ResMut<GizmoOptions>,
    //     ResMut<AppSettings>,
    //     ResMut<EditMode>,
    // )>::new(world);
    // let (vp, mut gizmo_options, mut settings, mut edit_mode) = ss.get_mut(world);

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
                    let mut gizmo_options = world.resource_mut::<GizmoOptions>();
                    ui.horizontal(|ui| {
                        let pivot = &mut gizmo_options.pivot_point;
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
                        let orientation = &mut gizmo_options.gizmo_orientation;
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
                        ui.checkbox(&mut gizmo_options.group_targets, "Group targets")
                            .on_hover_text_at_pointer(
                                "Use a single gizmo for all targets, rather than individual gizmos",
                            )
                    });
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
            // cursor/gizmo mode
            let vertical_res = ui
                .vertical(|ui| {
                    ui.style_mut().spacing.button_padding = egui::Vec2::ZERO;
                    let mode = &mut *world.resource_mut::<EditMode>();
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
