use bevy::{math::vec3, prelude::*};
use byteorder::{ReadBytesExt, BE};
use std::io::{self, Read, Seek, SeekFrom};
use strum_macros::{Display, EnumIter, EnumString, IntoStaticStr};

#[allow(dead_code)]
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

#[derive(Clone)]
pub struct VertexGroup {
    pub vertices: Vec<Vec3>,
}

impl Kcl {
    pub fn read(mut rdr: impl Read + Seek) -> io::Result<Self> {
        // offsets of position data, normals data, triangular prims, spatial index
        let mut offsets = [0u32; 4];
        for e in offsets.iter_mut() {
            *e = rdr.read_u32::<BE>()?;
        }

        // go to the start of pos_data
        rdr.seek(SeekFrom::Start(offsets[0] as u64))?;

        let mut vertices = Vec::new();

        // while the current position of the cursor is still in the position data section
        while rdr.stream_position()? < offsets[1] as u64 {
            let x = rdr.read_f32::<BE>()?;
            let y = rdr.read_f32::<BE>()?;
            let z = rdr.read_f32::<BE>()?;
            vertices.push(vec3(x, y, z));
        }

        // go to the start of the normal data section
        rdr.seek(SeekFrom::Start(offsets[1].into()))?;

        let mut normals: Vec<Vec3> = Vec::new();

        // while the current position is still in the normal data section
        // + 0x10 because the triangular prisms section starts 0x10 further along than it says it is
        while rdr.stream_position()? < (offsets[2] + 0x10) as u64 {
            let x = rdr.read_f32::<BE>()?;
            let y = rdr.read_f32::<BE>()?;
            let z = rdr.read_f32::<BE>()?;
            normals.push(vec3(x, y, z));
        }

        // go to the start of the triangular prisms section
        rdr.seek(SeekFrom::Start(offsets[2] as u64 + 0x10))?;

        let mut vertex_groups: Vec<VertexGroup> = Vec::with_capacity(32);
        for _ in 0..32 {
            vertex_groups.push(VertexGroup {
                vertices: Vec::new(),
            })
        }

        while rdr.stream_position()? < offsets[3] as u64 {
            let length = rdr.read_f32::<BE>()?;
            let pos_index = rdr.read_u16::<BE>()? as usize;
            let face_nrm_index = rdr.read_u16::<BE>()? as usize;

            let nrm_a_index = rdr.read_u16::<BE>()? as usize;
            let nrm_b_index = rdr.read_u16::<BE>()? as usize;
            let nrm_c_index = rdr.read_u16::<BE>()? as usize;

            let kcl_flag = rdr.read_u16::<BE>()?;
            // elimanates all the other data apart from the base type
            let kcl_type = (kcl_flag & 0x1f) as usize;

            if pos_index >= vertices.len()
                || face_nrm_index >= normals.len()
                || nrm_a_index >= normals.len()
                || nrm_b_index >= normals.len()
                || nrm_c_index >= normals.len()
            {
                continue;
            }

            let vertex = &vertices[pos_index];
            let face_nrm = &normals[face_nrm_index];

            let nrm_a = &normals[nrm_a_index];
            let nrm_b = &normals[nrm_b_index];
            let nrm_c = &normals[nrm_c_index];

            let cross_a = nrm_a.cross(*face_nrm);
            let cross_b = nrm_b.cross(*face_nrm);

            let v1 = *vertex;
            let v2 = *vertex + (cross_b * (length / cross_b.dot(*nrm_c)));
            let v3 = *vertex + (cross_a * (length / cross_a.dot(*nrm_c)));

            vertex_groups[kcl_type].vertices.extend([v1, v2, v3]);
        }
        Ok(Kcl { vertex_groups })
    }
}
