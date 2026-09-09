use crate::util::read_write_arrays::ReadArrays;
use bevy::{math::vec3, prelude::*};
use byteorder::{ReadBytesExt, BE};
use std::io::{self, Read, Seek, SeekFrom};
use strum_macros::{Display, EnumIter, EnumString, IntoStaticStr};

#[derive(Display, EnumString, IntoStaticStr, EnumIter)]
pub enum KclFlag {
    #[strum(serialize = "Road 1")]
    Road1,
    #[strum(serialize = "Slippery Road 1")]
    SlipperyRoad1,
    #[strum(serialize = "Weak Offroad")]
    WeakOffroad,
    #[strum(serialize = "Offroad")]
    Offroad,
    #[strum(serialize = "Heavy Offroad")]
    HeavyOffroad,
    #[strum(serialize = "Slippery Road 2")]
    SlipperyRoad2,
    #[strum(serialize = "Boost Panel")]
    BoostPanel,
    #[strum(serialize = "Boost Ramp")]
    BoostRamp,
    #[strum(serialize = "Jump Pad")]
    JumpPad,
    #[strum(serialize = "Item Road")]
    ItemRoad,
    #[strum(serialize = "Solid Fall")]
    SolidFall,
    #[strum(serialize = "Moving Water")]
    MovingWater,
    #[strum(serialize = "Wall 1")]
    Wall1,
    #[strum(serialize = "Invisible Wall 1")]
    InvisibleWall1,
    #[strum(serialize = "Item Wall")]
    ItemWall,
    #[strum(serialize = "Wall 2")]
    Wall2,
    #[strum(serialize = "Fall Boundary")]
    FallBoundary,
    #[strum(serialize = "Cannon Trigger")]
    CannonTrigger,
    #[strum(serialize = "Force Recalculation")]
    ForceRecalculation,
    #[strum(serialize = "Half Pipe Ramp")]
    HalfPipeRamp,
    #[strum(serialize = "Player Only Wall")]
    PlayerOnlyWall,
    #[strum(serialize = "Moving Road")]
    MovingRoad,
    #[strum(serialize = "Sticky Road")]
    StickyRoad,
    #[strum(serialize = "Road 2")]
    Road2,
    #[strum(serialize = "Sound Trigger")]
    SoundTrigger,
    #[strum(serialize = "Weak Wall")]
    WeakWall,
    #[strum(serialize = "Effect Trigger")]
    EffectTrigger,
    #[strum(serialize = "Item State Modifier")]
    ItemStateModifier,
    #[strum(serialize = "Half Pipe Invisible Wall")]
    HalfPipeInvisibleWall,
    #[strum(serialize = "Rotating Road")]
    RotatingRoad,
    #[strum(serialize = "Special Wall")]
    SpecialWall,
    #[strum(serialize = "Invisible Wall 2")]
    InvisibleWall2,
}

#[derive(Resource)]
pub struct Kcl {
    pub vertex_groups: Vec<VertexGroup>,
}
impl Default for Kcl {
    fn default() -> Self {
        let mut vertex_groups: Vec<VertexGroup> = Vec::with_capacity(32);
        for _ in 0..32 {
            vertex_groups.push(VertexGroup { vertices: Vec::new() })
        }
        Self { vertex_groups }
    }
}

#[derive(Clone)]
pub struct VertexGroup {
    pub vertices: Vec<Vec3>,
}

impl Kcl {
    pub fn read(mut r: impl Read + Seek) -> io::Result<Self> {
        let file_len = r.seek(SeekFrom::End(0))?;
        r.seek(SeekFrom::Start(0))?;

        // Offsets of position data, normals data, triangular prisms, and spatial index.
        let mut offsets = [0u32; 4];
        for offset in &mut offsets {
            *offset = r.read_u32::<BE>()?;
        }

        let position_start = u64::from(offsets[0]);
        let normal_start = u64::from(offsets[1]);
        let prism_start = u64::from(
            offsets[2]
                .checked_add(0x10)
                .ok_or_else(|| invalid_data("KCL prism offset overflow"))?,
        );
        let spatial_start = u64::from(offsets[3]);

        if position_start < 0x10
            || position_start > normal_start
            || normal_start > prism_start
            || prism_start > spatial_start
            || spatial_start > file_len
        {
            return Err(invalid_data("KCL section offsets are invalid or outside the file"));
        }
        if (normal_start - position_start) % 12 != 0
            || (prism_start - normal_start) % 12 != 0
            || (spatial_start - prism_start) % 16 != 0
        {
            return Err(invalid_data("KCL section sizes are not aligned to their record sizes"));
        }

        r.seek(SeekFrom::Start(position_start))?;
        let mut vertices = Vec::new();
        while r.stream_position()? < normal_start {
            let vertex = r.read_vec3()?;
            if !vertex.is_finite() {
                return Err(invalid_data("KCL contains a non-finite vertex"));
            }
            vertices.push(vertex);
        }

        r.seek(SeekFrom::Start(normal_start))?;
        let mut normals = Vec::new();
        while r.stream_position()? < prism_start {
            let normal = vec3(r.read_f32::<BE>()?, r.read_f32::<BE>()?, r.read_f32::<BE>()?);
            if !normal.is_finite() {
                return Err(invalid_data("KCL contains a non-finite normal"));
            }
            normals.push(normal);
        }

        r.seek(SeekFrom::Start(prism_start))?;
        let mut kcl = Kcl::default();
        let mut prism_count = 0_usize;
        let mut valid_triangle_count = 0_usize;
        while r.stream_position()? < spatial_start {
            prism_count += 1;
            let length = r.read_f32::<BE>()?;
            let pos_index = r.read_u16::<BE>()? as usize;
            let face_nrm_index = r.read_u16::<BE>()? as usize;
            let nrm_a_index = r.read_u16::<BE>()? as usize;
            let nrm_b_index = r.read_u16::<BE>()? as usize;
            let nrm_c_index = r.read_u16::<BE>()? as usize;
            let kcl_flag = r.read_u16::<BE>()?;
            let kcl_type = (kcl_flag & 0x1f) as usize;

            let Some((vertex, face_nrm, nrm_a, nrm_b, nrm_c)) = vertices
                .get(pos_index)
                .zip(normals.get(face_nrm_index))
                .zip(normals.get(nrm_a_index))
                .zip(normals.get(nrm_b_index))
                .zip(normals.get(nrm_c_index))
                .map(|((((vertex, face_nrm), nrm_a), nrm_b), nrm_c)| (vertex, face_nrm, nrm_a, nrm_b, nrm_c))
            else {
                continue;
            };
            if !length.is_finite() {
                continue;
            }

            let cross_a = nrm_a.cross(*face_nrm);
            let cross_b = nrm_b.cross(*face_nrm);
            let denominator_a = cross_a.dot(*nrm_c);
            let denominator_b = cross_b.dot(*nrm_c);
            if !denominator_a.is_finite()
                || !denominator_b.is_finite()
                || denominator_a.abs() <= f32::EPSILON
                || denominator_b.abs() <= f32::EPSILON
            {
                continue;
            }

            let v1 = *vertex;
            let v2 = *vertex + cross_b * (length / denominator_b);
            let v3 = *vertex + cross_a * (length / denominator_a);
            if v1.is_finite() && v2.is_finite() && v3.is_finite() {
                kcl.vertex_groups[kcl_type].vertices.extend([v1, v2, v3]);
                valid_triangle_count += 1;
            }
        }
        if prism_count > 0 && valid_triangle_count == 0 {
            return Err(invalid_data("KCL contains no usable triangle prisms"));
        }
        Ok(kcl)
    }
}

fn invalid_data(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn rejects_out_of_order_offsets() {
        let mut bytes = Vec::new();
        for offset in [16_u32, 12, 0, 16] {
            bytes.extend_from_slice(&offset.to_be_bytes());
        }

        let error = match Kcl::read(Cursor::new(bytes)) {
            Ok(_) => panic!("invalid KCL offsets should be rejected"),
            Err(error) => error,
        };
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn rejects_kcl_when_all_prisms_are_invalid() {
        let mut bytes = Vec::new();
        for offset in [16_u32, 28, 48, 80] {
            bytes.extend_from_slice(&offset.to_be_bytes());
        }
        for value in [0.0_f32; 12] {
            bytes.extend_from_slice(&value.to_be_bytes());
        }
        bytes.extend_from_slice(&1.0_f32.to_be_bytes());
        for value in [1_u16, 0, 0, 0, 0, 0] {
            bytes.extend_from_slice(&value.to_be_bytes());
        }

        let error = match Kcl::read(Cursor::new(bytes)) {
            Ok(_) => panic!("a KCL with no usable prisms should be rejected"),
            Err(error) => error,
        };
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn reads_course_fixtures_without_non_finite_geometry() {
        for relative_path in [
            "test_files/desert_course/course.kcl",
            "test_files/boardcross_course/course.kcl",
            "test_files/shopping_course/course.kcl",
        ] {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(relative_path);
            let kcl = Kcl::read(std::fs::File::open(&path).unwrap()).unwrap();
            let vertices = kcl.vertex_groups.iter().flat_map(|group| &group.vertices);
            assert!(
                vertices.clone().next().is_some(),
                "{} contains no triangles",
                path.display()
            );
            assert!(vertices.into_iter().all(|vertex| vertex.is_finite()));
        }
    }

    #[test]
    fn rejects_offsets_beyond_end_of_file() {
        let mut bytes = Vec::new();
        for offset in [16_u32, 16, 0, 32] {
            bytes.extend_from_slice(&offset.to_be_bytes());
        }

        let error = match Kcl::read(Cursor::new(bytes)) {
            Ok(_) => panic!("invalid KCL offsets should be rejected"),
            Err(error) => error,
        };
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }
}
