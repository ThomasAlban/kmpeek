#![allow(dead_code)]
use bevy::{
    math::{vec3, EulerRot, Quat, Vec3},
    transform::components::Transform,
};
use bevy_egui::egui::{
    self, emath::Numeric, include_image, Align, Align2, Area, Button, CollapsingResponse, Color32, Context, Image,
    ImageButton, ImageSource, Layout, Order, Response, Sense, Ui, Vec2, WidgetText,
};
use std::{
    fmt::Display,
    hash::Hash,
    ops::{AddAssign, SubAssign},
};

pub mod multi_edit {
    use super::{euler_to_quat, quat_to_euler};
    use bevy::{math::Vec3, transform::components::Transform};
    use bevy_egui::egui::{self, emath::Numeric, Checkbox, DragValue, Response, Ui, WidgetText};
    use std::{
        fmt::Display,
        ops::{AddAssign, Sub},
    };

    pub fn rotation_multi_edit<'a>(
        ui: &mut Ui,
        transforms: impl IntoIterator<Item = &'a mut Transform>,
        add_contents: impl FnOnce(&mut Ui, &mut [Vec3]) -> (Response, Response, Response),
    ) -> bool {
        let mut transforms: Vec<_> = transforms.into_iter().collect();
        let mut rots: Vec<_> = transforms.iter().map(|t| quat_to_euler(t)).collect();

        let res = add_contents(ui, &mut rots);

        let changed = res.0.changed() || res.1.changed() || res.2.changed();

        for (transform, new_rot) in transforms.iter_mut().zip(rots.iter()) {
            euler_to_quat(*new_rot, res.clone(), transform);
        }
        changed
    }

    pub fn drag_value_multi_edit<'a, T: 'a + Clone + PartialEq + Numeric + Sub<Output = T> + AddAssign<T>>(
        ui: &mut Ui,
        items: impl IntoIterator<Item = &'a mut T>,
    ) -> Response {
        let mut items: Vec<_> = items.into_iter().collect();
        let mut edit = *items[0];
        let before = edit;

        // if they are all the same
        let res = if items.iter().all(|x| **x == edit) {
            // show normal drag value
            ui.add(DragValue::new(&mut edit))
        } else {
            // show blank drag value
            ui.add(DragValue::new(&mut edit).custom_formatter(|_, _| "".into()))
        };

        if res.changed() && !res.dragged() {
            // if we have set the value by typing it in
            items.iter_mut().for_each(|x| **x = edit);
            return res;
        }
        let delta = edit - before;

        for item in items.iter_mut() {
            **item += delta;
        }
        res
    }

    pub fn combobox_enum_multi_edit<'a, T>(
        ui: &mut Ui,
        id: impl std::hash::Hash,
        width: Option<f32>,
        items: impl IntoIterator<Item = &'a mut T>,
    ) -> Response
    where
        T: 'a + strum::IntoEnumIterator + Display + PartialEq + Clone,
    {
        let mut items: Vec<_> = items.into_iter().collect();
        let mut edit = items[0].clone();

        // if they are all the same
        let mut selected_text = if items.iter().all(|x| **x == edit) {
            // display the value of what they all are
            edit.to_string()
        } else {
            // if they are different, display blank
            "".into()
        };

        let guess_combobox_width = |text: &str| {
            let widget: WidgetText = text.into();
            let galley = widget.into_galley(ui, None, ui.available_width(), egui::TextStyle::Body);
            let text_width = galley.size().x;
            let ui_spacing = &ui.style().spacing;
            text_width + ui_spacing.button_padding.x * 2. + ui_spacing.icon_width + ui_spacing.icon_spacing
        };

        let mut cur_char = if selected_text.is_empty() {
            0
        } else {
            selected_text.len() - 1
        };
        while guess_combobox_width(&selected_text) > ui.available_width() && cur_char > 0 {
            selected_text = format!("{}...", &selected_text[..cur_char]);
            cur_char -= 1;
        }

        let width = if let Some(width) = width {
            ui.available_width().min(width)
        } else {
            ui.available_width()
        };

        let combobox = egui::ComboBox::from_id_source(id)
            .selected_text(selected_text)
            .width(width);

        let mut changed = false;

        let res = combobox
            .show_ui(ui, |ui| {
                for variant in T::iter() {
                    let this_changed = ui
                        .selectable_value(&mut edit, variant.clone(), variant.to_string())
                        .changed();
                    if !changed {
                        changed = this_changed
                    };
                }
            })
            .response;

        if changed {
            items.iter_mut().for_each(|x| **x = edit.clone());
        }

        res
    }

    pub fn checkbox_multi_edit<'a>(ui: &mut Ui, items: impl IntoIterator<Item = &'a mut bool>) {
        let mut items: Vec<_> = items.into_iter().collect();
        let mut edit = *items[0];

        let res = if items.iter().all(|x| **x == edit) {
            ui.add(Checkbox::without_text(&mut edit))
        } else {
            // when we click the intermediate value, set edit to true
            let res = ui.add(Checkbox::without_text(&mut edit).indeterminate(true));
            if res.changed() {
                edit = true
            };
            res
        };

        if res.changed() {
            items.iter_mut().for_each(|x| **x = edit);
        }
    }
}

pub fn combobox_enum<T>(ui: &mut Ui, value: &mut T, id: impl std::hash::Hash, width: Option<f32>) -> Response
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
            .centered_and_justified(|ui| ui.add(egui::DragValue::new(&mut value.x).speed(speed).fixed_decimals(1)))
            .inner;
        let y = ui[1]
            .centered_and_justified(|ui| ui.add(egui::DragValue::new(&mut value.y).speed(speed).fixed_decimals(1)))
            .inner;
        let z = ui[2]
            .centered_and_justified(|ui| ui.add(egui::DragValue::new(&mut value.z).speed(speed).fixed_decimals(1)))
            .inner;
        (x, y, z)
    })
}

fn quat_to_euler(transform: &Transform) -> Vec3 {
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
    rot
}
fn euler_to_quat(rot: Vec3, res: (Response, Response, Response), transform: &mut Transform) {
    let changed = res.0.changed() || res.1.changed() || res.2.changed();

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
    update_rotation(res.0, Vec3::X);
    update_rotation(res.1, Vec3::Y);
    update_rotation(res.2, Vec3::Z);

    if changed {
        transform.rotation = transform.rotation.normalize();
    }
}

pub fn rotation_edit(
    ui: &mut egui::Ui,
    transform: &mut Transform,
    add_contents: impl FnOnce(&mut Ui, &mut Vec3) -> (Response, Response, Response),
) -> bool {
    let mut rot = quat_to_euler(transform);

    let res = add_contents(ui, &mut rot);

    let changed = res.0.changed() || res.1.changed() || res.2.changed();
    euler_to_quat(rot, res, transform);
    changed
}

pub fn framed_collapsing_header<R>(
    header: impl Into<WidgetText>,
    ui: &mut Ui,
    add_body: impl FnOnce(&mut Ui) -> R,
) -> CollapsingResponse<R> {
    ui.visuals_mut().collapsing_header_frame = true;
    egui::CollapsingHeader::new(header)
        .default_open(true)
        .show_unindented(ui, add_body)
}

pub fn button_triggered_popup<R>(ui: &mut Ui, id: impl Hash, btn: Response, add_contents: impl FnOnce(&mut Ui) -> R) {
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
        Icons::view_on(ui.ctx(), 14.)
    } else {
        Icons::view_off(ui.ctx(), 14.)
    };

    let res = ui.allocate_ui(egui::Vec2::splat(14.), |ui| {
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
    pub const SECTION_COLORS: [Color32; 10] = [
        Color32::from_rgb(80, 80, 255),  // Start Points
        Color32::RED,                    // Enemy Paths
        Color32::GREEN,                  // Item Paths
        Color32::GREEN,                  // Checkpoints (todo)
        Color32::YELLOW,                 // Respawn Points
        Color32::from_rgb(255, 0, 255),  // Objects
        Color32::from_rgb(255, 160, 0),  // Areas
        Color32::from_rgb(160, 0, 255),  // Cameras
        Color32::from_rgb(255, 50, 0),   // Cannon Points (todo)
        Color32::from_rgb(50, 170, 170), // Battle Finish Points (todo)
    ];

    pub fn view_on<'a>(ctx: &Context, size: impl Into<f32>) -> Image<'a> {
        svg_image(include_image!("../../assets/icons/view_on.svg"), ctx, size.into())
    }
    pub fn view_off<'a>(ctx: &Context, size: impl Into<f32>) -> Image<'a> {
        svg_image(include_image!("../../assets/icons/view_off.svg"), ctx, size.into())
    }
    pub fn path_group<'a>(ctx: &Context, size: impl Into<f32>) -> Image<'a> {
        svg_image(include_image!("../../assets/icons/path_group.svg"), ctx, size.into())
    }
    pub fn path<'a>(ctx: &Context, size: impl Into<f32>) -> Image<'a> {
        svg_image(include_image!("../../assets/icons/path.svg"), ctx, size.into())
    }
    pub fn cube_group<'a>(ctx: &Context, size: impl Into<f32>) -> Image<'a> {
        svg_image(include_image!("../../assets/icons/cube_group.svg"), ctx, size.into())
    }
    pub fn cube<'a>(ctx: &Context, size: impl Into<f32>) -> Image<'a> {
        svg_image(include_image!("../../assets/icons/cube.svg"), ctx, size.into())
    }

    pub fn origin_mean<'a>(ctx: &Context, size: impl Into<f32>) -> Image<'a> {
        svg_image(include_image!("../../assets/icons/origin_mean.svg"), ctx, size.into())
    }
    pub fn origin_first_selected<'a>(ctx: &Context, size: impl Into<f32>) -> Image<'a> {
        svg_image(
            include_image!("../../assets/icons/origin_first_selected.svg"),
            ctx,
            size.into(),
        )
    }
    pub fn origin_individual<'a>(ctx: &Context, size: impl Into<f32>) -> Image<'a> {
        svg_image(
            include_image!("../../assets/icons/origin_individual.svg"),
            ctx,
            size.into(),
        )
    }

    pub fn orient_global<'a>(ctx: &Context, size: impl Into<f32>) -> Image<'a> {
        svg_image(include_image!("../../assets/icons/orient_global.svg"), ctx, size.into())
    }
    pub fn orient_local<'a>(ctx: &Context, size: impl Into<f32>) -> Image<'a> {
        svg_image(include_image!("../../assets/icons/orient_local.svg"), ctx, size.into())
    }

    pub fn tweak<'a>(ctx: &Context, size: impl Into<f32>) -> Image<'a> {
        svg_image(include_image!("../../assets/icons/tweak.svg"), ctx, size.into())
    }
    pub fn select_box<'a>(ctx: &Context, size: impl Into<f32>) -> Image<'a> {
        svg_image(include_image!("../../assets/icons/select_box.svg"), ctx, size.into())
    }
    pub fn translate<'a>(ctx: &Context, size: impl Into<f32>) -> Image<'a> {
        svg_image(include_image!("../../assets/icons/translate.svg"), ctx, size.into())
    }
    pub fn rotate<'a>(ctx: &Context, size: impl Into<f32>) -> Image<'a> {
        svg_image(include_image!("../../assets/icons/rotate.svg"), ctx, size.into())
    }
    pub fn scale<'a>(ctx: &Context, size: impl Into<f32>) -> Image<'a> {
        svg_image(include_image!("../../assets/icons/scale.svg"), ctx, size.into())
    }
}
