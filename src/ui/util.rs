#![allow(dead_code)]
use bevy::{
    math::{vec3, EulerRot, Quat, Vec3},
    transform::components::Transform,
};
use bevy_egui::egui::{
    self, emath::Numeric, include_image, Align, Align2, Area, Button, Color32, Context, Image,
    ImageButton, ImageSource, Order, Response, Sense, Ui, Vec2,
};
use std::{
    fmt::Display,
    hash::Hash,
    ops::{AddAssign, SubAssign},
};

pub fn combobox_enum<T>(
    ui: &mut Ui,
    value: &mut T,
    id: impl std::hash::Hash,
    width: Option<f32>,
) -> Response
where
    T: strum::IntoEnumIterator + Display + PartialEq + Clone,
{
    let mut combobox = egui::ComboBox::from_id_source(id).selected_text(value.to_string());
    combobox = if let Some(width) = width {
        combobox.width(width)
    } else {
        combobox.width(ui.available_width())
    };
    combobox
        .show_ui(ui, |ui| {
            for variant in T::iter() {
                ui.selectable_value(value, variant.clone(), variant.to_string());
            }
        })
        .response
}

pub fn increment_buttons<Num>(ui: &mut Ui, value: &mut Num, increment: &Num)
where
    Num: Numeric + AddAssign + SubAssign,
{
    let width = 15.;
    if ui
        .add(Button::new("-").min_size(Vec2 { x: width, y: 0. }))
        .clicked()
    {
        *value -= *increment;
    }
    if ui
        .add(Button::new("+").min_size(Vec2 { x: width, y: 0. }))
        .clicked()
    {
        *value += *increment;
    }
}

pub fn svg_image<'a>(img: impl Into<ImageSource<'a>>, ctx: &Context, size: f32) -> Image<'a> {
    let img = egui::Image::new(img);
    // scale up the svg image by the window scale factor so it doesn't look blurry on lower resolution screens
    img.load_for_size(ctx, egui::Vec2::splat(size) * ctx.pixels_per_point())
        .unwrap();
    img
}

pub fn image_selectable_value<Value: PartialEq>(
    ui: &mut egui::Ui,
    current: &mut Value,
    selected: Value,
    img: Image,
    size: f32,
) -> Response {
    let res = ui.allocate_ui(egui::Vec2::splat(size), |ui| {
        let btn = ui.add(ImageButton::new(img).selected(*current == selected));
        if btn.clicked() {
            *current = selected;
        };
        btn
    });
    res.inner
}

pub fn drag_vec3(ui: &mut Ui, value: &mut Vec3, speed: f32) -> (Response, Response, Response) {
    ui.columns(3, |ui| {
        let x = ui[0]
            .centered_and_justified(|ui| {
                ui.add(
                    egui::DragValue::new(&mut value.x)
                        .speed(speed)
                        .fixed_decimals(1),
                )
            })
            .inner;
        let y = ui[1]
            .centered_and_justified(|ui| {
                ui.add(
                    egui::DragValue::new(&mut value.y)
                        .speed(speed)
                        .fixed_decimals(1),
                )
            })
            .inner;
        let z = ui[2]
            .centered_and_justified(|ui| {
                ui.add(
                    egui::DragValue::new(&mut value.z)
                        .speed(speed)
                        .fixed_decimals(1),
                )
            })
            .inner;
        (x, y, z)
    })
}

pub fn rotation_edit(ui: &mut egui::Ui, transform: &mut Transform, speed: f32) -> bool {
    // this was a fucking fuck to code

    let euler = transform.rotation.to_euler(EulerRot::XYZ);

    let mut rot = vec3(
        f32::to_degrees(euler.0),
        f32::to_degrees(euler.1),
        f32::to_degrees(euler.2),
    );

    let clamp_0_360 = |angle: &mut f32| {
        *angle %= 360.;
        if *angle < 0. {
            *angle += 360.;
        }
        if *angle == 360. {
            *angle = 0.;
        }
    };

    clamp_0_360(&mut rot.x);
    clamp_0_360(&mut rot.y);
    clamp_0_360(&mut rot.z);

    let (x, y, z) = drag_vec3(ui, &mut rot, speed);

    let changed = x.changed() || y.changed() || z.changed();

    let mut update_rotation = |res: Response, axis: Vec3| {
        if res.changed() {
            // get the drag delta
            let delta = res.drag_delta().x / 100.;
            if res.dragged() {
                // rotate around the specific axis if we are dragging it
                transform.rotate_local_axis(axis, delta);
            } else {
                // if we are setting the value by typing it in, set the quaternion directly
                // we can't always set the quaternion directly because it leads to problems involving
                // the fact that euler -> quat -> euler does not always give the same result
                transform.rotation = Quat::from_euler(
                    EulerRot::XYZ,
                    f32::to_radians(rot.x),
                    f32::to_radians(rot.y),
                    f32::to_radians(rot.z),
                );
            }
        }
    };
    update_rotation(x, Vec3::X);
    update_rotation(y, Vec3::Y);
    update_rotation(z, Vec3::Z);

    if changed {
        transform.rotation = transform.rotation.normalize();
    }

    changed
}

pub fn button_triggered_popup<R>(
    ui: &mut Ui,
    id: impl Hash,
    btn: Response,
    add_contents: impl FnOnce(&mut Ui) -> R,
) {
    let popup_id = ui.make_persistent_id(id);
    if btn.clicked() {
        ui.memory_mut(|mem| mem.toggle_popup(popup_id));
    }

    if ui.memory(|mem| mem.is_popup_open(popup_id)) {
        let (pos, pivot) = (btn.rect.left_bottom(), Align2::LEFT_TOP);

        let res = Area::new(popup_id)
            .order(Order::Foreground)
            .constrain(true)
            .fixed_pos(pos)
            .pivot(pivot)
            .show(ui.ctx(), |ui| {
                let frame = egui::Frame::popup(ui.style());
                let frame_margin = frame.total_margin();
                frame
                    .show(ui, |ui| {
                        ui.with_layout(egui::Layout::top_down_justified(Align::LEFT), |ui| {
                            ui.set_width(btn.rect.width() - frame_margin.sum().x);
                            add_contents(ui)
                        })
                        .inner
                    })
                    .inner
            });

        let clicked_elsewhere = res.response.clicked_elsewhere() && btn.clicked_elsewhere();

        if ui.input(|i| i.key_pressed(egui::Key::Escape)) || clicked_elsewhere {
            ui.memory_mut(|mem| mem.close_popup());
        }
    }
}

pub fn view_icon_btn(ui: &mut Ui, checked: &mut bool) -> Response {
    ui.style_mut().spacing.button_padding = Vec2::ZERO;

    let img = if *checked {
        Icons::view_on(ui.ctx())
    } else {
        Icons::view_off(ui.ctx())
    };

    let res = ui.allocate_ui(egui::Vec2::splat(Icons::SIZE), |ui| {
        let mut icon = ui.add(img.sense(Sense::click()));
        if icon.clicked() {
            *checked = !*checked;
            icon.mark_changed();
        };
        icon
    });
    res.inner
}

pub struct Icons;

impl Icons {
    pub const SIZE: f32 = 14.;
    pub const GIZMO_OPTIONS_SIZE: f32 = 25.;
    pub const EDIT_MODE_OPTIONS_SIZE: f32 = 35.;

    pub const START_POINTS_COLOR: Color32 = Color32::from_rgb(80, 80, 255);
    pub const ENEMY_PATHS_COLOR: Color32 = Color32::RED;
    pub const ITEM_PATHS_COLOR: Color32 = Color32::GREEN;
    pub const RESPAWN_POINTS_COLOR: Color32 = Color32::YELLOW;
    pub const OBJECTS_COLOR: Color32 = Color32::from_rgb(255, 0, 255);
    pub const AREAS_COLOR: Color32 = Color32::from_rgb(255, 160, 0);
    pub const CAMERAS_COLOR: Color32 = Color32::from_rgb(160, 0, 255);

    pub fn view_on<'a>(ctx: &Context) -> Image<'a> {
        svg_image(
            include_image!("../../assets/icons/view_on.svg"),
            ctx,
            Self::SIZE,
        )
    }
    pub fn view_off<'a>(ctx: &Context) -> Image<'a> {
        svg_image(
            include_image!("../../assets/icons/view_off.svg"),
            ctx,
            Self::SIZE,
        )
    }
    pub fn path_group<'a>(ctx: &Context) -> Image<'a> {
        svg_image(
            include_image!("../../assets/icons/path_group.svg"),
            ctx,
            Self::SIZE,
        )
    }
    pub fn path<'a>(ctx: &Context) -> Image<'a> {
        svg_image(
            include_image!("../../assets/icons/path.svg"),
            ctx,
            Self::SIZE,
        )
    }
    pub fn cube_group<'a>(ctx: &Context) -> Image<'a> {
        svg_image(
            include_image!("../../assets/icons/cube_group.svg"),
            ctx,
            Self::SIZE,
        )
    }
    pub fn cube<'a>(ctx: &Context) -> Image<'a> {
        svg_image(
            include_image!("../../assets/icons/cube.svg"),
            ctx,
            Self::SIZE,
        )
    }

    pub fn origin_mean<'a>(ctx: &Context) -> Image<'a> {
        svg_image(
            include_image!("../../assets/icons/origin_mean.svg"),
            ctx,
            Self::GIZMO_OPTIONS_SIZE,
        )
    }
    pub fn origin_first_selected<'a>(ctx: &Context) -> Image<'a> {
        svg_image(
            include_image!("../../assets/icons/origin_first_selected.svg"),
            ctx,
            Self::GIZMO_OPTIONS_SIZE,
        )
    }
    pub fn origin_individual<'a>(ctx: &Context) -> Image<'a> {
        svg_image(
            include_image!("../../assets/icons/origin_individual.svg"),
            ctx,
            Self::GIZMO_OPTIONS_SIZE,
        )
    }

    pub fn orient_global<'a>(ctx: &Context) -> Image<'a> {
        svg_image(
            include_image!("../../assets/icons/orient_global.svg"),
            ctx,
            Self::GIZMO_OPTIONS_SIZE,
        )
    }
    pub fn orient_local<'a>(ctx: &Context) -> Image<'a> {
        svg_image(
            include_image!("../../assets/icons/orient_local.svg"),
            ctx,
            Self::GIZMO_OPTIONS_SIZE,
        )
    }

    pub fn tweak<'a>(ctx: &Context) -> Image<'a> {
        svg_image(
            include_image!("../../assets/icons/tweak.svg"),
            ctx,
            Self::EDIT_MODE_OPTIONS_SIZE,
        )
    }
    pub fn select_box<'a>(ctx: &Context) -> Image<'a> {
        svg_image(
            include_image!("../../assets/icons/select_box.svg"),
            ctx,
            Self::EDIT_MODE_OPTIONS_SIZE,
        )
    }
    pub fn translate<'a>(ctx: &Context) -> Image<'a> {
        svg_image(
            include_image!("../../assets/icons/translate.svg"),
            ctx,
            Self::EDIT_MODE_OPTIONS_SIZE,
        )
    }
    pub fn rotate<'a>(ctx: &Context) -> Image<'a> {
        svg_image(
            include_image!("../../assets/icons/rotate.svg"),
            ctx,
            Self::EDIT_MODE_OPTIONS_SIZE,
        )
    }
    pub fn scale<'a>(ctx: &Context) -> Image<'a> {
        svg_image(
            include_image!("../../assets/icons/scale.svg"),
            ctx,
            Self::EDIT_MODE_OPTIONS_SIZE,
        )
    }
}
