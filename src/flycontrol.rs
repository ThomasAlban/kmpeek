use three_d::*;

pub struct FlyControl {
    speed: f32,
    right_mouse_down: bool,
    w_down: bool,
    a_down: bool,
    s_down: bool,
    d_down: bool,
    q_down: bool,
    e_down: bool,
    speed_modifier: f32,
}

impl FlyControl {
    pub fn new(speed: f32) -> Self {
        Self {
            speed,
            right_mouse_down: false,
            w_down: false,
            a_down: false,
            s_down: false,
            d_down: false,
            q_down: false,
            e_down: false,
            speed_modifier: 5.,
        }
    }

    /// Handles the events. Must be called each frame.
    pub fn handle_events<T>(&mut self, camera: &mut Camera, events: &mut [Event<T>]) -> bool
    where
        T: 'static + Clone,
    {
        let mut change = false;
        for event in events.iter_mut() {
            match event {
                Event::MouseMotion {
                    delta,
                    button,
                    handled,
                    ..
                } => {
                    if !*handled && button.is_some() {
                        if let Some(b) = button {
                            if let MouseButton::Right = b {
                                // horizontal
                                *handled = self.handle_action(
                                    camera,
                                    CameraAction::Yaw {
                                        speed: std::f32::consts::PI / 1800.0,
                                    },
                                    -delta.0 as f32,
                                );
                                // vertical
                                *handled |= self.handle_action(
                                    camera,
                                    CameraAction::Pitch {
                                        speed: std::f32::consts::PI / 1800.0,
                                    },
                                    -delta.1 as f32,
                                );
                                change |= *handled;
                            }
                        }
                    }
                }

                Event::MousePress {
                    button,
                    position: _,
                    modifiers: _,
                    handled: _,
                } => {
                    if let MouseButton::Right = button {
                        self.right_mouse_down = true;
                    }
                }
                Event::MouseRelease {
                    button,
                    position: _,
                    modifiers: _,
                    handled: _,
                } => {
                    if let MouseButton::Right = button {
                        self.right_mouse_down = false;
                    }
                }
                Event::KeyPress {
                    kind,
                    modifiers: _,
                    handled: _,
                } => match kind {
                    Key::W => self.w_down = true,
                    Key::A => self.a_down = true,
                    Key::S => self.s_down = true,
                    Key::D => self.d_down = true,
                    Key::Q => self.q_down = true,
                    Key::E => self.e_down = true,
                    Key::Space => self.speed_modifier = 25.,
                    _ => {}
                },
                Event::KeyRelease {
                    kind,
                    modifiers: _,
                    handled: _,
                } => match kind {
                    Key::W => self.w_down = false,
                    Key::A => self.a_down = false,
                    Key::S => self.s_down = false,
                    Key::D => self.d_down = false,
                    Key::Q => self.q_down = false,
                    Key::E => self.e_down = false,
                    Key::Space => self.speed_modifier = 5.,
                    _ => {}
                },
                _ => {}
            }
        }
        if self.w_down {
            self.handle_action(
                camera,
                CameraAction::Forward { speed: self.speed },
                self.speed_modifier,
            );
        }
        if self.a_down {
            self.handle_action(
                camera,
                CameraAction::Left { speed: self.speed },
                self.speed_modifier,
            );
        }
        if self.s_down {
            self.handle_action(
                camera,
                CameraAction::Forward { speed: -self.speed },
                self.speed_modifier,
            );
        }
        if self.d_down {
            self.handle_action(
                camera,
                CameraAction::Left { speed: -self.speed },
                self.speed_modifier,
            );
        }
        if self.q_down {
            self.handle_action(
                camera,
                CameraAction::Up { speed: -self.speed },
                self.speed_modifier,
            );
        }
        if self.e_down {
            self.handle_action(
                camera,
                CameraAction::Up { speed: self.speed },
                self.speed_modifier,
            );
        }
        change
    }

    fn handle_action(&mut self, camera: &mut Camera, control_type: CameraAction, x: f32) -> bool {
        match control_type {
            CameraAction::Pitch { speed } => {
                camera.pitch(radians(speed * x));
            }
            CameraAction::OrbitUp { speed, target } => {
                camera.rotate_around_with_fixed_up(&target, 0.0, speed * x);
            }
            CameraAction::Yaw { speed } => {
                camera.yaw(radians(speed * x));
            }
            CameraAction::OrbitLeft { speed, target } => {
                camera.rotate_around_with_fixed_up(&target, speed * x, 0.0);
            }
            CameraAction::Roll { speed } => {
                camera.roll(radians(speed * x));
            }
            CameraAction::Left { speed } => {
                let change = -camera.right_direction() * x * speed;
                camera.translate(&change);
            }
            CameraAction::Up { speed } => {
                let change = vec3(0., 1., 0.) * x * speed;
                camera.translate(&change);
            }
            CameraAction::Forward { speed } => {
                let change = camera.view_direction() * speed * x;
                camera.translate(&change);
            }
            CameraAction::Zoom {
                target,
                speed,
                min,
                max,
            } => {
                camera.zoom_towards(&target, speed * x, min, max);
            }
            CameraAction::None => {}
        }
        control_type != CameraAction::None
    }
}
