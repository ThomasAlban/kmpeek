use super::super::{
    checkpoints::{checkpoint_plugin, CheckpointLine, CheckpointPlane},
    meshes_materials::setup_kmp_meshes_materials,
    ordering::{ordering_plugin, NextOrderID},
    path::{path_plugin, EntityPathGroups, KmpPathNodeLink},
    routes::{routes_plugin, update_routes},
};
use super::*;
use crate::{
    ui::{settings::AppSettings, update_ui::KmpFileSelected},
    util::kmp_file::{KmpFile, Section, Stgi},
};
use bevy::ecs::system::RunSystemOnce;
use std::{
    io::Cursor,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

struct Fixture {
    app: App,
    path: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "kmpeek-restore-{}-{}.kmp",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let mut bytes = Cursor::new(Vec::new());
        KmpFile {
            stgi: Section::new(vec![Stgi::default()]),
            ..default()
        }
        .write(&mut bytes)
        .unwrap();
        std::fs::write(&path, bytes.into_inner()).unwrap();
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<AppSettings>()
            .add_message::<KmpFileSelected>()
            .add_plugins((checkpoint_plugin, path_plugin, ordering_plugin, routes_plugin));
        app.world_mut().run_system_once(setup_kmp_meshes_materials).unwrap();
        let mut fixture = Self { app, path };
        fixture.open();
        fixture
    }
    fn open(&mut self) {
        self.app.world_mut().write_message(KmpFileSelected(self.path.clone()));
        super::super::open_kmp(self.app.world_mut()).unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn assert_derived(world: &mut World) {
    fn groups<T: Component>(world: &mut World) {
        let current: std::collections::HashSet<_> = world
            .query_filtered::<Entity, (With<T>, With<KmpPathNode>)>()
            .iter(world)
            .collect();
        let groups = world.resource::<EntityPathGroups<T>>();
        let members: Vec<_> = groups.iter().flat_map(|g| g.path.iter().copied()).collect();
        assert_eq!(members.len(), current.len());
        assert_eq!(members.into_iter().collect::<std::collections::HashSet<_>>(), current);
        for group in groups.iter() {
            assert!(group
                .next_paths
                .iter()
                .chain(&group.prev_paths)
                .all(|&i| i < groups.len()));
        }
    }
    groups::<EnemyPathPoint>(world);
    groups::<ItemPathPoint>(world);
    groups::<Checkpoint>(world);
    groups::<RoutePoint>(world);
    for link in world.query::<&KmpPathNodeLink>().iter(world) {
        assert!(world.get::<KmpPathNode>(link.prev_node).is_some());
        assert!(world.get::<KmpPathNode>(link.next_node).is_some());
    }
    for (e, owners) in world.query::<(Entity, &RouteLinkedEntities)>().iter(world) {
        for &owner in owners.iter() {
            assert_eq!(world.get::<RouteLink>(owner).unwrap().0, e);
        }
    }
    for (owner, link) in world.query::<(Entity, &RouteLink)>().iter(world) {
        assert!(world.get::<RouteLinkedEntities>(link.0).unwrap().contains(&owner));
    }
    for line in world.query::<&CheckpointLine>().iter(world) {
        assert!(world.get::<CheckpointLeft>(line.left).is_some());
        assert!(world.get::<CheckpointRight>(line.right).is_some());
        assert!(world.get_entity(line.arrow).is_ok());
    }
    for plane in world.query::<&CheckpointPlane>().iter(world) {
        assert!(world.get::<CheckpointLeft>(plane.left).is_some());
        assert!(world.get::<CheckpointRight>(plane.right).is_some());
    }
}

#[test]
fn paired_checkpoint_delete_undo_redo_rebuilds_caches_and_counters() {
    let mut fixture = Fixture::new();
    let world = fixture.app.world_mut();
    let respawn = Spawner::<RespawnPoint>::builder().build().spawn(world);
    let (left, right) = checkpoint_spawner()
        .cp(Checkpoint::default())
        .order_id(10)
        .pos((Vec2::ZERO, Vec2::X * 100.))
        .world(world)
        .call();
    let (next_left, next_right) = checkpoint_spawner()
        .cp(Checkpoint {
            kind: CheckpointKind::Key(7),
        })
        .order_id(40)
        .pos((Vec2::Y * 100., Vec2::splat(100.)))
        .world(world)
        .call();
    for e in [left, next_left] {
        world.entity_mut(e).insert(CheckpointRespawnLink(respawn));
    }
    assert!(KmpPathNode::link_nodes(left, next_left, world));
    assert!(KmpPathNode::link_nodes(right, next_right, world));
    world.entity_mut(right).insert(Selected);
    checkpoint(world);
    let original = DocumentSnapshot::capture(world);
    let pair = world.get::<CheckpointLeft>(left).unwrap().clone();
    world.despawn(right); // Real reciprocal removal observers delete both halves.
    for e in [left, right, pair.line, pair.plane, pair.arrow] {
        assert!(world.get_entity(e).is_err());
    }
    checkpoint(world);
    let deleted = DocumentSnapshot::capture(world);
    undo(world, false);
    assert!(original.equivalent(&DocumentSnapshot::capture(world)));
    let live = identities(world);
    assert_eq!(world.get::<CheckpointLeft>(live[&left]).unwrap().right, live[&right]);
    assert_eq!(world.get::<CheckpointRight>(live[&right]).unwrap().left, live[&left]);
    assert_eq!(
        world.get::<CheckpointRespawnLink>(live[&left]).unwrap().0,
        live[&respawn]
    );
    assert!(world.get::<Selected>(live[&right]).is_some());
    assert_derived(world);
    fixture.app.update();
    let world = fixture.app.world_mut();
    assert!(
        original.equivalent(&DocumentSnapshot::capture(world)),
        "maintenance changed restored checkpoint data"
    );
    undo(world, true);
    assert!(deleted.equivalent(&DocumentSnapshot::capture(world)));
    assert_derived(world);
    undo(world, false);
    assert_eq!(world.resource::<NextOrderID<Checkpoint>>().get(), 41);
    // Counter queries consume IDs, so put it back and exercise the actual spawner.
    world.resource::<NextOrderID<Checkpoint>>().set(41u32);
    let (created, _) = checkpoint_spawner().cp(Checkpoint::default()).world(world).call();
    assert_eq!(world.get::<OrderId>(created).unwrap().0, 41);
    checkpoint(world);
    undo(world, false);
    assert!(original.equivalent(&DocumentSnapshot::capture(world)));
    undo(world, true);
    assert!(identities(world).contains_key(&created));
    assert_eq!(world.resource::<NextOrderID<Checkpoint>>().get(), 42);
    assert_derived(world);
}

#[test]
fn route_start_delete_undo_redo_preserves_settings_owners_and_order() {
    let mut fixture = Fixture::new();
    let world = fixture.app.world_mut();
    let head = Spawner::<RoutePoint>::builder()
        .max(1)
        .order_id(20)
        .build()
        .spawn(world);
    let tail = Spawner::<RoutePoint>::builder()
        .max(1)
        .order_id(80)
        .prev_nodes([head].into_iter().collect::<bevy::ecs::entity::EntityHashSet>())
        .build()
        .spawn(world);
    let settings = RouteSettings {
        smooth_motion: true,
        loop_style: RouteLoopStyle::Mirror,
    };
    world.entity_mut(head).insert(settings.clone());
    let owner = Spawner::<Object>::builder()
        .route(head)
        .order_id(30)
        .build()
        .spawn(world);
    world.run_system_once(update_routes).unwrap();
    checkpoint(world);
    let original = DocumentSnapshot::capture(world);
    world.despawn(head);
    world.run_system_once(update_routes).unwrap();
    assert_eq!(world.get::<RouteSettings>(tail), Some(&settings));
    assert_eq!(world.get::<RouteLink>(owner).unwrap().0, tail);
    checkpoint(world);
    let deleted = DocumentSnapshot::capture(world);
    undo(world, false);
    assert!(original.equivalent(&DocumentSnapshot::capture(world)));
    let live = identities(world);
    assert_eq!(world.get::<RouteSettings>(live[&head]), Some(&settings));
    assert!(world.get::<RouteSettings>(live[&tail]).is_none());
    assert_eq!(world.get::<RouteLink>(live[&owner]).unwrap().0, live[&head]);
    assert_derived(world);
    fixture.app.update();
    let world = fixture.app.world_mut();
    assert!(
        original.equivalent(&DocumentSnapshot::capture(world)),
        "maintenance changed restored route data"
    );
    undo(world, true);
    assert!(deleted.equivalent(&DocumentSnapshot::capture(world)));
    let live = identities(world);
    assert_eq!(world.get::<RouteSettings>(live[&tail]), Some(&settings));
    assert_eq!(world.get::<RouteLink>(live[&owner]).unwrap().0, live[&tail]);
    assert_derived(world);
    assert_eq!(world.resource::<NextOrderID<Object>>().get(), 31);
    let created = Spawner::<RoutePoint>::builder().max(1).build().spawn(world);
    assert_eq!(world.get::<OrderId>(created).unwrap().0, 81);
}

#[test]
fn incomplete_checkpoints_and_dangling_links_roundtrip_without_repair() {
    let mut fixture = Fixture::new();
    let world = fixture.app.world_mut();
    let route = Spawner::<RoutePoint>::builder().max(1).build().spawn(world);
    let object = Spawner::<Object>::builder().route(route).build().spawn(world);
    let respawn = Spawner::<RespawnPoint>::builder().build().spawn(world);
    let left = world
        .spawn((
            Checkpoint::default(),
            CheckpointMarker,
            KmpSelectablePoint,
            Transform::IDENTITY,
            OrderId(17),
            CheckpointRespawnLink(respawn),
        ))
        .id();
    let right = world
        .spawn((
            CheckpointRight::default(),
            CheckpointMarker,
            KmpSelectablePoint,
            Transform::IDENTITY,
        ))
        .id();
    checkpoint(world);
    // First restore reallocates the respawn. Deleting that allocation must keep
    // its logical identity in the dangling link captured by the next snapshot.
    world.get_mut::<Object>(object).unwrap().object_id = 99;
    checkpoint(world);
    undo(world, false);
    let live = identities(world);
    let dead = live[&respawn];
    world.despawn(dead);
    world.get_mut::<RouteLink>(live[&object]).unwrap().0 = dead;
    checkpoint(world);
    let malformed = DocumentSnapshot::capture(world);
    undo(world, false);
    undo(world, true);
    assert!(malformed.equivalent(&DocumentSnapshot::capture(world)));
    let live = identities(world);
    assert!(world.get::<CheckpointLeft>(live[&left]).is_none());
    assert_eq!(
        world.get::<CheckpointRight>(live[&right]).unwrap().left,
        Entity::PLACEHOLDER
    );
    assert!(world
        .get_entity(world.get::<RouteLink>(live[&object]).unwrap().0)
        .is_err());
    assert!(world
        .get_entity(world.get::<CheckpointRespawnLink>(live[&left]).unwrap().0)
        .is_err());
    assert_eq!(world.query::<&CheckpointLine>().iter(world).count(), 0);
    fixture.app.update();
    assert!(malformed.equivalent(&DocumentSnapshot::capture(fixture.app.world_mut())));
}

#[test]
fn grouping_mouse_and_text_sessions_commit_once_and_pending_undo_works() {
    for text in [false, true] {
        let mut fixture = Fixture::new();
        let world = fixture.app.world_mut();
        let original = world.resource::<TrackInfo>().lap_count;
        for value in [4, 5, 6] {
            world.resource_mut::<TrackInfo>().lap_count = value;
            record_with_input(
                world,
                GroupInput {
                    text: text.then(|| bevy_egui::egui::Id::new("laps")),
                    pointer: !text,
                    finish: false,
                },
            );
            let history = world.resource::<DocumentHistory>();
            assert!(history.undo.is_empty());
            assert!(history.can_undo(), "menu must see pending first action");
        }
        // Undo without releasing the mouse or leaving the text field.
        undo(world, false);
        assert_eq!(world.resource::<TrackInfo>().lap_count, original);
        assert!(!world.resource::<DocumentHistory>().can_undo());
        undo(world, true);
        assert_eq!(world.resource::<TrackInfo>().lap_count, 6);
        assert_eq!(world.resource::<DocumentHistory>().undo.len(), 1);
    }
}

#[test]
fn grouping_focus_change_enter_release_and_redo_invalidation() {
    let mut fixture = Fixture::new();
    let world = fixture.app.world_mut();
    let field = |name| GroupInput {
        text: Some(bevy_egui::egui::Id::new(name)),
        ..default()
    };
    world.resource_mut::<TrackInfo>().lap_count = 4;
    record_with_input(world, field("laps"));
    world.resource_mut::<TrackInfo>().lap_count = 5;
    record_with_input(world, field("laps"));
    world.resource_mut::<TrackInfo>().speed_mod = 2.;
    record_with_input(world, field("speed"));
    assert_eq!(world.resource::<DocumentHistory>().undo.len(), 1);
    world.resource_mut::<TrackInfo>().speed_mod = 3.;
    record_with_input(
        world,
        GroupInput {
            finish: true,
            ..field("speed")
        },
    );
    assert_eq!(world.resource::<DocumentHistory>().undo.len(), 2);
    record_with_input(world, GroupInput::default());
    assert_eq!(world.resource::<DocumentHistory>().undo.len(), 2);
    undo(world, false);
    assert!(world.resource::<DocumentHistory>().can_redo());
    world.resource_mut::<TrackInfo>().lap_count = 9;
    record_with_input(
        world,
        GroupInput {
            pointer: true,
            ..default()
        },
    );
    assert!(!world.resource::<DocumentHistory>().can_redo());
    world.resource_mut::<TrackInfo>().lap_count = 10;
    record_with_input(world, GroupInput::default()); // include final release value
    assert_eq!(world.resource::<DocumentHistory>().undo.len(), 2);
    undo(world, false);
    assert_eq!(world.resource::<TrackInfo>().lap_count, 5);
}

// Exercise real egui focus traversal and native shortcuts, with record_frame
// and the production shortcut router rather than only synthetic GroupInput.
fn text_frame(world: &mut World, ctx: &bevy_egui::egui::Context, events: Vec<bevy_egui::egui::Event>) {
    use bevy_egui::egui::{self, TextEdit};
    ctx.begin_pass(egui::RawInput { events, ..default() });
    crate::ui::keybinds::history_shortcuts(ctx, world);
    let mut ui = egui::Ui::new(ctx.clone(), "test-root".into(), egui::UiBuilder::new());
    let mut a = world.resource::<TrackInfo>().lap_count.to_string();
    let mut b = world.resource::<TrackInfo>().speed_mod.to_string();
    ui.add(TextEdit::singleline(&mut a).id(egui::Id::new("a")));
    ui.add(TextEdit::singleline(&mut b).id(egui::Id::new("b")));
    if let Ok(value) = a.parse() {
        world.resource_mut::<TrackInfo>().lap_count = value;
    }
    if let Ok(value) = b.parse() {
        world.resource_mut::<TrackInfo>().speed_mod = value;
    }
    ctx.end_pass().textures_delta.clear();
    record_frame(world);
}

fn key_event(key: bevy_egui::egui::Key, command: bool, shift: bool) -> bevy_egui::egui::Event {
    use bevy_egui::egui::{Event, Modifiers};
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers {
            ctrl: command,
            command,
            shift,
            ..default()
        },
    }
}

#[test]
fn tab_and_shift_tab_release_native_ownership_and_document_undo_restores_edit() {
    use bevy_egui::{
        egui::{Event, Id, Key},
        EguiContext, PrimaryEguiContext,
    };
    for shift in [false, true] {
        let mut fixture = Fixture::new();
        let world = fixture.app.world_mut();
        let mut context = EguiContext::default();
        let ctx = context.get_mut().clone();
        world.spawn((context, PrimaryEguiContext));
        let field = Id::new(if shift { "b" } else { "a" });
        let original = DocumentSnapshot::capture(world);
        text_frame(world, &ctx, vec![]);
        ctx.memory_mut(|m| m.request_focus(field));
        text_frame(
            world,
            &ctx,
            vec![key_event(Key::A, true, false), Event::Text("7".into())],
        );
        assert!(world.resource::<DocumentHistory>().owns_text_undo(Some(field)));
        text_frame(world, &ctx, vec![key_event(Key::Tab, false, shift)]);
        assert_eq!(world.resource::<DocumentHistory>().undo.len(), 1);
        assert!(!world.resource::<DocumentHistory>().text_edited);
        // Shift+Tab applies its destination at the beginning of this pass.
        text_frame(world, &ctx, vec![key_event(Key::Z, true, false)]);
        assert!(original.equivalent(&DocumentSnapshot::capture(world)));
        assert!(
            !ctx.input(|i| i.key_pressed(Key::Z)),
            "document shortcut must be consumed"
        );
        text_frame(world, &ctx, vec![]);
        assert!(
            original.equivalent(&DocumentSnapshot::capture(world)),
            "no stale field writeback"
        );
    }
}

#[test]
fn native_undo_redo_stays_in_edited_field_then_tab_commits_separate_groups() {
    use bevy_egui::{
        egui::{Event, Id, Key},
        EguiContext, PrimaryEguiContext,
    };
    let mut fixture = Fixture::new();
    let world = fixture.app.world_mut();
    let mut context = EguiContext::default();
    let ctx = context.get_mut().clone();
    world.spawn((context, PrimaryEguiContext));
    let original = DocumentSnapshot::capture(world);
    text_frame(world, &ctx, vec![]);
    ctx.memory_mut(|m| m.request_focus(Id::new("a")));
    text_frame(
        world,
        &ctx,
        vec![key_event(Key::A, true, false), Event::Text("7".into())],
    );
    text_frame(world, &ctx, vec![key_event(Key::Tab, false, false)]);
    let after_a = DocumentSnapshot::capture(world);
    text_frame(
        world,
        &ctx,
        vec![key_event(Key::A, true, false), Event::Text("9".into())],
    );
    assert_eq!(world.resource::<TrackInfo>().speed_mod, 9.);
    text_frame(world, &ctx, vec![key_event(Key::Z, true, false)]);
    assert!(after_a.equivalent(&DocumentSnapshot::capture(world)));
    assert!(ctx.input(|i| i.key_pressed(Key::Z)), "native shortcut left intact");
    assert!(world.resource::<DocumentHistory>().owns_text_undo(Some(Id::new("b"))));
    text_frame(world, &ctx, vec![key_event(Key::Z, true, true)]);
    assert_eq!(world.resource::<TrackInfo>().speed_mod, 9.);
    text_frame(world, &ctx, vec![key_event(Key::Tab, false, true)]);
    assert_eq!(world.resource::<DocumentHistory>().undo.len(), 2);
    text_frame(world, &ctx, vec![key_event(Key::Z, true, false)]);
    assert!(after_a.equivalent(&DocumentSnapshot::capture(world)));
    text_frame(world, &ctx, vec![key_event(Key::Z, true, false)]);
    assert!(original.equivalent(&DocumentSnapshot::capture(world)));
}

#[test]
fn tab_boundary_includes_final_frame_value_even_when_focus_already_changed() {
    for forward in [false, true] {
        let mut fixture = Fixture::new();
        let world = fixture.app.world_mut();
        let original = world.resource::<TrackInfo>().lap_count;
        let a = bevy_egui::egui::Id::new("a");
        let b = bevy_egui::egui::Id::new("b");
        world.resource_mut::<TrackInfo>().lap_count = 5;
        record_with_input(
            world,
            GroupInput {
                text: Some(a),
                ..default()
            },
        );
        world.resource_mut::<TrackInfo>().lap_count = 7;
        record_with_input(
            world,
            GroupInput {
                text: Some(if forward { b } else { a }),
                finish: true,
                ..default()
            },
        );
        assert_eq!(world.resource::<DocumentHistory>().undo.len(), 1);
        undo(world, false);
        assert_eq!(world.resource::<TrackInfo>().lap_count, original);
        undo(world, true);
        assert_eq!(world.resource::<TrackInfo>().lap_count, 7);
    }
}

#[test]
fn grouping_noops_nan_and_track_type_are_not_document_actions() {
    let mut fixture = Fixture::new();
    let world = fixture.app.world_mut();
    world.resource_mut::<TrackInfo>().track_type = TrackType::Battle;
    record_with_input(world, GroupInput::default());
    assert!(!world.resource::<DocumentHistory>().can_undo());
    let initial = world.resource::<TrackInfo>().lap_count;
    world.resource_mut::<TrackInfo>().lap_count = 8;
    record_with_input(
        world,
        GroupInput {
            pointer: true,
            ..default()
        },
    );
    world.resource_mut::<TrackInfo>().lap_count = initial;
    record_with_input(world, GroupInput::default());
    assert!(!world.resource::<DocumentHistory>().can_undo());
    world.resource_mut::<TrackInfo>().speed_mod = f32::from_bits(0x7fc00007);
    record_with_input(world, GroupInput::default());
    for _ in 0..3 {
        record_with_input(world, GroupInput::default());
    }
    assert_eq!(world.resource::<DocumentHistory>().undo.len(), 1);
    undo(world, false);
    assert!(matches!(world.resource::<TrackInfo>().track_type, TrackType::Battle));
    undo(world, true);
    assert_eq!(world.resource::<TrackInfo>().speed_mod.to_bits(), 0x7fc00007);
    assert!(matches!(world.resource::<TrackInfo>().track_type, TrackType::Battle));
}

#[test]
fn repeated_undo_redo_keeps_alias_allocations_bounded() {
    let mut fixture = Fixture::new();
    let world = fixture.app.world_mut();
    for _ in 0..16 {
        Spawner::<StartPoint>::builder().build().spawn(world);
    }
    checkpoint(world);
    world.resource_mut::<TrackInfo>().lap_count = 7;
    checkpoint(world);
    let bound = world.resource::<DocumentAliases>().0.capacity();
    // Every operation reallocates every point. Previously this retained 9,600
    // additional aliases even though history had only two committed edits.
    for _ in 0..300 {
        for redo in [false, true] {
            undo(world, redo);
            assert_eq!(world.resource::<DocumentAliases>().0.len(), 16);
            assert!(world.resource::<DocumentAliases>().0.capacity() <= bound);
            assert_eq!(world.resource::<DocumentHistory>().views.len(), 16);
        }
    }
}

#[test]
fn aliases_keep_only_live_allocations_and_dangling_document_targets() {
    let mut world = World::new();
    let mut targets = Vec::new();
    for _ in 0..6 {
        let logical = world.spawn_empty().id();
        world.despawn(logical);
        let allocation = world.spawn(DocumentId(logical)).id();
        targets.push((allocation, logical));
    }
    let owner = world
        .spawn((
            KmpSelectablePoint,
            StartPoint::default(),
            KmpPathNode::default()
                .with_prev([targets[0].0])
                .with_next([targets[1].0]),
            CheckpointLeft {
                right: targets[2].0,
                ..default()
            },
            CheckpointRight {
                left: targets[3].0,
                ..default()
            },
            RouteLink(targets[4].0),
            CheckpointRespawnLink(targets[4].0),
        ))
        .id();
    assign_identities(&mut world);
    for &(allocation, _) in &targets {
        world.despawn(allocation);
    }
    assign_identities(&mut world);
    assert_eq!(world.resource::<DocumentAliases>().0.len(), 6); // owner + five referenced dead targets
    for &(allocation, logical) in &targets[..5] {
        assert_eq!(reference_key(&world, allocation), logical);
    }
    assert!(!world.resource::<DocumentAliases>().0.contains_key(&targets[5].0));
    world
        .entity_mut(owner)
        .remove::<(KmpPathNode, CheckpointLeft, CheckpointRight, RouteLink)>();
    assign_identities(&mut world);
    assert_eq!(world.resource::<DocumentAliases>().0.len(), 2); // respawn link still needs its alias
    assert_eq!(reference_key(&world, targets[4].0), targets[4].1);
    world.entity_mut(owner).remove::<CheckpointRespawnLink>();
    assign_identities(&mut world);
    assert_eq!(world.resource::<DocumentAliases>().0.len(), 1);
}

#[test]
fn view_cache_retains_resurrectable_points_and_prunes_evicted_or_abandoned_ones() {
    let mut fixture = Fixture::new();
    let world = fixture.app.world_mut();
    let point = Spawner::<StartPoint>::builder().visible(false).build().spawn(world);
    checkpoint(world);
    world.despawn(point);
    checkpoint(world);
    assert!(world.resource::<DocumentHistory>().views.contains_key(&point));
    undo(world, false);
    let live = identities(world)[&point];
    assert_eq!(world.get::<Visibility>(live), Some(&Visibility::Hidden));
    undo(world, true);
    for i in 0..=CAPACITY {
        world.resource_mut::<TrackInfo>().lap_count = (i % 2 + 1) as u8;
        checkpoint(world);
    }
    assert_eq!(world.resource::<DocumentHistory>().undo.len(), CAPACITY);
    assert!(world.resource::<DocumentHistory>().views.is_empty());
    let point = Spawner::<StartPoint>::builder().build().spawn(world);
    checkpoint(world);
    undo(world, false); // Only redo can resurrect this new point.
    assert!(world.resource::<DocumentHistory>().views.contains_key(&point));
    world.resource_mut::<TrackInfo>().lap_count = 17;
    record_with_input(
        world,
        GroupInput {
            pointer: true,
            ..default()
        },
    );
    assert!(!world.resource::<DocumentHistory>().can_redo());
    assert!(!world.resource::<DocumentHistory>().views.contains_key(&point));
}

#[test]
fn successful_open_discards_old_identity_aliases() {
    let mut fixture = Fixture::new();
    let world = fixture.app.world_mut();
    let e = Spawner::<StartPoint>::builder().build().spawn(world);
    checkpoint(world);
    world.despawn(e);
    checkpoint(world);
    // Unreferenced dead allocations are now collected without waiting for open.
    assert!(!world.resource::<DocumentAliases>().0.contains_key(&e));
    let live = Spawner::<StartPoint>::builder().build().spawn(world);
    checkpoint(world);
    assert!(world.resource::<DocumentAliases>().0.contains_key(&live));
    fixture.open();
    assert!(fixture.app.world().resource::<DocumentAliases>().0.is_empty());
    assert!(!fixture.app.world().resource::<DocumentHistory>().can_undo());
}
