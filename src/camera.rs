use bevy::{
    input::mouse::{MouseMotion, MouseWheel},
    math::vec3,
    prelude::*,
    window::{CursorGrabMode, PrimaryWindow},
};
use bevy_infinite_grid::{GridShadowCamera, InfiniteGrid, InfiniteGridBundle, InfiniteGridPlugin};

pub struct CameraPlugin;
impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraSettings>()
            .add_startup_system(camera_setup)
            .add_plugin(InfiniteGridPlugin)
            .add_system(cursor_grab)
            .add_system(update_active_camera)
            .add_system(fly_cam_look)
            .add_system(fly_cam_move)
            .add_system(orbit_cam)
            .add_system(top_down_cam);
    }
}

#[derive(Component)]
pub struct FlyCam;

#[derive(Component)]
pub struct OrbitCam {
    pub focus: Vec3,
    pub radius: f32,
    pub upside_down: bool,
}
impl Default for OrbitCam {
    fn default() -> Self {
        Self {
            focus: Vec3::ZERO,
            radius: 10000.,
            upside_down: false,
        }
    }
}

#[derive(Component)]
pub struct TopDownCam;

#[derive(PartialEq, Clone, Copy)]
pub enum CameraMode {
    Fly,
    Orbit,
    TopDown,
}
impl Default for CameraMode {
    fn default() -> Self {
        Self::Fly
    }
}

#[derive(Resource, Default)]
pub struct CameraSettings {
    pub mode: CameraMode,
    pub fly: FlySettings,
    pub orbit: OrbitSettings,
    pub top_down: TopDownSettings,
}

pub struct FlySettings {
    pub start_pos: Vec3,
    pub look_sensitivity: f32,
    pub hold_mouse_to_move: bool,
    pub speed: f32,
    pub speed_boost: f32,
    pub key_bindings: FlyKeyBindings,
}
impl Default for FlySettings {
    fn default() -> Self {
        Self {
            start_pos: vec3(50000., 50000., 0.),
            look_sensitivity: 1.,
            hold_mouse_to_move: false,
            speed: 1.,
            speed_boost: 3.,
            key_bindings: FlyKeyBindings::default(),
        }
    }
}
pub struct FlyKeyBindings {
    pub move_forward: Vec<KeyCode>,
    pub move_backward: Vec<KeyCode>,
    pub move_left: Vec<KeyCode>,
    pub move_right: Vec<KeyCode>,
    pub move_ascend: Vec<KeyCode>,
    pub move_descend: Vec<KeyCode>,
    pub speed_boost: Vec<KeyCode>,
    pub mouse_button: MouseButton,
}
impl Default for FlyKeyBindings {
    fn default() -> Self {
        Self {
            move_forward: vec![KeyCode::W, KeyCode::Up],
            move_backward: vec![KeyCode::S, KeyCode::Down],
            move_left: vec![KeyCode::A, KeyCode::Left],
            move_right: vec![KeyCode::D, KeyCode::Right],
            move_ascend: vec![KeyCode::E, KeyCode::PageUp],
            move_descend: vec![KeyCode::Q, KeyCode::PageDown],
            speed_boost: vec![KeyCode::LShift, KeyCode::RShift],
            mouse_button: MouseButton::Right,
        }
    }
}

pub struct OrbitSettings {
    pub start_pos: Vec3,
    pub rotate_sensitivity: f32,
    pub pan_sensitivity: f32,
    pub scroll_sensitivity: f32,
    pub key_bindings: OrbitKeyBindings,
}
impl Default for OrbitSettings {
    fn default() -> Self {
        Self {
            start_pos: vec3(50000., 50000., 0.),
            rotate_sensitivity: 1.,
            pan_sensitivity: 1.,
            scroll_sensitivity: 1.,
            key_bindings: OrbitKeyBindings::default(),
        }
    }
}

pub struct OrbitKeyBindings {
    pub mouse_button: MouseButton,
    pub pan: Vec<KeyCode>,
}
impl Default for OrbitKeyBindings {
    fn default() -> Self {
        Self {
            mouse_button: MouseButton::Right,
            pan: vec![KeyCode::LShift, KeyCode::RShift],
        }
    }
}

pub struct TopDownSettings {
    pub start_pos: Vec3,
    pub near: f32,
    pub far: f32,
    pub scale: f32,
    pub move_sensitivity: f32,
    pub scroll_sensitivity: f32,
    pub key_bindings: TopDownKeyBindings,
}
impl Default for TopDownSettings {
    fn default() -> Self {
        Self {
            start_pos: vec3(0., 10000., 0.),
            near: 0.00001,
            far: 100000.,
            scale: 100.,
            move_sensitivity: 1.,
            scroll_sensitivity: 1.,
            key_bindings: TopDownKeyBindings::default(),
        }
    }
}
pub struct TopDownKeyBindings {
    pub mouse_button: MouseButton,
}
impl Default for TopDownKeyBindings {
    fn default() -> Self {
        Self {
            mouse_button: MouseButton::Right,
        }
    }
}

pub fn camera_setup(mut commands: Commands) {
    commands.spawn(InfiniteGridBundle {
        transform: Transform::from_scale(Vec3::ONE * 0.001),
        grid: InfiniteGrid {
            fadeout_distance: 400000.,
            ..default()
        },
        ..default()
    });
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.5,
    });

    let fly_default = FlySettings::default();
    let orbit_default = OrbitSettings::default();
    let topdown_default = TopDownSettings::default();

    let inactive = Camera {
        is_active: false,
        ..default()
    };
    commands
        .spawn((
            Camera3dBundle {
                transform: Transform::from_translation(fly_default.start_pos)
                    .looking_at(Vec3::ZERO, Vec3::Y),
                ..default()
            },
            FlyCam,
        ))
        .insert(GridShadowCamera);
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_translation(orbit_default.start_pos)
                .looking_at(Vec3::ZERO, Vec3::Y),
            camera: inactive.clone(),
            ..default()
        },
        OrbitCam {
            radius: OrbitSettings::default().start_pos.length(),
            ..default()
        },
    ));
    commands.spawn((
        Camera3dBundle {
            projection: Projection::Orthographic(OrthographicProjection {
                near: topdown_default.near,
                far: topdown_default.far,
                scale: topdown_default.scale,
                ..default()
            }),
            camera: inactive,
            transform: Transform::from_translation(topdown_default.start_pos)
                .looking_at(Vec3::ZERO, Vec3::Z),
            ..default()
        },
        TopDownCam,
    ));
}

pub fn cursor_grab(
    mouse_buttons: Res<Input<MouseButton>>,
    settings: Res<CameraSettings>,
    mut window: Query<&mut Window, With<PrimaryWindow>>,
) {
    let mut window = window
        .get_single_mut()
        .expect("Primary window not found for cursor grab");

    if (settings.mode == CameraMode::Fly
        && !mouse_buttons.pressed(settings.fly.key_bindings.mouse_button))
        || (settings.mode == CameraMode::Orbit
            && !mouse_buttons.pressed(settings.orbit.key_bindings.mouse_button))
        || (settings.mode == CameraMode::TopDown
            && !mouse_buttons.pressed(settings.top_down.key_bindings.mouse_button))
    {
        window.cursor.visible = true;
        window.cursor.grab_mode = CursorGrabMode::None;
        return;
    }
    // hide the cursor and lock its position
    window.cursor.visible = false;
    window.cursor.grab_mode = CursorGrabMode::Locked;
}

#[allow(clippy::type_complexity)]
pub fn update_active_camera(
    settings: Res<CameraSettings>,
    mut fly_cam: Query<&mut Camera, (With<FlyCam>, Without<OrbitCam>, Without<TopDownCam>)>,
    mut orbit_cam: Query<&mut Camera, (With<OrbitCam>, Without<FlyCam>, Without<TopDownCam>)>,
    mut topdown_cam: Query<&mut Camera, (With<TopDownCam>, Without<FlyCam>, Without<OrbitCam>)>,
) {
    if !settings.is_changed() {
        return;
    }
    let mut fly_cam = fly_cam.get_single_mut().unwrap();
    let mut orbit_cam = orbit_cam.get_single_mut().unwrap();
    let mut topdown_cam = topdown_cam.get_single_mut().unwrap();
    match settings.mode {
        CameraMode::Fly => {
            fly_cam.is_active = true;
            orbit_cam.is_active = false;
            topdown_cam.is_active = false;
        }
        CameraMode::Orbit => {
            fly_cam.is_active = false;
            orbit_cam.is_active = true;
            topdown_cam.is_active = false;
        }
        CameraMode::TopDown => {
            fly_cam.is_active = false;
            orbit_cam.is_active = false;
            topdown_cam.is_active = true;
        }
    }
}

pub fn fly_cam_move(
    keys: Res<Input<KeyCode>>,
    time: Res<Time>,
    window: Query<&Window, With<PrimaryWindow>>,
    settings: Res<CameraSettings>,
    mut fly_cam: Query<&mut Transform, With<FlyCam>>,
) {
    let window = window
        .get_single()
        .expect("Primary window not found for fly cam move");
    let mut fly_cam_transform = fly_cam
        .get_single_mut()
        .expect("Could not get single fly cam");

    let mut velocity = Vec3::ZERO;
    let local_z = fly_cam_transform.local_z();
    let forward = -Vec3::new(local_z.x, 0., local_z.z);
    let right = Vec3::new(local_z.z, 0., -local_z.x);

    if settings.fly.hold_mouse_to_move && window.cursor.grab_mode == CursorGrabMode::None {
        return;
    }
    let mut speed_boost = false;
    for key in keys.get_pressed() {
        let key_bindings = &settings.fly.key_bindings;
        if key_bindings.move_forward.contains(key) {
            velocity += forward;
        } else if key_bindings.move_backward.contains(key) {
            velocity -= forward;
        } else if key_bindings.move_left.contains(key) {
            velocity -= right;
        } else if key_bindings.move_right.contains(key) {
            velocity += right;
        } else if key_bindings.move_ascend.contains(key) {
            velocity += Vec3::Y;
        } else if key_bindings.move_descend.contains(key) {
            velocity -= Vec3::Y;
        } else if key_bindings.speed_boost.contains(key) {
            speed_boost = true;
        }
    }
    if speed_boost {
        velocity *= settings.fly.speed_boost;
    }
    fly_cam_transform.translation += velocity * time.delta_seconds() * 10000. * settings.fly.speed;
}

pub fn fly_cam_look(
    settings: Res<CameraSettings>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut mouse_motion: EventReader<MouseMotion>,
    mut fly_cam: Query<&mut Transform, With<FlyCam>>,
) {
    let window = window
        .get_single()
        .expect("Primary window not found for fly cam move");
    let mut fly_cam_transform = fly_cam
        .get_single_mut()
        .expect("Could not get single fly cam");

    for ev in mouse_motion.iter() {
        let (mut yaw, mut pitch, _) = fly_cam_transform.rotation.to_euler(EulerRot::YXZ);
        match window.cursor.grab_mode {
            CursorGrabMode::None => (),
            _ => {
                // Using smallest of height or width ensures equal vertical and horizontal sensitivity
                let window_scale = window.height().min(window.width());
                pitch -= (settings.fly.look_sensitivity * 0.00012 * ev.delta.y * window_scale)
                    .to_radians();
                yaw -= (settings.fly.look_sensitivity * 0.00012 * ev.delta.x * window_scale)
                    .to_radians();
            }
        }

        pitch = pitch.clamp(-1.54, 1.54);
        // order is important to prevent unintended roll
        fly_cam_transform.rotation =
            Quat::from_axis_angle(Vec3::Y, yaw) * Quat::from_axis_angle(Vec3::X, pitch);
    }
}

pub fn orbit_cam(
    primary_window: Query<&mut Window, With<PrimaryWindow>>,
    mut mouse_motion: EventReader<MouseMotion>,
    mut mouse_scroll: EventReader<MouseWheel>,
    mouse_buttons: Res<Input<MouseButton>>,
    mut query: Query<(&mut OrbitCam, &mut Transform, &Projection)>,
    settings: Res<CameraSettings>,
    keys: Res<Input<KeyCode>>,
) {
    if settings.mode != CameraMode::Orbit {
        return;
    }

    let window = primary_window.get_single();
    if window.is_err() {
        warn!("Primary window not found for camera controller");
        return;
    }
    let window = window.unwrap();

    let mut pan = Vec2::ZERO;
    let mut rotation_move = Vec2::ZERO;
    let mut scroll = 0.0;
    let mut orbit_button_changed = false;

    if mouse_buttons.pressed(settings.orbit.key_bindings.mouse_button) {
        // check if the pan key is being pressed, and if so, pan rather than orbit
        let mut rotate = true;
        for key in keys.get_pressed() {
            if settings.orbit.key_bindings.pan.contains(key) {
                rotate = false;
                break;
            }
        }
        if rotate {
            for ev in mouse_motion.iter() {
                rotation_move += ev.delta * settings.orbit.rotate_sensitivity;
            }
        } else {
            for ev in mouse_motion.iter() {
                pan += ev.delta * settings.orbit.pan_sensitivity;
            }
        }
    }

    for ev in mouse_scroll.iter() {
        scroll += ev.y;
    }

    if mouse_buttons.just_released(settings.orbit.key_bindings.mouse_button)
        || mouse_buttons.just_pressed(settings.orbit.key_bindings.mouse_button)
    {
        orbit_button_changed = true;
    }

    for (mut orbit_cam, mut transform, projection) in query.iter_mut() {
        if orbit_button_changed {
            // only check for upside down when orbiting started or ended this frame
            // if the camera is "upside" down, panning horizontally would be inverted, so invert the input to make it correct
            let up = transform.rotation * Vec3::Y;
            orbit_cam.upside_down = up.y <= 0.0;
        }

        let window_size = Vec2::new(window.width(), window.height());

        let mut any = false;
        if rotation_move.length_squared() > 0.0 {
            any = true;
            let delta_x = {
                let delta = rotation_move.x / window_size.x * std::f32::consts::PI * 2.0;
                if orbit_cam.upside_down {
                    -delta
                } else {
                    delta
                }
            };
            let delta_y = rotation_move.y / window_size.y * std::f32::consts::PI;

            let yaw = Quat::from_rotation_y(-delta_x);
            let pitch = Quat::from_rotation_x(-delta_y);
            transform.rotation = yaw * transform.rotation; // rotate around global y axis
            transform.rotation *= pitch; // rotate around local x axis
        } else if pan.length_squared() > 0.0 {
            any = true;
            // make panning distance independent of resolution and FOV
            if let Projection::Perspective(projection) = projection {
                pan *= Vec2::new(projection.fov * projection.aspect_ratio, projection.fov)
                    / window_size;
            }
            // translate by local axes
            let right = transform.rotation * Vec3::X * -pan.x;
            let up = transform.rotation * Vec3::Y * pan.y;
            // make panning proportional to distance away from focus point
            let translation = (right + up) * orbit_cam.radius;
            orbit_cam.focus += translation;
        } else if scroll.abs() > 0.0 {
            any = true;
            orbit_cam.radius -=
                scroll * orbit_cam.radius * 0.002 * settings.orbit.scroll_sensitivity;
            // dont allow zoom to reach zero or you get stuck
            orbit_cam.radius = orbit_cam.radius.clamp(1., 1000000.);
        }

        if any {
            // emulating parent/child to make the yaw/y-axis rotation behave like a turntable
            // parent = x and y rotation
            // child = z-offset
            let rot_matrix = Mat3::from_quat(transform.rotation);
            transform.translation =
                orbit_cam.focus + rot_matrix.mul_vec3(Vec3::new(0.0, 0.0, orbit_cam.radius));
        }
    }
    // consume any remaining events, so they don't pile up if we don't need them
    // (and also to avoid Bevy warning us about not checking events every frame update)
    mouse_motion.clear();
}

pub fn top_down_cam(
    primary_window: Query<&mut Window, With<PrimaryWindow>>,
    mut mouse_motion: EventReader<MouseMotion>,
    mut mouse_scroll: EventReader<MouseWheel>,
    mouse_buttons: Res<Input<MouseButton>>,
    mut query: Query<(&TopDownCam, &mut Transform, &mut Projection)>,
    settings: Res<CameraSettings>,
) {
    if settings.mode != CameraMode::TopDown {
        return;
    }

    let window = primary_window.get_single();
    if window.is_err() {
        warn!("Primary window not found for camera controller");
        return;
    }
    let window = window.unwrap();

    let mut pan = Vec2::ZERO;
    let mut scroll = 0.;

    if mouse_buttons.pressed(settings.orbit.key_bindings.mouse_button) {
        for ev in mouse_motion.iter() {
            pan += ev.delta;
        }
    }
    for ev in mouse_scroll.iter() {
        scroll += ev.y;
    }

    let window_size = Vec2::new(window.width(), window.height());

    for (_, mut transform, mut projection) in query.iter_mut() {
        if let Projection::Orthographic(projection) = &*projection {
            pan *= Vec2::new(projection.area.width(), projection.area.height()) / window_size;
        }
        transform.translation += vec3(pan.x, 0., pan.y) * settings.top_down.move_sensitivity;

        if scroll.abs() > 0. {
            if let Projection::Orthographic(projection) = &mut *projection {
                projection.scale -=
                    (scroll * projection.scale) * 0.001 * settings.top_down.scroll_sensitivity;
                projection.scale = projection.scale.clamp(1., 1000.);
            }
        }
    }
}
