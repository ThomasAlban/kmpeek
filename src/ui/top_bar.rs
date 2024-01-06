use bevy::{ecs::system::SystemParam, window::PrimaryWindow};
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use strum::IntoEnumIterator;
use crate::viewer::camera::{CameraMode, CameraModeChanged};
use super::app_state::AppModeChanged;
use super::{
    app_state::{AppMode, AppSettings, AppState},
    update_ui::UiSection,
};

#[derive(SystemParam)]
pub struct ShowTopBar<'w, 's> {
    contexts: EguiContexts<'w, 's>,
    settings: ResMut<'w, AppSettings>,
    app_state: ResMut<'w, AppState>,
    ev_camera_mode_changed: EventWriter<'w, CameraModeChanged>,
    window: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    ev_app_mode_changed: EventWriter<'w, AppModeChanged>,
}
impl UiSection for ShowTopBar<'_, '_> {
    fn show(&mut self) {
        let ctx = self.contexts.ctx_mut();
        let window = self.window.get_single().unwrap();

        egui::TopBottomPanel::top("Top Bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {

                    if self.app_state.show_modes_collapsed.is_some() {
                        ui.horizontal(|ui| {
                            ui.label("Mode");
                            egui::ComboBox::from_id_source("Mode")
                            .selected_text(self.app_state.mode.to_string())
                            .width(160.)
                            .show_ui(ui, |ui| {
                                for mode in AppMode::iter() {
                                    if ui.selectable_value(&mut self.app_state.mode, mode, mode.to_string()).clicked() {
                                        self.ev_app_mode_changed.send_default();
                                    }
                                }  
                            });
                        });                  
                    } else {
                        ui.horizontal(|ui| {
                            for mode in AppMode::iter() {
                                if ui.selectable_value(&mut self.app_state.mode, mode, mode.to_string()).clicked() {
                                    self.ev_app_mode_changed.send_default();
                                };
                            }
                        });
                    }
                });
                ui.with_layout(egui::Layout::top_down(egui::Align::RIGHT), |ui| {
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut self.settings.increment).speed(1));
                        ui.label("Increment:").on_hover_text(
                            "How much the increment and decrement buttons should change the value by",
                        );
                        ui.separator();
                        if ui.selectable_value(&mut self.settings.camera.mode, CameraMode::TopDown, "Top Down").clicked() {
                            self.ev_camera_mode_changed.send(CameraModeChanged(CameraMode::TopDown));
                        }
                        if ui.selectable_value(&mut self.settings.camera.mode, CameraMode::Orbit, "Orbit").clicked() {
                            self.ev_camera_mode_changed.send(CameraModeChanged(CameraMode::Orbit));
                        }
                        if ui.selectable_value(&mut self.settings.camera.mode, CameraMode::Fly, "Fly").clicked() {
                            self.ev_camera_mode_changed.send(CameraModeChanged(CameraMode::Fly));
                        }
                        ui.label("Camera Mode:");
                        
                        // if we are overflowing
                        if ui.available_width() == 0. {
                            self.app_state.show_modes_collapsed = Some(window.width());
                        }
                        if let Some(window_width) = self.app_state.show_modes_collapsed {
                            if window.width() > window_width {
                                self.app_state.show_modes_collapsed = None;
                            }
                        }

                    });
                });
            });
        });
    }
}
