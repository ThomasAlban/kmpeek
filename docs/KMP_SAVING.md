# KMP saving: patch and rebuild modes

## Choosing a mode

**Settings → General → Patch saving (preserve original KMP data)** is disabled by default. Old settings files that lack this property also default to disabled. The choice is persisted with the other application settings.

| Mode | Behavior |
| --- | --- |
| Patch saving ON | Preserve source layout and untouched values. A no-edit save is byte-identical. Field edits are patched into their original records. Structural changes are refused with instructions to switch modes. |
| Patch saving OFF | Rebuild all represented editor sections in a deterministic packed layout. Add/delete/reconnect/reorder edits are saved, supported references remapped, and local IDs renumbered. Unrepresented/unsupported source data may be discarded. |

A data-loss explanation is shown beside the toggle in Settings while patch saving is off; it is intentionally not a permanent top-level banner. **Use Save As on copies when testing.** Both modes share `document::save` and its atomic replacement; rebuild mode does not mean opening the original file with truncation.

After a successful rebuild, the saved file is reloaded into the editor. This clears selection and makes displayed indices, resolved links, and the next patch baseline agree with the actual output. You can then turn patch mode back on. Save runs after the frame's deferred editor changes and route maintenance, not halfway through a structural operation.

## Patch contract

For a successfully loaded KMP with no persistent edits, patch saving returns the **exact source bytes**, including section order, group/link slots, unused records, empty routes, raw IDs, padding, unknown settings, unedited Euler angles, header extensions, gaps, and trailing data. A game-ready input remains the same game-ready KMP.

Changes are measured against the editor's post-import baseline, not the original parsed model, because importing can normalize values. Changed scalar fields are merged into original records. Euler triples are replaced together when orientation changes; ITPT flag edits retain unknown bits. CAME's opening-camera index does not overwrite its second metadata byte.

Patch saving still refuses structural changes, conflicting edits to aliases of a shared source point, and removal of required checkpoint respawn links. Non-finite source floats survive no-op saves, but scalar patching that requires their JSON round-trip can fail safely.

## Rebuild contract

`rebuild.rs` plans complete output order before converting records:

- Every current path entity is covered once; branch, cycle, disconnected and singleton ENPT/ITPT/CKPT graphs are supported.
- Groups are contiguous chains with deterministic links and checked starts/lengths. Long chains split rather than truncate. Original group boundaries, duplicate link slots, asymmetric metadata and `group_link` values are not retained; rebuilt `group_link` is zero.
- POTI is rebuilt from linear route chains and their smooth/loop settings. Splitting, merging and deleting route points is supported. Deleting a route start transfers its settings and references; existing destination settings win an ambiguous merge. Graph cycles must be disconnected and expressed with route loop settings; POTI cannot represent branching graphs.
- Route references use final POTI indices, including links aimed at a nonstart route member. Objects support 16-bit route indices; camera/AREA routes have narrower limits.
- Checkpoint neighbors use final serialized group order, not display OrderIds. Respawns target final JGPT section positions, not stored JGPT IDs.
- Camera next links and AREA camera/enemy-point links follow surviving source entity identities even when their target's numeric index changes. New or edited numeric values resolve against current editor row order, then map to final output order. An explicitly invalid index is refused. Unchanged optional camera references whose targets were deleted become `255`; missing required respawn/enemy targets must be relinked.
- Both resolvable CAME header camera indices are remapped. The low byte is selection-video metadata, not the replay-camera root; Wii ignores it at runtime. Replay associations reside in AREA camera links.
- JGPT/CNPT/MSPT local IDs are assigned dense section indices, matching the conventional Nintendo layout. They are internal file bookkeeping and are never exposed as editor controls.
- All 15 section headers, counts, POTI totals, offsets and file length are regenerated. Serialized output is parsed again before disk replacement.

Rebuild removes unrepresented orphan points, empty source routes, additional unrepresented STGI records, source gaps/extensions/tail bytes and unsupported metadata. It does **not** garbage-collect every unlinked visible object or camera: current editor entities are intentional content, not automatically unused data. Represented raw fields, such as object extended presence and MSPT's unknown word, are retained from the editor.

Invalid/asymmetric/stale/cross-section edges, malformed checkpoint pairs, non-finite fields, out-of-range indices and unrepresentable counts are errors, not silent truncation. ENPT/ITPT are limited to 255 points; group indices reserve `255`; checkpoint layouts needing non-sentinel neighbor indices beyond the byte range are refused.

## Editable fields and scope

All currently represented editor sections participate in both encoders. The edit panel includes AREA moving-road route links and forced-recalculation enemy-point indices, full signed JGPT extra data, object presence, and the STGI speed modifier. Internal IDs, padding/reserved words, and unknown bookkeeping are deliberately not exposed: patch mode preserves them, while rebuild mode generates or retains them according to its canonical policy.

STGI speed uses the LS-Mod extension: its last two bytes are the **high word of an IEEE-754 float**, not half precision. Encoded zero means normal (1×) speed; saving truncates the low word. Game-side extension support is required. Track type and visibility controls remain editor-only; they are not invented KMP binary fields.

This is not an arbitrary raw-byte editor: unknown enum encodings, inactive AREA fields and object-specific setting semantics are not all individually modeled. Patch mode preserves untouched unsupported values; rebuild may normalize/drop them as its warning states. Neither mode proves that arbitrary edits create a playable course.

### External references cannot be remapped by a KMP-only save

KCL cannon triggers refer to **CNPT array indices**, not stored cannon IDs. If cannons are reordered, inserted before existing targets, or deleted, update the KCL triggers separately. Rewriting KMP IDs alone cannot repair those references. Opaque object-specific settings may also have external meanings; the writer must not guess that every number is an index.

## File safety

Both encoders finish before disk I/O. Saving creates an exclusive same-directory temporary file, writes and syncs it, preserves permissions, and renames it over the destination. Failures before replacement leave the original intact. Save As changes the active path only after the write succeeds. A refresh error after a rebuild explicitly reports that the file was saved but needs reopening.

Opened and previously saved destinations are checked against their last known bytes before temporary writing and again before replacement. External edits cause a refusal. This is not an OS lock: another writer can still race the final check. File permissions are copied, not all ACLs/xattrs. The directory is not fsynced, so full power-loss durability is not claimed. Windows and network-filesystem behavior require platform testing.

Opening another KMP or requesting application close checks the KMP bytes represented by the current editor against the last successful save. Persisted field and structural edits prompt with **Save / Don't Save / Cancel**; editor-only selection, visibility, camera, and layout changes do not. A failed or unrepresentable save keeps the prompt open and does not continue the destructive action. KCL-only loading does not replace the KMP and therefore does not prompt. Undo remains unimplemented.

## Verification

```sh
cargo fmt --all -- --check
cargo check --locked
cargo test --locked
cargo test --locked --release
```

The suite covers all-section binary and actual headless-editor roundtrips, original layout/metadata, settings defaults/import/export, field conversions, mode switching, rebuild→reload→patch, structural reference remapping, source-index holes, route settings migration, count limits, failed writes and disk conflicts. Graph tests exhaust all 512 directed three-node graphs, with and without an anchored start. Synthetic fixtures do not replace stock race/battle or console testing.

### Manual acceptance (copies only)

1. Record platform, build mode, commit and source hashes for known-working race and battle course copies.
2. With patch saving ON, load each KMP/KCL, wait several frames, change selection/view/visibility, Save and Save As without persistent edits. Full bytes/hashes must match.
3. Make representative field edits. Check intended fields and unrelated bytes; test raw fields, camera zoom endpoints, route/respawn links and the speed extension separately.
4. Make add/delete/reconnect/reorder edits. Patch mode must refuse without writing. Turn patch mode OFF, verify the warning, and Save As.
5. Inspect the rebuilt file with an independent KMP tool: every intended point remains, counts/links are consistent, and all supported references still select the intended targets. Test branches, cycles, disconnected paths, singleton groups, route split/merge/start deletion and camera reindexing.
6. Verify the editor refreshes to new indices after rebuild. Turn patch saving ON; an immediate save must now be byte-identical to the rebuilt file, and subsequent field patches must affect the correct records.
7. Test missing required targets, invalid graph links, width overflows, unwritable destinations and external disk edits. No failure may truncate the old file.
8. Validate stock race/battle copies in Dolphin/Wii, including race start, CPUs/items, laps/checkpoints, respawns, cameras, objects and cannon triggers. Reconcile external KCL references before testing reordered cannons. Repeat debug/release and supported platforms.

### Format references

- [KMP fields and CAME metadata](https://mkwiiki.org/wiki/KMP_(File_Format))
- [Wiimm CAME parameters](https://szs.wiimm.de/info/kmp-syntax.html#came-par)
- [LS-Mod STGI encoding](https://mkwiiki.org/wiki/Lap_%26_Speed_Modifier#How_it_works_(STGI))
- [KCL cannon triggers](https://mkwiiki.org/wiki/KCL_flag#Cannon_Trigger_(0x11))
