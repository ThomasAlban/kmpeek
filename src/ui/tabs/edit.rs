use crate::{
    ui::util::{
        combobox_enum, framed_collapsing_header, link_select_btn,
        multi_edit::{checkbox_multi_edit, combobox_enum_multi_edit, drag_value_multi_edit, map, rotation_multi_edit},
        DragSpeed, Icons, LinkSelectBtnType,
    },
    util::{give_me_a_mut, iter_mut_from_entities},
    viewer::{
        edit::{link_select_mode::LinkSelectMode, select::Selected},
        kmp::{
            checkpoints::{CheckpointRespawnLink, GetSelectedCheckpoints},
            components::{
                AreaKind, AreaPoint, BattleFinishPoint, CannonPoint, Checkpoint, CheckpointKind, EnemyPathPoint,
                ItemPathPoint, KmpCamera, KmpCameraIntroStart, Object, PathOverallStart, RespawnPoint, RoutePoint,
                RouteSettings, StartPoint, TrackInfo, TransformEditOptions,
            },
            ordering::OrderId,
            path::{EntityPathGroups, PathType, RecalcPaths, ToPathType},
            routes::{GetRouteStart, RouteLink, RouteLinkedEntities},
            sections::KmpEditMode,
        },
    },
};
use bevy::{
    ecs::{
        entity::EntityHashSet,
        query::{QueryData, WorldQuery},
        system::{SystemParam, SystemState},
    },
    log::warn,
    prelude::*,
};
use bevy_egui::egui::{self, emath::Numeric, Align, Checkbox, DragValue, Layout, Response, Sense, Ui, WidgetText};
use std::{
    fmt::Display,
    ops::{AddAssign, Sub, SubAssign},
};

pub fn show_edit_tab(ui: &mut Ui, world: &mut World) {
    edit_track_info(ui, world);

    edit_component::<(Option<&TransformEditOptions>, &mut Transform), ()>(ui, world, "Transform", |ui, items, _| {
        let all_hide_rot = items.iter().all(|x| x.0.is_some_and(|x| x.hide_rotation));
        let all_hide_y_tr = items.iter().all(|x| x.0.is_some_and(|x| x.hide_y_translation));

        drag_value_edit_row(ui, "Translation X", DragSpeed::Fast, map!(items => 1 translation.x));
        if !all_hide_y_tr {
            drag_value_edit_row(ui, "Y", DragSpeed::Fast, map!(items => 1 translation.y));
        }
        drag_value_edit_row(ui, "Z", DragSpeed::Fast, map!(items => 1 translation.z));

        if !all_hide_rot {
            edit_spacing(ui);
            rotation_multi_edit(ui, items.iter_mut().map(|(_, x)| &mut **x), |ui, rots| {
                give_me_a_mut(rots, |rots| {
                    let [x, y, z] = vec3_drag_value_edit_row(ui, "Rotation", DragSpeed::Slow, rots);
                    (x, y, z)
                })
            });
        }
    });

    edit_component::<&mut StartPoint, ()>(ui, world, "Start Point", |ui, items, _| {
        drag_value_edit_row(ui, "Player Index", DragSpeed::Slow, map!(items => player_index));
    });

    edit_component::<(&mut EnemyPathPoint, Entity), PathStartBtn<EnemyPathPoint>>(
        ui,
        world,
        "Enemy Point",
        |ui, items, mut path_start_btn| {
            drag_value_edit_row(ui, "Leniency", DragSpeed::Slow, map!(items => 0 leniency));
            combobox_edit_row(ui, "Setting 1", map!(items => 0 setting_1));
            combobox_edit_row(ui, "Setting 2", map!(items => 0 setting_2));
            drag_value_edit_row(ui, "Setting 3", DragSpeed::Slow, map!(items => 0 setting_3));
            edit_spacing(ui);
            path_start_btn.show(ui, items.iter().map(|x| x.1));
        },
    );

    edit_component::<(&mut ItemPathPoint, Entity), PathStartBtn<ItemPathPoint>>(
        ui,
        world,
        "Item Point",
        |ui, items, mut path_start_btn| {
            drag_value_edit_row(ui, "Bullet Control", DragSpeed::Slow, map!(items => 0 bullet_control));
            edit_spacing(ui);
            combobox_edit_row(ui, "Bullet Height", map!(items => 0 bullet_height));
            checkbox_edit_row(ui, "Bullet Can't Drop", map!(items => 0 bullet_cant_drop));
            checkbox_edit_row(ui, "Low Shell Priority", map!(items => 0 low_shell_priority));
            edit_spacing(ui);
            path_start_btn.show(ui, items.iter().map(|x| x.1));
        },
    );

    edit_component_entities::<
        GetSelectedCheckpoints,
        (
            Query<(Entity, &mut Checkpoint)>,
            PathStartBtn<Checkpoint>,
            Query<&CheckpointRespawnLink>,
            Query<&mut Visibility>,
            Query<&OrderId>,
            Commands,
        ),
    >(
        ui,
        world,
        |cps| cps.get_entities(),
        "Checkpoint",
        |ui,
         entities,
         (mut q_cp, mut path_start_btn, q_cp_respawn_link, mut q_visibility, q_order_id, mut commands)| {
            let mut items = iter_mut_from_entities(&entities, &mut q_cp);
            combobox_edit_row(ui, "Type", map!(items => kind));

            // see https://github.com/bevyengine/bevy/pull/14837
            let kcp_ids: Vec<Mut<u8>> = items
                .iter_mut()
                .filter_map(|x| {
                    if let CheckpointKind::Key(_) = x.bypass_change_detection().kind {
                        Some(x.reborrow().map_unchanged(|y| {
                            if let CheckpointKind::Key(id) = &mut y.kind {
                                id
                            } else {
                                unreachable!()
                            }
                        }))
                    } else {
                        None
                    }
                })
                .collect();
            if !kcp_ids.is_empty() {
                drag_value_edit_row(ui, "Key Checkpoint ID", DragSpeed::Slow, kcp_ids);
            }

            edit_row(ui, "Respawn", false, |ui| {
                let mut cp_respawn_links = Vec::new();
                for e in entities.iter() {
                    cp_respawn_links.push(q_cp_respawn_link.get(*e).ok());
                }
                // list of Option<bool>
                // none if no link, bool represents visibility if it does exist
                let mut visibilities = Vec::new();
                for link in cp_respawn_links.iter() {
                    visibilities
                        .push((*link).and_then(|x| q_visibility.get(x.0).ok().map(|x| x == Visibility::Visible)));
                }

                // // looks weird but basically means 'go through all the visibilities which exist (skipping the ones that don't) and ask if all of them are visible or not'
                let all_visible = visibilities.iter().filter_map(|x| *x).all(|x| x);

                let link_select_btn_type = {
                    if cp_respawn_links.iter().all(|x| x.is_none()) {
                        LinkSelectBtnType::NoLink
                    } else {
                        let first_link = cp_respawn_links[0];

                        // if they are all the same (and we know already that they are not all none)
                        // so this means 'are we talking about a single respawn point
                        if cp_respawn_links.iter().all(|x| *x == first_link) {
                            let index = q_order_id.get(first_link.unwrap().0).unwrap().0 as usize;
                            LinkSelectBtnType::Single {
                                index,
                                visible: all_visible,
                            }
                        } else {
                            LinkSelectBtnType::Multi {
                                indexes: Vec::new(),
                                visible: all_visible,
                            }
                        }
                    }
                };

                let res = link_select_btn(ui, &link_select_btn_type, "Respawn");

                if res.cross_pressed {
                    for e in entities.iter() {
                        commands.entity(*e).remove::<CheckpointRespawnLink>();
                    }
                }
                if res.view_pressed {
                    for e in cp_respawn_links.iter().filter_map(|x| *x).map(|x| x.0) {
                        let mut v_mut = q_visibility.get_mut(e).unwrap();
                        *v_mut = if all_visible {
                            Visibility::Hidden
                        } else {
                            Visibility::Visible
                        };
                    }
                }
                if res.eyedropper_pressed {
                    commands.insert_resource(LinkSelectMode::<RespawnPoint>::new(entities.clone()));
                }
            });

            path_start_btn.show(ui, entities);
        },
    );

    edit_component::<&mut RespawnPoint, ()>(ui, world, "Respawn Point", |ui, items, _| {
        drag_value_edit_row(ui, "Sound Trigger", DragSpeed::Slow, map!(items => sound_trigger));
    });

    edit_component::<(&mut Object, Entity), RouteEditRowParam>(ui, world, "Object", |ui, items, mut route_edit_row| {
        vec3_drag_value_edit_row(ui, "Scale", DragSpeed::Fast, map!(items => 0 scale));
        edit_spacing(ui);
        drag_value_edit_row(ui, "ID", DragSpeed::Slow, map!(items => 0 object_id));
        edit_spacing(ui);
        for i in 0..8 {
            drag_value_edit_row(
                ui,
                format!("Setting {}", i + 1),
                DragSpeed::Slow,
                map!(items => 0 settings[i]),
            );
        }
        edit_spacing(ui);
        route_edit_row.show(ui, items.iter().map(|x| x.1));
    });

    edit_component_entities::<
        GetRouteStart,
        (
            Query<(Entity, (&mut RouteSettings, &RouteLinkedEntities))>,
            Query<&mut Visibility>,
        ),
    >(
        ui,
        world,
        |r| r.get_selected(),
        "Route Settings",
        |ui, entities, (mut q, mut q_visibility)| {
            let mut items = iter_mut_from_entities(&entities, &mut q);

            checkbox_edit_row(ui, "Smooth Motion", map!(items => 0 smooth_motion));
            combobox_edit_row(ui, "Loop Style", map!(items => 0 loop_style));

            let mut all_visible = true;
            let mut iterated = false;
            for (_, route_linked_es) in items.iter() {
                for e in route_linked_es.iter() {
                    iterated = true;
                    let visible = q_visibility.get(*e).unwrap() == Visibility::Visible;
                    if !visible {
                        all_visible = false
                    };
                }
            }
            if !iterated {
                all_visible = false;
            }

            let res = edit_row(ui, "Show Linked", false, |ui| {
                let size = egui::Vec2::splat(ui.spacing().interact_size.y * 0.8);
                ui.add_sized(
                    size,
                    if all_visible {
                        Icons::view_on(ui.ctx(), ui.spacing().interact_size.y * 0.8)
                    } else {
                        Icons::view_off(ui.ctx(), ui.spacing().interact_size.y * 0.8)
                    }
                    .sense(Sense::click()),
                )
            });

            if res.clicked() {
                for (_, route_linked_es) in items.iter() {
                    for e in route_linked_es.iter() {
                        let mut v = q_visibility.get_mut(*e).unwrap();
                        *v = if all_visible {
                            Visibility::Hidden
                        } else {
                            Visibility::Visible
                        };
                    }
                }
            }
        },
    );

    edit_component::<&mut RoutePoint, ()>(ui, world, "Route Point", |ui, items, _| {
        drag_value_edit_row(ui, "Settings", DragSpeed::Slow, map!(items => settings));
        drag_value_edit_row(
            ui,
            "Additional Settings",
            DragSpeed::Slow,
            map!(items => additional_settings),
        );
    });

    edit_component::<&mut AreaPoint, ()>(ui, world, "Area", |ui, items, _| {
        vec3_drag_value_edit_row(ui, "Scale", DragSpeed::Slow, map!(items => scale));
        edit_spacing(ui);
        combobox_edit_row(ui, "Shape", map!(items => shape));
        drag_value_edit_row(ui, "Priority", DragSpeed::Slow, map!(items => priority));
        combobox_edit_row(ui, "Type", map!(items => kind));

        // for now, area type UI settings will only work when 1 point is selected
        if let Some(item) = items.iter_mut().next() {
            match &mut item.kind {
                AreaKind::Camera { cam_index } => {
                    edit_row(ui, "Camera Index", true, |ui| {
                        ui.add(DragValue::new(cam_index).speed(DragSpeed::Slow));
                    });
                }
                AreaKind::EnvEffect(env_effect_obj) => {
                    edit_row(ui, "Env Effect Object", true, |ui| {
                        combobox_enum(ui, env_effect_obj, None);
                    });
                }
                AreaKind::FogEffect { bfg_entry, setting_2 } => {
                    edit_row(ui, "BFG Entry", true, |ui| {
                        ui.add(DragValue::new(bfg_entry).speed(DragSpeed::Slow));
                    });
                    edit_row(ui, "Setting 2", true, |ui| {
                        ui.add(DragValue::new(setting_2).speed(DragSpeed::Slow));
                    });
                }
                AreaKind::MovingRoad => {
                    // TODO - add route link here
                }
                AreaKind::MinimapControl { setting_1, setting_2 } => {
                    edit_row(ui, "Setting 1", true, |ui| {
                        ui.add(DragValue::new(setting_1).speed(DragSpeed::Slow));
                    });
                    edit_row(ui, "Setting 2", true, |ui| {
                        ui.add(DragValue::new(setting_2).speed(DragSpeed::Slow));
                    });
                }
                AreaKind::BloomEffect { bblm_file, fade_time } => {
                    edit_row(ui, "BBLM File", true, |ui| {
                        ui.add(DragValue::new(bblm_file).speed(DragSpeed::Slow));
                    });
                    edit_row(ui, "Fade Time", true, |ui| {
                        ui.add(DragValue::new(fade_time).speed(DragSpeed::Slow));
                    });
                }
                AreaKind::ObjectGroup { group_id } | AreaKind::ObjectUnload { group_id } => {
                    edit_row(ui, "Group ID", true, |ui| {
                        ui.add(DragValue::new(group_id).speed(DragSpeed::Slow));
                    });
                }
                // other types of area don't have any settings
                _ => {}
            }
        }
        edit_spacing(ui);
        checkbox_edit_row(ui, "Always Show Area", map!(items => show_area));
    });

    edit_component::<(&mut KmpCamera, Entity), (RouteEditRowParam, Query<Entity, With<KmpCameraIntroStart>>, Commands)>(
        ui,
        world,
        "Camera",
        |ui, items, (mut route_edit_row, q_cam_start, mut commands)| {
            edit_row(ui, "Intro Start", false, |ui| {
                let mut intro_start_in_items = items.iter().any(|x| q_cam_start.contains(x.1));
                let intermediate = intro_start_in_items && items.len() > 1;
                ui.add_enabled_ui(false, |ui| {
                    ui.add(Checkbox::without_text(&mut intro_start_in_items).indeterminate(intermediate));
                });
                if items.len() == 1 {
                    let e = items[0].1;
                    if ui.button("Set").clicked() {
                        for e in q_cam_start.iter() {
                            commands.entity(e).remove::<KmpCameraIntroStart>();
                        }
                        commands.entity(e).insert(KmpCameraIntroStart);
                    }
                }
            });
            edit_spacing(ui);

            combobox_edit_row(ui, "Type", map!(items => 0 kind));
            edit_spacing(ui);
            drag_value_edit_row(ui, "Next Index", DragSpeed::Slow, map!(items => 0 next_index));

            route_edit_row.show(ui, items.iter().map(|x| x.1));

            edit_spacing(ui);
            drag_value_edit_row(ui, "Time", DragSpeed::Slow, map!(items => 0 time));
            edit_spacing(ui);
            drag_value_edit_row(ui, "Point Speed", DragSpeed::Slow, map!(items => 0 point_velocity));
            drag_value_edit_row(ui, "Zoom Speed", DragSpeed::Slow, map!(items => 0 zoom_velocity));
            drag_value_edit_row(ui, "View Speed", DragSpeed::Slow, map!(items => 0 view_velocity));
            edit_spacing(ui);
            drag_value_edit_row(ui, "Zoom Start", DragSpeed::Slow, map!(items => 0 zoom_end));
            edit_spacing(ui);
            vec3_drag_value_edit_row(ui, "View Start", DragSpeed::Slow, map!(items => 0 view_start));
            edit_spacing(ui);
            vec3_drag_value_edit_row(ui, "View End", DragSpeed::Slow, map!(items => 0 view_end));
            edit_spacing(ui);
            drag_value_edit_row(ui, "Shake (?)", DragSpeed::Slow, map!(items => 0 shake));
            drag_value_edit_row(ui, "Start (?)", DragSpeed::Slow, map!(items => 0 start));
            drag_value_edit_row(ui, "Movie (?)", DragSpeed::Slow, map!(items => 0 movie));
        },
    );

    edit_component::<&mut CannonPoint, ()>(ui, world, "Cannon Point", |ui, items, _| {
        combobox_edit_row(ui, "Shoot Effect", map!(items => shoot_effect));
    });

    edit_component::<&mut BattleFinishPoint, ()>(ui, world, "Battle Finish Point", |_, _, _| {});
}

fn edit_track_info(ui: &mut Ui, world: &mut World) {
    if *world.resource::<KmpEditMode>() != KmpEditMode::TrackInfo {
        return;
    }

    let Some(mut track_info) = world.get_resource_mut::<TrackInfo>() else {
        return;
    };

    framed_collapsing_header("Track Info", ui, |ui| {
        edit_row(ui, "Track Type", false, |ui| {
            combobox_enum(ui, &mut track_info.track_type, None);
        });
        edit_row(ui, "Lap Count", true, |ui| {
            ui.add(DragValue::new(&mut track_info.lap_count).speed(DragSpeed::Slow))
        });
        edit_row(ui, "Speed Mod", true, |ui| {
            ui.add(DragValue::new(&mut track_info.speed_mod).speed(DragSpeed::Slow))
        });
        edit_spacing(ui);
        edit_row(ui, "Lens Flare Colour", false, |ui| {
            ui.color_edit_button_srgba_unmultiplied(&mut track_info.lens_flare_color);
        });
        edit_row(ui, "Lens Flare Flashing", false, |ui| {
            ui.add(Checkbox::without_text(&mut track_info.lens_flare_flashing));
        });
        edit_spacing(ui);
        edit_row(ui, "First Player Pos", false, |ui| {
            combobox_enum(ui, &mut track_info.first_player_pos, None);
        });
        edit_row(ui, "Narrow Player Spacing", false, |ui| {
            ui.add(Checkbox::without_text(&mut track_info.narrow_player_spacing));
        });
    });
    edit_spacing(ui);
}

fn edit_component<D: QueryData + 'static, P: SystemParam + 'static>(
    ui: &mut Ui,
    world: &mut World,
    title: &'static str,
    add_body: impl FnOnce(&mut Ui, &mut [<D as WorldQuery>::Item<'_>], <P as SystemParam>::Item<'_, '_>),
) {
    let mut system_state = SystemState::<(Query<D, With<Selected>>, P)>::new(world);
    {
        let (mut q, p) = system_state.get_mut(world);

        let mut items: Vec<_> = q.iter_mut().collect();
        if items.is_empty() {
            return;
        }
        let title = edit_component_title(title, items.len());

        framed_collapsing_header(title, ui, |ui| add_body(ui, &mut items, p));
        edit_spacing(ui);
    }
    system_state.apply(world);
}

fn edit_component_entities<PEntities: SystemParam + 'static, P: SystemParam + 'static>(
    ui: &mut Ui,
    world: &mut World,
    get_entities: impl FnOnce(<PEntities as SystemParam>::Item<'_, '_>) -> EntityHashSet,
    title: &'static str,
    add_body: impl FnOnce(&mut Ui, EntityHashSet, <P as SystemParam>::Item<'_, '_>),
) {
    let mut ss = SystemState::<ParamSet<(PEntities, P)>>::new(world);
    let mut paramset = ss.get_mut(world);

    let p_entities = paramset.p0();

    let entities = get_entities(p_entities);

    if entities.is_empty() {
        return;
    }
    let title = edit_component_title(title, entities.len());

    framed_collapsing_header(title, ui, |ui| add_body(ui, entities, paramset.p1()));
    edit_spacing(ui);

    ss.apply(world);
}

#[derive(SystemParam)]
struct PathStartBtn<'w, 's, T: Component + ToPathType> {
    commands: Commands<'w, 's>,
    q_path_start: Query<'w, 's, Entity, (With<PathOverallStart>, With<T>)>,
    ev_recalc_paths: EventWriter<'w, RecalcPaths>,
}
impl<T: Component + ToPathType> PathStartBtn<'_, '_, T> {
    fn show(&mut self, ui: &mut Ui, items: impl IntoIterator<Item = Entity>) {
        let items: Vec<_> = items.into_iter().collect();
        ui.with_layout(Layout::top_down(Align::Center), |ui| {
            if items.len() != 1 {
                ui.disable();
            }
            if ui.button("Set As Path Start").clicked() && items.len() == 1 {
                for e in self.q_path_start.iter() {
                    self.commands.entity(e).remove::<PathOverallStart>();
                }
                self.commands.entity(items[0]).insert(PathOverallStart);

                let ev = match T::to_path_type() {
                    PathType::Enemy => RecalcPaths::enemy(),
                    PathType::Item => RecalcPaths::item(),
                    PathType::Checkpoint { .. } => RecalcPaths::cp(),
                    PathType::Route => RecalcPaths::route(),
                };
                self.ev_recalc_paths.send(ev);
            }
        });
    }
}

fn edit_component_title(name: impl Into<String>, num: usize) -> String {
    let name = name.into();
    if num > 1 {
        format!("{} ({})", name, num)
    } else {
        name
    }
}

pub fn edit_spacing(ui: &mut Ui) {
    ui.vertical(|ui| ui.add_space(3.));
}

pub fn drag_value_edit_row<'a, T: 'a + Clone + PartialEq + Numeric + Sub<Output = T> + AddAssign<T> + SubAssign<T>>(
    ui: &mut Ui,
    label: impl Into<WidgetText>,
    speed: DragSpeed,
    items: impl IntoIterator<Item = Mut<'a, T>>,
) -> Response {
    edit_row(ui, label, true, |ui| drag_value_multi_edit(ui, speed, items))
}

pub fn vec3_drag_value_edit_row<'a>(
    ui: &mut Ui,
    label: impl Into<String>,
    speed: DragSpeed,
    items: impl IntoIterator<Item = Mut<'a, Vec3>>,
) -> [Response; 3] {
    let mut items: Vec<_> = items.into_iter().collect();
    let x_label = format!("{} X", label.into());
    [
        edit_row(ui, x_label, true, |ui| {
            drag_value_multi_edit(ui, speed, map!(items => x))
        }),
        edit_row(ui, "Y", true, |ui| drag_value_multi_edit(ui, speed, map!(items => y))),
        edit_row(ui, "Z", true, |ui| drag_value_multi_edit(ui, speed, map!(items => z))),
    ]
}

pub fn combobox_edit_row<'a, T: 'a + strum::IntoEnumIterator + Display + PartialEq + Clone>(
    ui: &mut Ui,
    label: impl Into<WidgetText>,
    items: impl IntoIterator<Item = Mut<'a, T>>,
) -> Response {
    edit_row(ui, label, true, |ui| combobox_enum_multi_edit(ui, None, items))
}

pub fn checkbox_edit_row<'a>(
    ui: &mut Ui,
    label: impl Into<WidgetText>,
    items: impl IntoIterator<Item = Mut<'a, bool>>,
) -> Response {
    edit_row(ui, label, false, |ui| checkbox_multi_edit(ui, items))
}

pub fn edit_row<R>(
    ui: &mut Ui,
    label: impl Into<WidgetText>,
    justified: bool,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> R {
    ui.horizontal(|ui| {
        ui.columns(2, |ui| {
            ui[0].with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add(egui::Label::new(label).truncate());
            });
            if justified {
                ui[1].centered_and_justified(add_contents)
            } else {
                ui[1].with_layout(Layout::left_to_right(egui::Align::Center), add_contents)
            }
            .inner
        })
    })
    .inner
}

use crate::ui::util::LinkSelectBtnType::*;

#[derive(SystemParam)]
pub struct RouteEditRowParam<'w, 's> {
    q_route_link: Query<'w, 's, &'static RouteLink>,
    q_route_start: Query<'w, 's, Entity, With<RouteSettings>>,
    path_groups: Option<Res<'w, EntityPathGroups<RoutePoint>>>,
    q_visibility: Query<'w, 's, &'static mut Visibility>,
    commands: Commands<'w, 's>,
}
impl RouteEditRowParam<'_, '_> {
    fn show(&mut self, ui: &mut Ui, items: impl IntoIterator<Item = Entity>) {
        let items: Vec<_> = items.into_iter().collect();
        if items.is_empty() {
            return;
        }
        let Some(ref path_groups) = self.path_groups else {
            return;
        };

        let mut route_starts = Vec::new();
        for e in items.iter() {
            let route_e = self.q_route_link.get(*e).ok().map(|x| **x);
            route_starts.push(route_e.and_then(|x| self.q_route_start.get(x).ok()));
        }
        let first_e = route_starts.first().copied().flatten();

        let route_btn_type = if route_starts.iter().all(|x| x.is_none()) {
            NoLink
        } else {
            let mut route_visibilities = Vec::new();
            for route_start in route_starts.iter() {
                route_visibilities.push(
                    route_start
                        .and_then(|x| self.q_visibility.get(x).ok())
                        .map(|x| x == Visibility::Visible),
                );
            }

            // looks weird but basically means 'go through all the route visibilities which exist (skipping the ones that don't) and ask if all of them are visible or not'
            let all_visible = route_visibilities.iter().filter_map(|x| *x).all(|x| x);

            // if all are the same (and aren't none because we already checked that) this means we have selected only points linking to a single route
            if route_starts.iter().all(|x| *x == first_e) {
                let first_e = first_e.unwrap();
                let mut index = None;
                for (i, path_group) in path_groups.iter().enumerate() {
                    if path_group.path[0] == first_e {
                        index = Some(i);
                    }
                }
                let Some(index) = index else {
                    warn!("There is something wrong with the route path groups, because a route start entity wasn't found as the first point in the route path groups!");
                    return;
                };

                Single {
                    index,
                    visible: all_visible,
                }
            } else {
                let mut indexes = Vec::new();
                'outer: for route_start_e in route_starts.iter() {
                    if let Some(route_start_e) = *route_start_e {
                        for (i, path_group) in path_groups.iter().enumerate() {
                            if path_group.path[0] == route_start_e {
                                indexes.push(Some(i));
                                continue 'outer;
                            }
                        }
                        warn!("There is something wrong with the route path groups, because a route start entity wasn't found as the first point in the route path groups!");
                    } else {
                        indexes.push(None);
                    }
                }
                Multi {
                    indexes,
                    visible: all_visible,
                }
            }
        };

        let route_res = edit_row(ui, "Route", false, |ui| link_select_btn(ui, &route_btn_type, "Route"));

        if route_res.cross_pressed {
            for e in items.iter() {
                self.commands.entity(*e).remove::<RouteLink>();
            }
        }

        if route_res.view_pressed {
            match route_btn_type {
                Single { index, visible } => {
                    let Some(path) = path_groups.get(index) else {
                        warn!("Something got fucked because the index of the route isn't found in the path groups");
                        return;
                    };
                    for e in path.path.iter() {
                        let Ok(mut visibility) = self.q_visibility.get_mut(*e) else {
                            continue;
                        };
                        *visibility = if visible {
                            Visibility::Hidden
                        } else {
                            Visibility::Visible
                        };
                    }
                }
                Multi { indexes, visible } => {
                    // go through all the routes which are linked
                    for index in indexes.iter().filter_map(|x| *x) {
                        let Some(path) = path_groups.get(index) else {
                            warn!("Something got fucked because the index of the route isn't found in the path groups");
                            return;
                        };
                        for e in path.path.iter() {
                            let Ok(mut visibility) = self.q_visibility.get_mut(*e) else {
                                continue;
                            };
                            *visibility = if visible {
                                Visibility::Hidden
                            } else {
                                Visibility::Visible
                            };
                        }
                    }
                }
                _ => (),
            }
        }

        if route_res.eyedropper_pressed {
            self.commands.insert_resource(LinkSelectMode::<RoutePoint>::new(items));
        }
    }
}
