#![allow(dead_code)]
use bevy::ecs::system::SystemState;
use bevy::math::{vec3, Dir3, EulerRot, Quat};
use bevy::prelude::{Entity, Has, Query, With, World};
use bevy::window::{PrimaryWindow, Window};
use bevy::{math::Vec3, transform::components::Transform};
use bevy_egui::egui::{self, pos2, vec2, Rect, Response, TextStyle, Ui, WidgetText};
use bevy_egui::egui::{
    Align, Align2, Area, CollapsingResponse, Color32, Context, Image, ImageButton, ImageSource, Order, Sense, Vec2,
};
use bevy_egui::{EguiContext, EguiContexts};
use std::{fmt::Display, hash::Hash};

pub fn get_egui_ctx(world: &mut World) -> Context {
    let mut system_state = SystemState::<Query<&mut EguiContext, With<PrimaryWindow>>>::new(world);
    let mut q = system_state.get_mut(world);
    q.single_mut().get_mut().clone()
}

#[derive(Clone, Copy)]
pub enum DragSpeed {
    Slow,
    Medium,
    Fast,
}
impl From<DragSpeed> for f64 {
    fn from(value: DragSpeed) -> Self {
        match value {
            DragSpeed::Slow => 0.05,
            DragSpeed::Medium => 1.,
            DragSpeed::Fast => 5.,
        }
    }
}

pub mod multi_edit {
    use super::{euler_to_quat, quat_to_euler, DragSpeed};
    use bevy::{math::Vec3, prelude::Mut, transform::components::Transform};
    use bevy_egui::egui::{self, emath::Numeric, Checkbox, DragValue, Response, Ui, WidgetText};
    use std::{
        fmt::Display,
        ops::{AddAssign, Sub, SubAssign},
    };

    /// Maps an iterator to a child of each element of that iterator
    macro_rules! map {
        ($iter:ident => 0 $($fields:tt)*) => {
            $iter.iter_mut().map(|x| x.0.reborrow().map_unchanged(|x| &mut x.$($fields)*))
        };
        ($iter:ident => 1 $($fields:tt)*) => {
            $iter.iter_mut().map(|x| x.1.reborrow().map_unchanged(|x| &mut x.$($fields)*))
        };
        ($iter:ident => $($fields:tt)*) => {
            $iter.iter_mut().map(|x| x.reborrow().map_unchanged(|x| &mut x.$($fields)*))
        };
    }
    pub(crate) use map;

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

    // pub fn rotation_multi_edit_mut<'a>(
    //     ui: &mut Ui,
    //     transforms: impl IntoIterator<Item = Mut<'a, Transform>>,
    //     add_contents: impl FnOnce(&mut Ui, &mut [Mut<Vec3>]) -> (Response, Response, Response),
    // ) -> bool {
    //     let mut transforms: Vec<_> = transforms.into_iter().collect();
    //     let mut rots: Vec<_> = transforms.iter().map(|t| quat_to_euler(t)).collect();

    //     let res = add_contents(ui, &mut rots);

    //     let changed = res.0.changed() || res.1.changed() || res.2.changed();

    //     for (transform, new_rot) in transforms.iter_mut().zip(rots.iter()) {
    //         euler_to_quat(*new_rot, res.clone(), transform);
    //     }
    //     changed
    // }

    pub fn drag_value_multi_edit<
        'a,
        T: 'a + Clone + PartialEq + Numeric + Sub<Output = T> + AddAssign<T> + SubAssign<T>,
    >(
        ui: &mut Ui,
        speed: DragSpeed,
        items: impl IntoIterator<Item = Mut<'a, T>>,
    ) -> Response {
        let mut items: Vec<_> = items.into_iter().collect();
        let mut edit = *items[0];
        let before = edit;

        // if they are all the same
        let res = if items.iter().all(|x| **x == edit) {
            // show normal drag value
            ui.add(DragValue::new(&mut edit).speed(speed))
        } else {
            // show blank drag value
            ui.add(
                DragValue::new(&mut edit)
                    .speed(speed)
                    .custom_formatter(|_, _| "".into()),
            )
        };

        if res.changed() && !res.dragged() {
            // if we have set the value by typing it in
            items.iter_mut().for_each(|x| **x = edit);
            return res;
        }

        // we cannot simply calculate the delta and add it to the value, because that might be out of bounds of the type T (for example if it is a usize)
        let positive_delta = if edit > before { edit - before } else { before - edit };
        for item in items.iter_mut() {
            // work out the f64 result which may be negative
            let f64_result = if edit > before {
                item.to_f64() + positive_delta.to_f64()
            } else {
                item.to_f64() - positive_delta.to_f64()
            };
            // if the f64 result is out of bounds of the value, then we continue, as attempting to apply this delta would crash the program
            if f64_result < T::MIN.to_f64() || f64_result > T::MAX.to_f64() {
                continue;
            }
            if edit > before {
                **item += positive_delta
            } else {
                **item -= positive_delta
            };
        }
        res
    }

    pub fn combobox_enum_multi_edit<'a, T>(
        ui: &mut Ui,
        width: Option<f32>,
        items: impl IntoIterator<Item = Mut<'a, T>>,
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

        let combobox = egui::ComboBox::from_id_source(ui.next_auto_id())
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

    pub fn checkbox_multi_edit<'a>(ui: &mut Ui, items: impl IntoIterator<Item = Mut<'a, bool>>) -> Response {
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
        res
    }
}

pub fn combobox_enum<T>(ui: &mut Ui, value: &mut T, width: Option<f32>) -> Response
where
    T: strum::IntoEnumIterator + Display + PartialEq + Clone,
{
    let mut combobox = egui::ComboBox::from_id_source(ui.next_auto_id()).selected_text(value.to_string());
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

pub fn drag_vec3(ui: &mut Ui, value: &mut Vec3, speed: impl Into<f64>) -> (Response, Response, Response) {
    let speed = speed.into();
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

pub fn quat_to_euler(transform: &Transform) -> Vec3 {
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
pub fn euler_to_quat(rot: Vec3, res: (Response, Response, Response), transform: &mut Transform) {
    let changed = res.0.changed() || res.1.changed() || res.2.changed();

    let mut update_rotation = |res: Response, axis: Dir3| {
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
    update_rotation(res.0, Dir3::X);
    update_rotation(res.1, Dir3::Y);
    update_rotation(res.2, Dir3::Z);

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

pub fn button_triggered_popup<R>(
    ui: &mut Ui,
    id: impl Hash,
    btn: Response,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> Option<Response> {
    let popup_id = ui.make_persistent_id(id);
    if btn.clicked() {
        ui.memory_mut(|mem| mem.toggle_popup(popup_id));
    }
    let mut res: Option<Response> = None;

    if ui.memory(|mem| mem.is_popup_open(popup_id)) {
        let (pos, pivot) = (btn.rect.left_bottom(), Align2::LEFT_TOP);

        let r = Area::new(popup_id)
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
            })
            .response;
        res = Some(r.clone());

        let clicked_elsewhere = r.clicked_elsewhere() && btn.clicked_elsewhere();

        if ui.input(|i| i.key_pressed(egui::Key::Escape)) || clicked_elsewhere {
            ui.memory_mut(|mem| mem.close_popup());
        }
    }
    res
}

#[derive(Clone)]
pub enum LinkSelectBtnType {
    /// All that is selected has no link
    NoLink,
    /// There are multiple things selected with differing links
    Multi { indexes: Vec<Option<usize>>, visible: bool },
    /// All that is selected is linked to the same thing
    Single { index: usize, visible: bool },
}
#[derive(Default)]
pub struct LinkSelectBtnResponse {
    pub cross_pressed: bool,
    pub eyedropper_pressed: bool,
    pub view_pressed: bool,
}
pub fn link_select_btn(
    ui: &mut Ui,
    route_btn_type: &LinkSelectBtnType,
    name: impl Into<String>,
) -> LinkSelectBtnResponse {
    use LinkSelectBtnType::*;

    let mut res = LinkSelectBtnResponse::default();

    let width = ui.available_width();

    ui.spacing_mut().item_spacing = Vec2::splat(0.);

    let bg_size = vec2(width, ui.spacing().interact_size.y);
    let icon_hb_size = vec2(
        ui.spacing().icon_width_inner + 1.5 * ui.spacing().icon_spacing,
        ui.spacing().interact_size.y,
    );

    // allocate the background
    let (bg_rect, _bg_res) = ui.allocate_at_least(bg_size, Sense::hover());
    let bg_visuals = ui.style().visuals.widgets.inactive;

    // paint the background
    ui.painter()
        .rect_filled(bg_rect, bg_visuals.rounding, bg_visuals.weak_bg_fill);

    let mut next_icon_hb_rect = Rect::from_min_size(
        bg_rect.right_top() - vec2(ui.spacing().button_padding.x + icon_hb_size.x, 0.),
        icon_hb_size,
    );

    // do the cross

    if let Multi { .. } | Single { .. } = route_btn_type {
        // allocate the cross icon hitbox
        let cross_hb_res = ui
            .allocate_rect(next_icon_hb_rect, Sense::click())
            .on_hover_text_at_pointer("Delete route Link");

        res.cross_pressed = cross_hb_res.clicked();

        // draw the cross
        let cross_middle = next_icon_hb_rect.center();
        let cross_size = ui.spacing().icon_width_inner;
        let cross_rect = Rect::from_center_size(cross_middle, Vec2::splat(cross_size));
        let cross_color = ui.style().interact(&cross_hb_res).fg_stroke;

        ui.painter() // paints \
            .line_segment([cross_rect.left_top(), cross_rect.right_bottom()], cross_color);
        ui.painter() // paints /
            .line_segment([cross_rect.right_top(), cross_rect.left_bottom()], cross_color);

        next_icon_hb_rect = next_icon_hb_rect.translate(vec2(-icon_hb_size.x, 0.));
    }

    // do the eyedropper

    // allocate the eyedropper hitbox
    let eyedropper_hb_res = ui
        .allocate_rect(next_icon_hb_rect, Sense::click())
        .on_hover_text_at_pointer("Link route");
    res.eyedropper_pressed = eyedropper_hb_res.clicked();

    // draw the eyedropper
    let eyedropper_middle = next_icon_hb_rect.center();
    let eyedropper_size = ui.spacing().icon_width_inner * 1.2;
    let eyedropper_rect = Rect::from_center_size(eyedropper_middle, Vec2::splat(eyedropper_size));
    let eyedropper_color = ui.style().interact(&eyedropper_hb_res).fg_stroke.color;

    Icons::eyedropper(ui.ctx(), eyedropper_size)
        .tint(eyedropper_color)
        .paint_at(ui, eyedropper_rect);

    next_icon_hb_rect = next_icon_hb_rect.translate(vec2(-icon_hb_size.x, 0.));

    // do the view btn

    if let Multi { visible, .. } | Single { visible, .. } = route_btn_type {
        // allocate the view hitbox
        let view_hb_res = ui
            .allocate_rect(next_icon_hb_rect, Sense::click())
            .on_hover_text_at_pointer("View linked route");
        res.view_pressed = view_hb_res.clicked();

        // draw the view btn
        let view_middle = next_icon_hb_rect.center();
        let view_size = ui.spacing().icon_width_inner * 1.3;
        let view_rect = Rect::from_center_size(view_middle, Vec2::splat(view_size));

        if *visible {
            Icons::view_on(ui.ctx(), view_size)
        } else {
            Icons::view_off(ui.ctx(), view_size)
        }
        .paint_at(ui, view_rect);
    }

    // draw the text
    let text: WidgetText = match route_btn_type {
        NoLink => "None".into(),
        Multi { .. } => "".into(),
        Single { index, .. } => format!("{} {}", name.into(), index).into(),
    };
    let galley = text.into_galley(ui, None, f32::INFINITY, TextStyle::Button);

    let text_start = pos2(
        bg_rect.min.x + ui.spacing().button_padding.x,
        bg_rect.center().y - 0.5 * galley.size().y,
    );

    ui.painter()
        .galley(text_start, galley, ui.style().visuals.widgets.inactive.text_color());

    res
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

macro_rules! impl_img {
    ($name:ident) => {
        impl Icons {
            pub fn $name<'a>(ctx: &Context, size: impl Into<f32>) -> Image<'a> {
                svg_image(
                    egui::ImageSource::Bytes {
                        uri: ::std::borrow::Cow::Borrowed(concat!(
                            "bytes://../../assets/icons/",
                            stringify!($name),
                            ".svg"
                        )),
                        bytes: egui::load::Bytes::Static(include_bytes!(concat!(
                            "../../assets/icons/",
                            stringify!($name),
                            ".svg"
                        ))),
                    },
                    ctx,
                    size.into(),
                )
            }
        }
    };
}
impl_img!(cube);
impl_img!(cube_group);
impl_img!(orient_global);
impl_img!(orient_local);
impl_img!(path);
impl_img!(path_group);
impl_img!(pivot_first_selected);
impl_img!(pivot_individual);
impl_img!(pivot_median);
impl_img!(rotate);
impl_img!(scale);
impl_img!(select_box);
impl_img!(track_info);
impl_img!(translate);
impl_img!(tweak);
impl_img!(view_off);
impl_img!(view_on);
impl_img!(eyedropper);

impl Icons {
    pub const SECTION_COLORS: [Color32; 12] = [
        Color32::from_rgb(80, 80, 255),  // Start Points
        Color32::RED,                    // Enemy Paths
        Color32::GREEN,                  // Item Paths
        Color32::from_rgb(70, 190, 255), // Checkpoints (todo)
        Color32::YELLOW,                 // Respawn Points
        Color32::from_rgb(255, 0, 255),  // Objects
        Color32::from_rgb(70, 190, 255), // Routes (todo)
        Color32::from_rgb(255, 160, 0),  // Areas
        Color32::from_rgb(160, 0, 255),  // Cameras
        Color32::from_rgb(255, 50, 0),   // Cannon Points
        Color32::from_rgb(50, 170, 170), // Battle Finish Points
        Color32::WHITE,                  // Track Info
    ];
}
