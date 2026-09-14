//! Lossless, non-structural edits against a loaded KMP's physical layout.
use std::io::Cursor;

use anyhow::{bail, ensure, Context, Result};
use serde_json::Value;

use crate::util::kmp_file::KmpFile;

// KMP offset-table order, independent of the sections' physical order on disk.
const SECTIONS: [&str; 15] = [
    "ktpt", "enpt", "enph", "itpt", "itph", "ckpt", "ckph", "gobj", "poti", "area", "came", "jgpt", "cnpt", "mspt",
    "stgi",
];

/// Apply only raw scalar fields changed since the editor baseline. Structural
/// edits are rejected; the baseline may expose only the first STGI record.
/// Empty POTI routes must already have been reconciled by the caller.
/// Header/section metadata is retained, except CAME's editable high byte.
///
/// This is a three-way merge: compare the editor's initial normalized values
/// with its current values, but apply changes to parsed original values. Then
/// compare canonical encodings of original and merged models to locate changed
/// bytes, translating those offsets into the original physical layout. Writing
/// the canonical encoding itself would lose gaps, padding, and trailing data.
pub fn patch(original_bytes: &[u8], original: &KmpFile, baseline: &KmpFile, current: &KmpFile) -> Result<Vec<u8>> {
    let original_canonical = canonical(original)?;
    // Validate the supplied snapshot against the physical file, including bounds
    // and section counts. Padding is deliberately absent from this comparison.
    let parsed = KmpFile::read(&mut Cursor::new(original_bytes))?;
    ensure!(
        canonical(&parsed)? == original_canonical,
        "original KMP snapshot does not match the source bytes"
    );
    let original_value = serde_json::to_value(original)?;
    let mut merged = original_value.clone();
    let baseline = serde_json::to_value(baseline)?;
    let current = serde_json::to_value(current)?;
    for section in SECTIONS {
        merge(
            &mut merged[section]["entries"],
            &baseline[section]["entries"],
            &current[section]["entries"],
            &format!("{section}.entries"),
        )?;
    }
    // Section headers are excluded from the recursive merge. Only the intro
    // camera's high byte is editable; preserve the source's other CAME byte.
    let additional = |value: &Value| -> Result<u16> {
        let number = value["came"]["section_header"]["additional_value"]
            .as_u64()
            .context("invalid CAME additional value")?;
        Ok(u16::try_from(number)?)
    };
    let before = additional(&baseline)?;
    let after = additional(&current)?;
    if before & 0xff00 != after & 0xff00 {
        let value = (additional(&merged)? & 0x00ff) | (after & 0xff00);
        merged["came"]["section_header"]["additional_value"] = value.into();
    }
    // Preserve even non-finite original floats on a no-op. JSON cannot round-trip
    // those values, so an actual edit in a file containing them fails below.
    // String comparison also distinguishes +0.0 from -0.0.
    if merged.to_string() == original_value.to_string() {
        return Ok(original_bytes.to_vec());
    }
    let merged: KmpFile = serde_json::from_value(merged)
        .context("cannot preserve KMP scalar values (non-finite floats are unsupported)")?;
    let edited = canonical(&merged)?;
    ensure!(edited.len() == original_canonical.len(), "KMP length changed");
    let canonical_starts = section_starts(&original_canonical)?;
    ensure!(
        section_starts(&edited)? == canonical_starts,
        "KMP section lengths changed"
    );
    let physical_starts = section_starts(original_bytes)?;
    let mut result = original_bytes.to_vec();
    for i in 0..15 {
        let start = canonical_starts[i];
        let end = canonical_starts.get(i + 1).copied().unwrap_or(edited.len());
        let physical = physical_starts[i];
        let physical_end = physical.checked_add(end - start).context("section extent overflow")?;
        // Physical sections may be shuffled or separated by gaps. Bound the
        // payload by the next physical section, not the next table entry.
        let boundary = physical_starts
            .iter()
            .copied()
            .filter(|&s| s > physical)
            .min()
            .unwrap_or(original_bytes.len());
        ensure!(
            physical_end <= boundary && physical_end <= result.len(),
            "section payload is out of bounds"
        );
        // Never replace section headers, padding, or unmodified binary data.
        for offset in 8..end - start {
            if original_canonical[start + offset] != edited[start + offset] {
                result[physical + offset] = edited[start + offset];
            }
        }
        if i == 10 {
            // CAME's opening-camera index is the sole editable header byte.
            if original_canonical[start + 6] != edited[start + 6] {
                result[physical + 6] = edited[start + 6];
            }
        }
    }
    Ok(result)
}

/// Produce a packed encoding for comparison only. Equal canonical padding is
/// ignored by the byte diff, leaving the source's unmodeled padding untouched.
fn canonical(file: &KmpFile) -> Result<Vec<u8>> {
    let mut cursor = Cursor::new(Vec::new());
    file.clone().write(&mut cursor)?;
    Ok(cursor.into_inner())
}

/// Read the 15 table offsets relative to the declared header length, which may
/// exceed the usual 76 bytes. Check arithmetic and header bounds here; the patch
/// loop separately checks each complete payload against its physical boundary.
fn section_starts(bytes: &[u8]) -> Result<Vec<usize>> {
    ensure!(bytes.len() >= 76, "truncated KMP header");
    let header_len = usize::from(u16::from_be_bytes([bytes[10], bytes[11]]));
    ensure!(
        header_len >= 76 && header_len <= bytes.len(),
        "invalid KMP header length"
    );
    (0..15)
        .map(|i| {
            let offset = 16 + i * 4;
            let relative = u32::from_be_bytes(bytes[offset..offset + 4].try_into()?);
            let start = header_len
                .checked_add(usize::try_from(relative)?)
                .context("section offset overflow")?;
            ensure!(
                start.checked_add(8).is_some_and(|end| end <= bytes.len()),
                "invalid section offset"
            );
            Ok(start)
        })
        .collect()
}

/// Walk the serialized record tree, copying only editor changes into original
/// values. Arrays must keep their shape: adding/removing records would invalidate
/// fixed offsets and references. Equal-length reorder/topology edits cannot be
/// identified here, so the document layer must reject those before calling us.
/// `path` supplies error context and identifies format-specific merge rules.
fn merge(original: &mut Value, baseline: &Value, current: &Value, path: &str) -> Result<()> {
    // Euler triples are one orientation, not independent coordinates. The
    // editor canonicalizes equivalent rotations through a quaternion; mixing
    // original axes with edited canonical axes can produce a different rotation.
    if path.ends_with(".rotation") && baseline.to_string() != current.to_string() {
        ensure!(
            current
                .as_array()
                .is_some_and(|axes| axes.len() == 3 && axes.iter().all(Value::is_number)),
            "unsupported rotation at {path}"
        );
        *original = current.clone();
        return Ok(());
    }
    match (original, baseline, current) {
        (Value::Array(original), Value::Array(baseline), Value::Array(current)) => {
            ensure!(baseline.len() == current.len(), "structural edit at {path}");
            // Older/narrower snapshots may expose only the editable STGI row;
            // merge that prefix without discarding additional source records.
            let prefix = path == "stgi.entries" && baseline.len() == 1 && !original.is_empty();
            ensure!(
                original.len() == baseline.len() || prefix,
                "snapshot structure mismatch at {path}"
            );
            for (i, (before, after)) in baseline.iter().zip(current).enumerate() {
                merge(&mut original[i], before, after, &format!("{path}.{i}"))?;
            }
        }
        (Value::Object(original), Value::Object(baseline), Value::Object(current)) => {
            ensure!(
                original.len() == baseline.len() && baseline.len() == current.len(),
                "object structure mismatch at {path}"
            );
            for (key, before) in baseline {
                let after = current.get(key).context("missing current field")?;
                let value = original.get_mut(key).context("missing original field")?;
                merge(value, before, after, &format!("{path}.{key}"))?;
            }
        }
        (original @ Value::Number(_), Value::Number(before), Value::Number(after)) => {
            // Compare entire numbers, NOT byte deltas between editor snapshots:
            // a normalized baseline float can share bytes with the edited value
            // that differ in the original, and those bytes must change too.
            if before.to_string() != after.to_string() {
                // ITPT setting_2 is a bitfield: the editor exposes only some
                // bits. Apply toggled bits rather than erasing unknown flags.
                if path.starts_with("itpt.entries.") && path.ends_with(".setting_2") {
                    let before = before.as_u64().context("invalid ITPT flags")?;
                    let after = after.as_u64().context("invalid ITPT flags")?;
                    let old = original.as_u64().context("invalid original ITPT flags")?;
                    let mask = before ^ after;
                    *original = ((old & !mask) | (after & mask)).into();
                } else {
                    *original = Value::Number(after.clone());
                }
            }
        }
        (original @ Value::Bool(_), Value::Bool(before), Value::Bool(after)) => {
            if before != after {
                *original = (*after).into();
            }
        }
        // serde_json represents non-finite floats as null. Leave unchanged
        // nulls alone for the exact no-op fast path; an edited document that
        // still contains them will fail conversion back to KmpFile safely.
        (_, Value::Null, Value::Null) => {}
        (Value::Null, Value::Number(before), Value::Number(after)) if before == after => {}
        _ => bail!("unsupported or non-finite scalar at {path}"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::kmp_file::{Cnpt, Itpt, Ktpt, Mspt, Poti, PotiPoint, Section, Stgi};

    /// Equivalent imported Euler angles must stay untouched until edited; then
    /// replace the entire orientation rather than mixing two representations.
    #[test]
    fn edited_rotation_replaces_whole_canonical_orientation() {
        let mut original = serde_json::json!([0.0, 270.0, 0.0]);
        let baseline = serde_json::json!([180.0, -90.0, 180.0]);
        let current = serde_json::json!([180.0, -80.0, 180.0]);
        merge(&mut original, &baseline, &current, "ktpt.entries.0.rotation").unwrap();
        assert_eq!(original, current);
    }

    /// Build a deliberately noncanonical physical file and a lossy editor
    /// baseline. Extended headers, shuffled sections, gaps, opaque bytes, and
    /// extra STGI records make accidental whole-file regeneration observable.
    fn fixture() -> (Vec<u8>, KmpFile, KmpFile) {
        let mut file = KmpFile::default();
        file.ktpt = Section::new(vec![
            Ktpt {
                position: [1.234567, 2.0, 3.0],
                player_index: -1,
                ..Default::default()
            },
            Ktpt::default(),
        ]);
        file.itpt = Section::new(vec![Itpt {
            setting_1: 0x1234,
            setting_2: 0xa580,
            ..Default::default()
        }]);
        file.cnpt = Section::new(vec![Cnpt::default()]);
        file.mspt = Section::new(vec![Mspt::default()]);
        file.stgi = Section::new(vec![
            Stgi {
                padding_2: 0xabcd,
                ..Default::default()
            },
            Stgi {
                lap_count: 7,
                ..Default::default()
            },
        ]);
        file.poti = Section::new(vec![Poti {
            points: vec![PotiPoint::default()],
            ..Default::default()
        }]);
        file.came.section_header.additional_value = 0x127e;
        let packed = canonical(&file).unwrap();
        let starts = section_starts(&packed).unwrap();
        let mut bytes = packed[..76].to_vec();
        bytes.extend_from_slice(&[0xab; 12]);
        bytes[10..12].copy_from_slice(&88u16.to_be_bytes());
        // Reverse physical section order and insert gaps between every section.
        for i in (0..15).rev() {
            bytes.extend_from_slice(&[0xcd; 5]);
            let offset = (bytes.len() - 88) as u32;
            bytes[16 + i * 4..20 + i * 4].copy_from_slice(&offset.to_be_bytes());
            let end = starts.get(i + 1).copied().unwrap_or(packed.len());
            bytes.extend_from_slice(&packed[starts[i]..end]);
        }
        let len = bytes.len() as u32;
        bytes[4..8].copy_from_slice(&len.to_be_bytes());
        bytes.extend_from_slice(b"retained tail");
        let physical = section_starts(&bytes).unwrap();
        for (index, offset) in [(0, 26), (12, 24), (13, 24)] {
            let start = physical[index] + 8 + offset;
            bytes[start..start + 2].copy_from_slice(&[0xde, 0xad]);
        }
        bytes[physical[0] + 6..physical[0] + 8].copy_from_slice(&0xbeefu16.to_be_bytes());
        let original = KmpFile::read(&mut Cursor::new(&bytes)).unwrap();
        let mut baseline = original.clone();
        baseline.ktpt[0].position[0] = 1.0;
        baseline.itpt[0].setting_1 = 0;
        baseline.itpt[0].setting_2 = 0;
        baseline.came.section_header.additional_value = 0x1200;
        baseline.stgi.entries.truncate(1);
        baseline.stgi[0].padding_2 = 0;
        (bytes, original, baseline)
    }

    /// Import normalization alone must never produce a disk edit.
    #[test]
    fn no_op_preserves_every_byte() {
        let (bytes, original, baseline) = fixture();
        assert_eq!(patch(&bytes, &original, &baseline, &baseline).unwrap(), bytes);
    }

    /// Check exact changed byte ranges as well as decoded values, covering whole
    /// scalars, masked flags, the CAME header exception, and retained unknown data.
    #[test]
    fn edits_merge_whole_scalars_and_preserve_unknown_data() {
        let (bytes, original, baseline) = fixture();
        let mut current = baseline.clone();
        current.ktpt[0].position[0] = 2.0;
        current.ktpt[0].player_index = 3;
        current.itpt[0].setting_2 = 1;
        current.came.section_header.additional_value = 0x3400;
        current.stgi[0].lap_count = 5;
        let patched = patch(&bytes, &original, &baseline, &current).unwrap();
        let parsed = KmpFile::read(&mut Cursor::new(&patched)).unwrap();
        assert_eq!(parsed.ktpt[0].position[0], 2.0);
        assert_eq!(parsed.itpt[0].setting_1, 0x1234);
        assert_eq!(parsed.itpt[0].setting_2, 0xa581);
        assert_eq!(parsed.came.section_header.additional_value, 0x347e);
        assert_eq!(parsed.stgi[0].padding_2, 0xabcd);
        assert_eq!(parsed.stgi[1].lap_count, 7);
        let starts = section_starts(&bytes).unwrap();
        let mut expected = bytes.clone();
        expected[starts[0] + 8..starts[0] + 12].copy_from_slice(&2.0f32.to_be_bytes());
        expected[starts[0] + 32..starts[0] + 34].copy_from_slice(&3i16.to_be_bytes());
        expected[starts[3] + 26..starts[3] + 28].copy_from_slice(&0xa581u16.to_be_bytes());
        expected[starts[10] + 6] = 0x34;
        expected[starts[14] + 8] = 5;
        assert_eq!(patched, expected);
    }

    /// Numeric equality is insufficient for floats: changing the sign of zero
    /// must replace the source scalar with the requested IEEE representation.
    #[test]
    fn signed_zero_is_a_whole_scalar_edit() {
        let (bytes, original, mut baseline) = fixture();
        baseline.ktpt[0].position[0] = 0.0;
        let mut current = baseline.clone();
        current.ktpt[0].position[0] = -0.0;
        let patched = patch(&bytes, &original, &baseline, &current).unwrap();
        let parsed = KmpFile::read(&mut Cursor::new(&patched)).unwrap();
        assert_eq!(parsed.ktpt[0].position[0].to_bits(), (-0.0f32).to_bits());
    }

    /// Preserve NaN bytes on no-op, but reject an edit requiring a JSON round trip
    /// that cannot reconstruct the remaining non-finite scalar.
    #[test]
    fn non_finite_no_op_is_exact_but_edits_fail_safely() {
        let (mut bytes, _, _) = fixture();
        let start = section_starts(&bytes).unwrap()[0] + 8;
        bytes[start..start + 4].copy_from_slice(&f32::NAN.to_be_bytes());
        let original = KmpFile::read(&mut Cursor::new(&bytes)).unwrap();
        assert_eq!(patch(&bytes, &original, &original, &original).unwrap(), bytes);
        let mut current = original.clone();
        current.ktpt[0].player_index = 1;
        assert!(patch(&bytes, &original, &original, &current).is_err());
    }

    /// Refuse outer/nested record-count changes, mismatched baselines, and
    /// truncated source bytes rather than patching an incompatible layout.
    #[test]
    fn rejects_structural_edits_and_inconsistent_snapshots() {
        let (bytes, original, baseline) = fixture();
        let mut current = baseline.clone();
        current.ktpt.entries.pop();
        assert!(patch(&bytes, &original, &baseline, &current).is_err());
        current = baseline.clone();
        current.poti[0].points.push(PotiPoint::default());
        assert!(patch(&bytes, &original, &baseline, &current).is_err());
        current = baseline.clone();
        current.itpt.entries.clear();
        assert!(patch(&bytes, &original, &current, &current).is_err());
        assert!(patch(&bytes[..75], &original, &baseline, &baseline).is_err());
    }
}
