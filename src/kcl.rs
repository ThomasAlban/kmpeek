use byteorder::{ReadBytesExt, BE};
use std::io::{self, Read, Seek, SeekFrom};
use three_d::*;
use three_d_asset::*;

// pub const KCL_COLORS: [[f32; 4]; 32] = [
//     [1.0, 1.0, 1.0, 1.0], // road
//     [1.0, 0.9, 0.8, 1.0], // slippery road (sand/dirt)
//     [0.0, 0.8, 0.0, 1.0], // weak off-road
//     [0.0, 0.6, 0.0, 1.0], // off-road
//     [0.0, 0.4, 0.0, 1.0], // heavy off-road
//     [0.8, 0.9, 1.0, 1.0], // slippery road (ice)
//     [1.0, 0.5, 0.0, 1.0], // boost panel
//     [1.0, 0.6, 0.0, 1.0], // boost ramp
//     [1.0, 0.8, 0.0, 1.0], // slow ramp
//     [0.9, 0.9, 1.0, 0.5], // item road
//     [0.7, 0.1, 0.1, 1.0], // solid fall
//     [0.0, 0.5, 1.0, 1.0], // moving water
//     [0.6, 0.6, 0.6, 1.0], // wall
//     [0.0, 0.0, 0.6, 0.8], // invisible wall
//     [0.6, 0.6, 0.7, 0.5], // item wall
//     [0.6, 0.6, 0.6, 1.0], // wall
//     [0.8, 0.0, 0.0, 0.8], // fall boundary
//     [1.0, 0.0, 0.5, 0.8], // cannon activator
//     [0.5, 0.0, 1.0, 0.5], // force recalculation
//     [0.0, 0.3, 1.0, 1.0], // half-pipe ramp
//     [0.6, 0.6, 0.6, 1.0], // wall (items pass through)
//     [0.9, 0.9, 1.0, 1.0], // moving road
//     [0.9, 0.7, 1.0, 1.0], // sticky road
//     [1.0, 1.0, 1.0, 1.0], // road (alt sfx)
//     [1.0, 0.0, 1.0, 0.8], // sound trigger
//     [0.4, 0.6, 0.4, 0.8], // weak wall
//     [0.8, 0.0, 1.0, 0.8], // effect trigger
//     [1.0, 0.0, 1.0, 0.5], // item state modifier
//     [0.0, 0.6, 0.0, 0.8], // half-pipe invis wall
//     [0.9, 0.9, 1.0, 1.0], // rotating road
//     [0.8, 0.7, 0.8, 1.0], // special wall
//     [0.6, 0.6, 0.6, 1.0], // wall
// ];

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
    [1.0, 0.0, 1.0, 0.4], // sound trigger
    [0.4, 0.6, 0.4, 0.8], // weak wall
    [0.8, 0.0, 1.0, 0.1], // effect trigger
    [1.0, 0.0, 1.0, 0.5], // item state modifier
    [0.0, 0.6, 0.0, 0.8], // half-pipe invis wall
    [0.9, 0.9, 1.0, 1.0], // rotating road
    [0.8, 0.7, 0.8, 1.0], // special wall
    [0.6, 0.6, 0.6, 1.0], // wall
];

pub struct Tri {
    pub vertices: Vec<Vec3>,
}

pub struct KCL {
    pub tri_groups: Vec<Vec<Tri>>,
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

        let mut normals = Vec::new();

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

        let mut tri_groups: Vec<Vec<Tri>> = Vec::new();
        for _ in 0..KCL_COLORS.len() {
            tri_groups.push(Vec::new())
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
            let v2 = vertex + (cross_b * (length / cross_b.dot(*nrm_c)));
            let v3 = vertex + (cross_a * (length / cross_a.dot(*nrm_c)));

            tri_groups[kcl_type].push(Tri {
                vertices: vec![v1, v2, v3],
            });
        }
        Ok(KCL { tri_groups })
    }

    pub fn build_model(&self, context: &Context) -> Vec<Gm<Mesh, PhysicalMaterial>> {
        let mut gms = Vec::new();

        for (i, tri_group) in self.tri_groups.iter().enumerate() {
            let mut positions = Vec::new();
            let mut colors = Vec::new();
            if tri_group.len() == 0 {
                continue;
            }
            let color = Color::from_rgba_slice(&KCL_COLORS[i]);
            for tri in tri_group {
                positions.extend(tri.vertices.clone());
                colors.extend([color; 3]);
            }

            let positions = Positions::F32(positions);

            let mut cpu_mesh = CpuMesh {
                positions,
                colors: Some(colors),
                ..Default::default()
            };
            cpu_mesh.compute_normals();

            let geometry = Mesh::new(&context, &cpu_mesh);

            let material = PhysicalMaterial::new(
                context,
                &PbrMaterial {
                    albedo: color,
                    metallic: 0.7,
                    ..Default::default()
                },
            );

            gms.push(Gm::new(geometry, material));
        }

        gms
    }

    // pub fn build_model(&self, context: &Context) -> Gm<Mesh, ColorMaterial> {
    //     let mut positions = Vec::new();
    //     let mut colors = Vec::new();

    // for (i, tri_group) in self.tri_groups.iter().enumerate() {
    //     for tri in tri_group.iter() {
    //         let color = Color::from_rgba_slice(&KCL_COLORS[i]);
    //         positions.extend(tri.vertices.clone());
    //         colors.extend([color; 3]);
    //     }
    // }

    //     let positions = Positions::F32(positions);

    //     Gm::new(
    //         Mesh::new(
    //             &context,
    //             &CpuMesh {
    //                 positions,
    //                 colors: Some(colors),
    //                 ..Default::default()
    //             },
    //         ),
    //         ColorMaterial::default(),
    //     )
    // }

    // pub fn build_model(&self, context: &Context) -> Vec<Gm<Mesh, PhysicalMaterial>> {
    //     let mut gms = Vec::new();

    //     for (i, tri_group) in self.tri_groups.iter().enumerate() {
    //         for tri in tri_group {
    //             let positions = vec![
    //                 vec3(tri.vertices[0].x, tri.vertices[0].y, tri.vertices[0].z),
    //                 vec3(tri.vertices[1].x, tri.vertices[1].y, tri.vertices[1].z),
    //                 vec3(tri.vertices[2].x, tri.vertices[2].y, tri.vertices[2].z),
    //             ];
    //             let color = Color::from_rgba_slice(&KCL_COLORS[i]);

    //             let positions = Positions::F32(positions);

    //             let mut cpu_mesh = CpuMesh {
    //                 positions,
    //                 colors: Some(vec![color; 3]),
    //                 ..Default::default()
    //             };
    //             cpu_mesh.compute_normals();

    //             let geometry = Mesh::new(&context, &cpu_mesh);

    //             let material = PhysicalMaterial::new(
    //                 context,
    //                 &PbrMaterial {
    //                     albedo: color,
    //                     metallic: 0.7,
    //                     ..Default::default()
    //                 },
    //             );

    //             gms.push(Gm::new(geometry, material));
    //         }
    //     }
    //     gms
    // }
}
