use byteorder::{ReadBytesExt, BE};
use std::io::{self, Read, Seek, SeekFrom};

use bevy::{math::vec3, prelude::*, render::mesh::PrimitiveTopology};

pub const KCL_COLORS: [[f32; 4]; 32] = [
    [1.0, 1.0, 1.0, 1.0], // road
    [1.0, 0.9, 0.8, 1.0], // slippery road (sand/dirt)
    [0.0, 0.8, 0.0, 1.0], // weak off-road
    [0.0, 0.6, 0.0, 1.0], // off-road
    [0.0, 0.4, 0.0, 1.0], // heavy off-road
    [0.8, 0.9, 1.0, 1.0], // slippery road (ice)
    [1.0, 0.5, 0.0, 1.0], // boost panel
    [1.0, 0.6, 0.0, 1.0], // boost ramp
    [1.0, 0.8, 0.0, 1.0], // slow ramp
    [0.9, 0.9, 1.0, 0.5], // item road
    [0.7, 0.1, 0.1, 1.0], // solid fall
    [0.0, 0.5, 1.0, 1.0], // moving water
    [0.6, 0.6, 0.6, 1.0], // wall
    [0.0, 0.0, 0.6, 0.8], // invisible wall
    [0.6, 0.6, 0.7, 0.5], // item wall
    [0.6, 0.6, 0.6, 1.0], // wall
    [0.8, 0.0, 0.0, 0.8], // fall boundary
    [1.0, 0.0, 0.5, 0.8], // cannon activator
    [0.5, 0.0, 1.0, 0.5], // force recalculation
    [0.0, 0.3, 1.0, 1.0], // half-pipe ramp
    [0.6, 0.6, 0.6, 1.0], // wall (items pass through)
    [0.9, 0.9, 1.0, 1.0], // moving road
    [0.9, 0.7, 1.0, 1.0], // sticky road
    [1.0, 1.0, 1.0, 1.0], // road (alt sfx)
    [1.0, 0.0, 1.0, 0.8], // sound trigger
    [0.4, 0.6, 0.4, 0.8], // weak wall
    [0.8, 0.0, 1.0, 0.8], // effect trigger
    [1.0, 0.0, 1.0, 0.5], // item state modifier
    [0.0, 0.6, 0.0, 0.8], // half-pipe invis wall
    [0.9, 0.9, 1.0, 1.0], // rotating road
    [0.8, 0.7, 0.8, 1.0], // special wall
    [0.6, 0.6, 0.6, 1.0], // wall
];

pub struct KCL {
    pub vertex_groups: Vec<Vec<Vec3>>,
}

impl KCL {
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
        while rdr.seek(SeekFrom::Current(0))? < offsets[1] as u64 {
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
        while rdr.seek(SeekFrom::Current(0))? < (offsets[2] + 0x10) as u64 {
            let x = rdr.read_f32::<BE>()?;
            let y = rdr.read_f32::<BE>()?;
            let z = rdr.read_f32::<BE>()?;
            normals.push(vec3(x, y, z));
        }

        // go to the start of the triangular prisms section
        rdr.seek(SeekFrom::Start(offsets[2] as u64 + 0x10))?;

        let mut vertex_groups: Vec<Vec<Vec3>> = Vec::new();

        for _ in 0..KCL_COLORS.len() {
            vertex_groups.push(Vec::new());
        }

        while rdr.seek(SeekFrom::Current(0))? < offsets[3] as u64 {
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

            vertex_groups[kcl_type].extend([v1, v2, v3]);
        }
        Ok(KCL { vertex_groups })
    }

    pub fn build_model(
        &self,
        commands: &mut Commands,
        meshes: &mut ResMut<Assets<Mesh>>,
        materials: &mut ResMut<Assets<StandardMaterial>>,
    ) {
        for (i, vertex_group) in self.vertex_groups.iter().enumerate() {
            let vertex_group = vertex_group.clone();

            let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);

            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertex_group);

            mesh.compute_flat_normals();

            let color: Color = KCL_COLORS[i].into();

            commands.spawn(PbrBundle {
                mesh: meshes.add(mesh),
                material: materials.add(StandardMaterial {
                    base_color: color,
                    cull_mode: None,
                    double_sided: true,
                    alpha_mode: if color.a() < 1. {
                        AlphaMode::Add
                    } else {
                        AlphaMode::Opaque
                    },
                    ..default()
                }),
                ..default()
            });
        }
    }
}
