use byteorder::{ReadBytesExt, BE};
use std::io::{self, Read, Seek, SeekFrom};
use three_d::*;

const KCL_COLORS: [[f32; 4]; 32] = [
    [1.0, 1.0, 1.0, 1.0], // Road
    [1.0, 0.9, 0.8, 1.0], // Slippery Road (sand/dirt)
    [0.0, 0.8, 0.0, 1.0], // Weak Off-Road
    [0.0, 0.6, 0.0, 1.0], // Off-Road
    [0.0, 0.4, 0.0, 1.0], // Heavy Off-Road
    [0.8, 0.9, 1.0, 1.0], // Slippery Road (ice)
    [1.0, 0.5, 0.0, 1.0], // Boost Panel
    [1.0, 0.6, 0.0, 1.0], // Boost Ramp
    [1.0, 0.8, 0.0, 1.0], // Slow Ramp
    [0.9, 0.9, 1.0, 0.5], // Item Road
    [0.7, 0.1, 0.1, 1.0], // Solid Fall
    [0.0, 0.5, 1.0, 1.0], // Moving Water
    [0.6, 0.6, 0.6, 1.0], // Wall
    [0.0, 0.0, 0.6, 0.8], // Invisible Wall
    [0.6, 0.6, 0.7, 0.5], // Item Wall
    [0.6, 0.6, 0.6, 1.0], // Wall
    [0.8, 0.0, 0.0, 0.8], // Fall Boundary
    [1.0, 0.0, 0.5, 0.8], // Cannon Activator
    [0.5, 0.0, 1.0, 0.5], // Force Recalculation
    [0.0, 0.3, 1.0, 1.0], // Half-pipe Ramp
    [0.6, 0.6, 0.6, 1.0], // Wall (items pass through)
    [0.9, 0.9, 1.0, 1.0], // Moving Road
    [0.9, 0.7, 1.0, 1.0], // Sticky Road
    [1.0, 1.0, 1.0, 1.0], // Road (alt sfx)
    [1.0, 0.0, 1.0, 0.8], // Sound Trigger
    [0.4, 0.6, 0.4, 0.8], // Weak Wall
    [0.8, 0.0, 1.0, 0.8], // Effect Trigger
    [1.0, 0.0, 1.0, 0.5], // Item State Modifier
    [0.0, 0.6, 0.0, 0.8], // Half-pipe Invis Wall
    [0.9, 0.9, 1.0, 1.0], // Rotating Road
    [0.8, 0.7, 0.8, 1.0], // Special Wall
    [0.6, 0.6, 0.6, 1.0], // Wall
];

pub struct Tri {
    pub vertices: [Vec3; 3],
    pub color: [f32; 4],
}

pub struct KCL {
    pub tris: Vec<Tri>,
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

        let mut tris = Vec::new();

        while rdr.seek(SeekFrom::Current(0))? < offsets[3] as u64 {
            let length = rdr.read_f32::<BE>()?;
            let pos_index = rdr.read_u16::<BE>()? as usize;
            let face_nrm_index = rdr.read_u16::<BE>()? as usize;

            let nrm_a_index = rdr.read_u16::<BE>()? as usize;
            let nrm_b_index = rdr.read_u16::<BE>()? as usize;
            let nrm_c_index = rdr.read_u16::<BE>()? as usize;

            let kcl_flag = rdr.read_u16::<BE>()?;
            // elimanates all the other data apart from the base type
            let kcl_flag_index = kcl_flag & 0x1f;

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
            // let v2 = vertex + &(cross_b.scale(length / cross_b.dot(nrm_c)));
            // let v3 = vertex + &(cross_a.scale(length / cross_a.dot(nrm_c)));

            let color = KCL_COLORS[kcl_flag_index as usize];

            tris.push(Tri {
                vertices: [v1, v2, v3],
                color,
            });
        }

        Ok(KCL { tris })
    }
    //test
    pub fn build_model(&self, context: &Context) -> Vec<Gm<Mesh, ColorMaterial>> {
        let mut gm: Vec<Gm<Mesh, ColorMaterial>> = Vec::new();

        for tri in &self.tris {
            let positions = vec![
                vec3(tri.vertices[0].x, tri.vertices[0].y, tri.vertices[0].z),
                vec3(tri.vertices[1].x, tri.vertices[1].y, tri.vertices[1].z),
                vec3(tri.vertices[2].x, tri.vertices[2].y, tri.vertices[2].z),
            ];

            let mesh = Gm::new(
                Mesh::new(
                    &context,
                    &CpuMesh {
                        positions: Positions::F32(positions),
                        colors: Some(vec![
                            Color {
                                r: (tri.color[0] * 255.) as u8,
                                g: (tri.color[1] * 255.) as u8,
                                b: (tri.color[2] * 255.) as u8,
                                a: (tri.color[3] * 255.) as u8,
                            };
                            3
                        ]),
                        ..Default::default()
                    },
                ),
                ColorMaterial::default(),
            );

            // Construct a model, with a default color material, thereby transferring the mesh data to the GPU
            gm.push(mesh);
        }
        gm
    }

    //pub fn build_model(&self, context: &Context) -> Model<ColorMaterial> {}
}
