// Included in document::tests to share the real importer and disposable fixtures.
fn editor_view(world: &mut World) -> Vec<(Entity, bool, Option<Visibility>, Option<bool>)> {
    use crate::viewer::edit::select::Selected;
    let mut view: Vec<_> = world
        .query_filtered::<(Entity, Has<Selected>, Option<&Visibility>, Option<&AreaPoint>), With<KmpSelectablePoint>>()
        .iter(world)
        .map(|(e, selected, visibility, area)| (e, selected, visibility.copied(), area.map(|a| a.show_area)))
        .collect();
    view.sort_by_key(|v| v.0);
    view
}

#[test]
fn both_save_modes_and_save_as_preserve_view_entities_history_and_dirty_baseline() {
    use super::super::history::{self, DocumentHistory};
    use crate::viewer::edit::select::Selected;
    for patch in [false, true] {
        for save_as in [false, true] {
            let dir = TestDir::new();
            let path = dir.0.join("course.kmp");
            let destination = if save_as { dir.0.join("copy.kmp") } else { path.clone() };
            let mut app = headless_app();
            let mut kmp = fixture();
            kmp.area = Section::new(vec![Area { kind: 9, ..default() }]);
            load(&mut app, &path, &kmp);
            let world = app.world_mut();
            world.resource_mut::<AppSettings>().patch_saving = patch;
            let start = ordered::<StartPoint>(world)[0];
            let area = ordered::<AreaPoint>(world)[0];
            let object = ordered::<Object>(world)[0];
            world.entity_mut(start).insert((Selected, Visibility::Visible));
            world.entity_mut(area).insert(Visibility::Visible);
            world.get_mut::<AreaPoint>(area).unwrap().show_area = true;
            world.entity_mut(object).insert(Visibility::Hidden);
            world.resource_mut::<TrackInfo>().track_type = TrackType::Battle;
            assert!(!has_unsaved_changes(world).unwrap());
            world.get_mut::<Transform>(start).unwrap().translation.x = 100.;
            // Normal-save coverage includes a structural edit as well.
            if !patch {
                Spawner::<StartPoint>::builder()
                    .pos(Vec3::X * 200.)
                    .build()
                    .spawn(world);
            }
            history::checkpoint(world);
            world.resource_mut::<TrackInfo>().lap_count = 9;
            history::checkpoint(world);
            history::undo(world, false); // Save with BOTH undo and redo available.
            let view = editor_view(world);
            assert!(view.iter().any(|(_, selected, _, _)| *selected));
            assert!(view
                .iter()
                .any(|(_, _, visibility, _)| *visibility == Some(Visibility::Hidden)));
            assert!(view.iter().any(|(_, _, _, show)| *show == Some(true)));
            let entities = history::identities(world);
            assert!(world.resource::<DocumentHistory>().can_undo());
            assert!(world.resource::<DocumentHistory>().can_redo());
            save(world, save_as.then(|| destination.clone())).unwrap();
            assert_eq!(
                editor_view(world),
                view,
                "save changed view; patch={patch}, Save As={save_as}"
            );
            assert_eq!(history::identities(world), entities);
            assert!(matches!(world.resource::<TrackInfo>().track_type, TrackType::Battle));
            assert!(world.resource::<DocumentHistory>().can_undo());
            assert!(world.resource::<DocumentHistory>().can_redo());
            assert!(!has_unsaved_changes(world).unwrap());
            let bytes = fs::read(&destination).unwrap();
            let versions = world.resource::<LoadedKmp>().disk_versions.clone();
            save(world, None).unwrap();
            assert_eq!(editor_view(world), view);
            assert_eq!(fs::read(&destination).unwrap(), bytes);
            history::undo(world, false);
            assert!(has_unsaved_changes(world).unwrap_or(true));
            assert_eq!(world.resource::<KmpFilePath>().0, destination);
            assert_eq!(world.resource::<LoadedKmp>().disk_versions, versions);
            history::undo(world, true); // Return to the newly saved baseline.
            assert!(!has_unsaved_changes(world).unwrap());
            history::undo(world, true); // Pre-save redo branch must still exist.
            assert_eq!(world.resource::<TrackInfo>().lap_count, 9);
            assert!(has_unsaved_changes(world).unwrap());
            history::undo(world, false);
            assert!(!has_unsaved_changes(world).unwrap());
            assert_eq!(fs::read(&destination).unwrap(), bytes);
        }
    }
}

#[test]
fn original_layout_patch_respects_bound_target_after_temporary_reorder() {
    use super::super::history;
    let dir = TestDir::new();
    let path = dir.0.join("bound-patch.kmp");
    let mut app = headless_app();
    let kmp = KmpFile {
        stgi: Section::new(vec![Stgi::default()]),
        came: Section::new(vec![
            Came {
                next_index: 1,
                ..default()
            },
            Came {
                next_index: 255,
                ..default()
            },
            Came {
                next_index: 255,
                ..default()
            },
        ]),
        ..default()
    };
    load(&mut app, &path, &kmp);
    let world = app.world_mut();
    let cameras = ordered::<KmpCamera>(world);
    world.entity_mut(cameras[2]).insert(OrderId(0));
    world.entity_mut(cameras[0]).insert(OrderId(1));
    world.entity_mut(cameras[1]).insert(OrderId(2));
    world.get_mut::<KmpCamera>(cameras[0]).unwrap().next_index = 0;
    history::checkpoint(world); // Bind edited row zero to camera C.
    for (i, &e) in cameras.iter().enumerate() {
        world.entity_mut(e).insert(OrderId(i as u32));
    }
    history::checkpoint(world); // Structure now matches the original source.
    save(world, None).unwrap();
    let parsed = KmpFile::read(&mut Cursor::new(fs::read(&path).unwrap())).unwrap();
    assert_eq!(
        parsed.came[0].next_index, 2,
        "patch must follow C, not reinterpret zero as A"
    );
    assert!(!has_unsaved_changes(world).unwrap());
}

#[test]
fn original_layout_patch_maps_edited_enemy_row_over_source_hole() {
    use super::super::history;
    let dir = TestDir::new();
    let path = dir.0.join("holes.kmp");
    let mut app = headless_app();
    let mut kmp = fixture();
    kmp.enph = Section::new(vec![PathGroup::new(1, 1, [255; 6], [255; 6], 0)]);
    kmp.area = Section::new(vec![Area {
        kind: 4,
        enpt_id: 0,
        ..default()
    }]);
    let original = load(&mut app, &path, &kmp);
    let world = app.world_mut();
    save(world, None).unwrap(); // Unresolved imported row zero is preserved.
    assert_eq!(fs::read(&path).unwrap(), original);
    let area = ordered::<AreaPoint>(world)[0];
    world.get_mut::<AreaPoint>(area).unwrap().kind = AreaKind::ForceRecalc { enemy_path_id: 255 };
    history::checkpoint(world);
    world.get_mut::<AreaPoint>(area).unwrap().kind = AreaKind::ForceRecalc { enemy_path_id: 0 };
    // Dirty inspection must bind the current edit even before a history frame.
    assert!(has_unsaved_changes(world).unwrap());
    save(world, None).unwrap();
    let saved = fs::read(&path).unwrap();
    let parsed = KmpFile::read(&mut Cursor::new(&saved)).unwrap();
    assert_eq!(parsed.area[0].enpt_id, 1);
    assert!(!has_unsaved_changes(world).unwrap());
    world.get_mut::<AreaPoint>(area).unwrap().kind = AreaKind::ForceRecalc { enemy_path_id: 12 };
    assert!(save(world, None).is_err());
    assert_eq!(fs::read(&path).unwrap(), saved);
}

#[test]
fn kcl_only_load_preserves_kmp_view_entities_and_history() {
    use super::super::history::{self, DocumentHistory};
    use crate::{
        ui::update_ui::KclFileSelected,
        viewer::kcl_model::{spawn_model, KCLModelSection},
    };
    let dir = TestDir::new();
    let path = dir.0.join("course.kmp");
    let kcl_path = dir.0.join("course.kcl");
    // One synthetic, non-degenerate triangle; no external course data is needed.
    let mut bytes = Vec::new();
    for offset in [16_u32, 28, 60, 92] {
        bytes.extend_from_slice(&offset.to_be_bytes());
    }
    for value in [0_f32, 0., 0., 0., 1., 0., -1., 0., 0., 0., 0., 1., -1., 0., -1.] {
        bytes.extend_from_slice(&value.to_be_bytes());
    }
    bytes.extend_from_slice(&1_f32.to_be_bytes());
    for value in [0_u16, 0, 1, 2, 3, 0] {
        bytes.extend_from_slice(&value.to_be_bytes());
    }
    fs::write(&kcl_path, bytes).unwrap();
    let mut app = headless_app();
    app.add_message::<KclFileSelected>();
    load(&mut app, &path, &fixture());
    let world = app.world_mut();
    world.resource_mut::<TrackInfo>().lap_count = 4;
    history::checkpoint(world);
    world.resource_mut::<TrackInfo>().lap_count = 5;
    history::checkpoint(world);
    history::undo(world, false);
    let view = editor_view(world);
    world.write_message(KclFileSelected(kcl_path));
    world.run_system_once(spawn_model).unwrap();
    assert_eq!(world.query::<&KCLModelSection>().iter(world).count(), 1);
    assert_eq!(editor_view(world), view);
    assert_eq!(world.resource::<KmpFilePath>().0, path);
    assert!(world.resource::<DocumentHistory>().can_undo());
    assert!(world.resource::<DocumentHistory>().can_redo());
    history::undo(world, true);
    assert_eq!(world.resource::<TrackInfo>().lap_count, 5);
}

#[test]
fn failed_open_preserves_history_successful_open_clears_it() {
    use super::super::history::{self, DocumentHistory};
    let dir = TestDir::new();
    let path = dir.0.join("course.kmp");
    let mut app = headless_app();
    load(&mut app, &path, &fixture());
    let world = app.world_mut();
    world.resource_mut::<TrackInfo>().lap_count = 4;
    history::checkpoint(world);
    world.resource_mut::<TrackInfo>().lap_count = 5;
    history::checkpoint(world);
    history::undo(world, false);
    let view = editor_view(world);
    world.write_message(KmpFileSelected(dir.0.join("missing.kmp")));
    assert!(open_kmp(world).is_err());
    assert_eq!(editor_view(world), view);
    assert!(world.resource::<DocumentHistory>().can_undo());
    assert!(world.resource::<DocumentHistory>().can_redo());
    history::undo(world, true);
    assert_eq!(world.resource::<TrackInfo>().lap_count, 5);
    world.write_message(KmpFileSelected(path.clone()));
    open_kmp(world).unwrap();
    assert!(!world.resource::<DocumentHistory>().can_undo());
    assert!(!world.resource::<DocumentHistory>().can_redo());
    assert!(!has_unsaved_changes(world).unwrap());
}
