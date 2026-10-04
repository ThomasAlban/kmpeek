# KMP saving: patch and rebuild modes

## Choosing a mode

**Settings → General → Patch saving (preserve original KMP data)** is disabled by default. Old settings files that lack this property also default to disabled. The choice is persisted with the other application settings.

| Mode | Behavior |
| --- | --- |
| Patch saving ON | Preserve the current patch layout and untouched values. A no-edit save is byte-identical. Structural changes relative to that layout are refused with instructions to switch modes. |
| Patch saving OFF | Rebuild represented editor sections in a deterministic packed layout. Add/delete/reconnect/reorder edits are saved, supported references remapped, and output local IDs renumbered. Unrepresented/unsupported source data may be discarded. |

The data-loss explanation belongs beside the toggle in Settings, not in a permanent top-level banner. **Use Save As on copies when testing.** Both modes share `document::save` and its atomic replacement; rebuild mode never truncates the original file in place.

**Saving does not reload the document.** Normal saves, patch saves, and Save As preserve live entity allocations, selection, visibility, AREA show flags, camera/settings, and undo/redo history. Output IDs can differ from editor row order; generating them must not rewrite live components. Save runs after deferred editor changes and route maintenance, not halfway through a structural operation. An active edit is checkpointed before saving; saving does not erase an existing redo branch when document data is unchanged.

## Import provenance versus the committed baseline

The original source bytes, parsed records, normalized import baseline, and source identities remain fixed for the document lifetime. They explain which raw source row a logical editor point represents, including holes and aliases in grouped sections. Imported numeric references retain this interpretation even after undo reallocates ECS entities.

A successful rebuild establishes a **separate committed layout baseline**: the written bytes, parsed output, and logical structural signature. Subsequent patch saves compare a checked rebuild projection with this baseline and refuse structural changes relative to it. The import provenance is not replaced. Immediately enabling patch saving after a rebuild allows a byte-identical no-edit save without a reload.

Numeric camera-next and AREA camera/enemy references bind to logical document identities. An edited number is interpreted against editor row order when observed, rather than reinterpreted at every save. Rebuild maps the target to its output row; original-layout patch saving maps it back to its source row, including source-index holes. Temporarily reordering points and then restoring source order must not silently retarget a previously edited reference. Untouched unresolved imported references remain raw source data in original-layout patches; explicitly invalid edited indices are refused.

## Patch contract

Before the first rebuild, a successfully loaded KMP with no persistent edits saves as the **exact source bytes**: section order, group/link slots, unused records, empty routes, raw IDs, padding, unknown settings, unedited Euler angles, header extensions, gaps, and trailing data remain intact.

Changes are measured against the editor's post-import baseline, not the original parsed model, because importing can normalize values. Changed scalar fields are merged into original records. Euler triples are replaced together when orientation changes; ITPT flag edits retain unknown bits. CAME's opening-camera index does not overwrite its second metadata byte.

Patch saving refuses structural changes, conflicting edits to aliases of a shared source point, and removal of required checkpoint respawn links. Non-finite source floats survive original-layout no-op saves, but scalar patching that requires their JSON round-trip can fail safely. After a rebuild, the checked rebuild projection also validates current values and required references.

## Rebuild contract

`rebuild.rs` plans complete output order before converting records:

- Every current path entity is covered once; branch, cycle, disconnected and singleton ENPT/ITPT/CKPT graphs are supported.
- Groups are contiguous chains with deterministic links and checked starts/lengths. Long chains split rather than truncate. Original group boundaries, duplicate link slots, asymmetric metadata and `group_link` values are not retained; rebuilt `group_link` is zero.
- POTI is rebuilt from linear route chains and their smooth/loop settings. Splitting, merging and deleting route points is supported. Deleting a route start transfers its settings and references; existing destination settings win an ambiguous merge. Graph cycles must be disconnected and expressed with route loop settings; POTI cannot represent branching graphs.
- Route references use final POTI indices, including links aimed at a nonstart route member. Objects support 16-bit route indices; camera/AREA routes have narrower limits.
- Checkpoint neighbors use final serialized group order, not display OrderIds. Respawns target final JGPT section positions, not stored JGPT IDs.
- Camera next links and AREA camera/enemy-point links follow their logical targets. Deleted optional camera targets become `255`; missing required respawn/enemy targets must be relinked. Explicitly invalid edited indices are errors.
- Both resolvable CAME header camera indices are remapped. The low byte is selection-video metadata, not the replay-camera root; Wii ignores it at runtime. Replay associations reside in AREA camera links.
- JGPT/CNPT/MSPT output IDs are assigned dense section indices. They remain internal bookkeeping, never user-editable controls.
- All 15 section headers, counts, POTI totals, offsets and file length are regenerated. Serialized output is parsed again before disk replacement.

Rebuild removes unrepresented orphan points, empty source routes, additional unrepresented STGI records, source gaps/extensions/tail bytes and unsupported metadata. It does **not** garbage-collect every unlinked visible object or camera: current editor entities are intentional content. Represented raw fields, such as object extended presence and MSPT's unknown word, remain in the editor and output.

Invalid/asymmetric/stale/cross-section edges, malformed checkpoint pairs, non-finite fields, out-of-range indices and unrepresentable counts are errors, not silent truncation. ENPT/ITPT are limited to 255 points; group indices reserve `255`; checkpoint layouts needing non-sentinel neighbor indices beyond the byte range are refused.

## Undo/redo and dirty state

History uses bounded typed document snapshots (100 committed undo steps), not serialized KMP files or whole-World copies. Snapshots retain editable payloads, hidden represented fields, transforms, order, graph/pairing/reference data, and TrackInfo. They can represent intermediate states that cannot currently be saved. Renderer entities and derived caches are regenerated on restore.

Selection, visibility, AREA display flags, camera/settings, and editor-only track type are not document edits. Restored points recover view state through a separate cache; saving does not alter it. Undo/redo may reallocate entities, while logical document identities preserve references. Pointer drags and focused text-field sessions coalesce; focus loss or Enter commits a text session. Pending edits are undoable from the menu. Keyboard shortcuts defer to egui's native text undo while a field owns keyboard input, and document undo is blocked behind dialogs/modals.

Saving advances disk-version tracking and the active destination's persisted-byte baseline, **not** the undo position. Undoing a saved edit makes the document dirty; redoing it back to the persisted state makes it clean. Structural/invalid projections conservatively count as dirty. Changing the save-mode preference alone does not dirty a document. Undo/redo never rewinds disk-conflict baselines or the active Save As path.

Successful KMP open clears history and identity aliases. Failed open and KCL-only loading preserve KMP history. Opening another KMP or requesting close goes through **Save / Don't Save / Cancel**; a failed save does not continue the destructive action. There is no save-triggered internal open that bypasses this guard.

## Editable fields and scope

All represented editor sections participate in both encoders. The edit panel includes AREA moving-road route links and forced-recalculation enemy-point indices, full signed JGPT extra data, object presence, and the STGI speed modifier. Internal IDs, padding/reserved words, and unknown bookkeeping are deliberately not exposed.

STGI speed uses the LS-Mod extension: the last two bytes are the **high word of an IEEE-754 float**, not half precision. Encoded zero means normal (1×) speed; saving truncates the low word. Game-side extension support is required. Track type and visibility controls are editor-only, not invented KMP binary fields.

This is not an arbitrary raw-byte editor: unknown enum encodings, inactive AREA fields and object-specific setting semantics are not all individually modeled. Patch mode preserves untouched unsupported values; rebuild may normalize/drop them as its warning states. Neither mode proves arbitrary edits create a playable course.

### External references cannot be remapped by a KMP-only save

KCL cannon triggers refer to **CNPT array indices**, not stored cannon IDs. If cannons are reordered, inserted before existing targets, or deleted, update KCL triggers separately. Rewriting KMP IDs alone cannot repair those references. Opaque object-specific settings may also have external meanings; the writer must not guess every number is an index.

## File safety

Both encoders finish before disk I/O. Saving creates an exclusive same-directory temporary file, writes and syncs it, preserves permissions, and renames it over the destination. Failures before replacement leave the original intact. Save As changes the active path only after the write succeeds. No post-save reload is needed.

Opened and previously saved destinations are checked against their last known bytes before temporary writing and again before replacement. External edits cause refusal. This is not an OS lock: another writer can race the final check. File permissions are copied, not all ACLs/xattrs. The directory is not fsynced, so full power-loss durability is not claimed. Windows and network-filesystem behavior require platform testing.

## Verification

```sh
cargo fmt --all -- --check
cargo check --locked
cargo test --locked
cargo test --locked --release
```

The suite covers binary/headless-editor roundtrips, original metadata, mode switching, rebuild→patch without reload, source-index holes, reference bindings, structural undo/redo, route settings/owners, checkpoint deletion cascades, order counters, grouping, invalid writes and disk conflicts. Explicit save tests compare entity IDs, selection, visibility and AREA display flags across normal/patch Save and Save As, with both undo and redo available; they check dirty state and the active path after history navigation. Graph tests exhaust all 512 directed three-node graphs, with and without an anchored start. Synthetic fixtures do not replace stock race/battle or console testing.

### Manual acceptance (copies only)

1. Record platform, build mode, commit and hashes for known-working race and battle course copies.
2. With patch saving ON, load KMP/KCL, wait several frames, change selection/view/visibility, then Save and Save As without document edits. Bytes/hashes must match and view state must remain unchanged.
3. Edit representative fields; inspect intended fields and unrelated bytes. Test camera/AREA and route/respawn references, raw fields, and the speed extension separately.
4. Add/delete/reconnect/reorder points. Patch mode must refuse structural changes. Turn patch mode OFF, verify the Settings warning, and Save As. Selection, hidden points, AREA show flags, camera, and history must remain unchanged.
5. Inspect rebuilt output with an independent KMP tool. Test branches, cycles, disconnected paths, route split/merge/start deletion, checkpoints and camera reindexing. Output indices may differ from editor rows.
6. Enable patch saving after rebuild: an immediate save must be byte-identical, and subsequent patches must hit the correct records. Undo across the save, verify dirty state, redo to the saved state, and verify clean state and the unchanged Save As path. Test a pre-save redo branch too.
7. Exercise typed numeric sessions, Enter/focus loss, viewport drags, native text undo, and undo while holding a drag. Interrupted AREA/gizmo gestures must not restart until release.
8. Test missing targets, invalid graph links, width overflows, unwritable destinations and external disk edits. Failed saves must not truncate files; failed KMP opens and KCL-only loads must retain history.
9. Validate copies in Dolphin/Wii, including start, CPUs/items, laps/checkpoints, respawns, cameras, objects and cannon triggers. Reconcile external KCL references before testing reordered cannons. Repeat debug/release and supported platforms.

### Format references

- [KMP fields and CAME metadata](https://mkwiiki.org/wiki/KMP_(File_Format))
- [Wiimm CAME parameters](https://szs.wiimm.de/info/kmp-syntax.html#came-par)
- [LS-Mod STGI encoding](https://mkwiiki.org/wiki/Lap_%26_Speed_Modifier#How_it_works_(STGI))
- [KCL cannon triggers](https://mkwiiki.org/wiki/KCL_flag#Cannon_Trigger_(0x11))
