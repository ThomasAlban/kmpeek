// this is from bevy_more_shapes: https://github.com/redpandamonium/bevy_more_shapes/
// since it is no longer maintained, I took the cylinder part of it

use bevy::math::Vec3;
use bevy::prelude::Vec2;
use bevy::render::mesh::{Indices, Mesh};
use bevy::render::render_resource::PrimitiveTopology;

// When indexing a mesh we commonly find flat (occupying a 2 dimensional subspace) trapezes.
#[derive(Copy, Clone)]
pub struct FlatTrapezeIndices {
    pub lower_left: u32,
    pub upper_left: u32,
    pub lower_right: u32,
    pub upper_right: u32,
}

impl FlatTrapezeIndices {
    // Triangulate the trapeze
    pub fn generate_triangles(&self, indices: &mut Vec<u32>) {
        indices.push(self.upper_left);
        indices.push(self.upper_right);
        indices.push(self.lower_left);
        indices.push(self.upper_right);
        indices.push(self.lower_right);
        indices.push(self.lower_left);
    }
}

struct MeshData {
    positions: Vec<Vec3>,
    normals: Vec<Vec3>,
    uvs: Vec<Vec2>,
    indices: Vec<u32>,
}

impl MeshData {
    fn new(num_vertices: usize, num_indices: usize) -> Self {
        Self {
            positions: Vec::with_capacity(num_vertices),
            normals: Vec::with_capacity(num_vertices),
            uvs: Vec::with_capacity(num_vertices),
            indices: Vec::with_capacity(num_indices),
        }
    }
}

pub struct Cylinder {
    pub height: f32,
    pub radius_bottom: f32,
    pub radius_top: f32,
    pub radial_segments: u32,
    pub height_segments: u32,
}

impl Default for Cylinder {
    fn default() -> Self {
        Self {
            height: 1.0,
            radius_bottom: 0.5,
            radius_top: 0.5,
            radial_segments: 32,
            height_segments: 1,
        }
    }
}

impl Cylinder {
    /// Create a cylinder where the top and bottom disc have the same radius.
    pub fn new_regular(height: f32, radius: f32, subdivisions: u32) -> Self {
        Self {
            height,
            radius_bottom: radius,
            radius_top: radius,
            radial_segments: subdivisions,
            height_segments: 1,
        }
    }
}

fn add_top(mesh: &mut MeshData, cylinder: &Cylinder) {
    let angle_step = std::f32::consts::TAU / cylinder.radial_segments as f32;
    let base_index = mesh.positions.len() as u32;

    // Center
    let center_pos = Vec3::new(0.0, cylinder.height / 2.0, 0.0);
    mesh.positions.push(center_pos);
    mesh.uvs.push(Vec2::new(0.5, 0.5));
    mesh.normals.push(Vec3::Y);

    // Vertices
    for i in 0..=cylinder.radial_segments {
        let theta = i as f32 * angle_step;
        let x_unit = f32::cos(theta);
        let z_unit = f32::sin(theta);

        let pos = Vec3::new(
            cylinder.radius_top * x_unit,
            cylinder.height / 2.0,
            cylinder.radius_top * z_unit,
        );
        let uv = Vec2::new((z_unit * 0.5) + 0.5, (x_unit * 0.5) + 0.5);

        mesh.positions.push(pos);
        mesh.uvs.push(uv);
        mesh.normals.push(Vec3::Y)
    }

    // Indices
    for i in 0..cylinder.radial_segments {
        mesh.indices.push(base_index);
        mesh.indices.push(base_index + i + 2);
        mesh.indices.push(base_index + i + 1);
    }
}

fn add_bottom(mesh: &mut MeshData, cylinder: &Cylinder) {
    let angle_step = std::f32::consts::TAU / cylinder.radial_segments as f32;
    let base_index = mesh.positions.len() as u32;

    // Center
    let center_pos = Vec3::new(0.0, -cylinder.height / 2.0, 0.0);
    mesh.positions.push(center_pos);
    mesh.uvs.push(Vec2::new(0.5, 0.5));
    mesh.normals.push(-Vec3::Y);

    // Vertices
    for i in 0..=cylinder.radial_segments {
        let theta = i as f32 * angle_step;
        let x_unit = f32::cos(theta);
        let z_unit = f32::sin(theta);

        let pos = Vec3::new(
            cylinder.radius_bottom * x_unit,
            -cylinder.height / 2.0,
            cylinder.radius_bottom * z_unit,
        );
        let uv = Vec2::new((z_unit * 0.5) + 0.5, (x_unit * -0.5) + 0.5);

        mesh.positions.push(pos);
        mesh.uvs.push(uv);
        mesh.normals.push(-Vec3::Y)
    }

    // Indices
    for i in 0..cylinder.radial_segments {
        mesh.indices.push(base_index + i + 1);
        mesh.indices.push(base_index + i + 2);
        mesh.indices.push(base_index);
    }
}

fn add_body(mesh: &mut MeshData, cylinder: &Cylinder) {
    let angle_step = std::f32::consts::TAU / cylinder.radial_segments as f32;
    let base_index = mesh.positions.len() as u32;

    // Vertices
    for i in 0..=cylinder.radial_segments {
        let theta = angle_step * i as f32;
        let x_unit = f32::cos(theta);
        let z_unit = f32::sin(theta);

        // Calculate normal of this segment, it's a straight line so all normals are the same
        let slope = (cylinder.radius_bottom - cylinder.radius_top) / cylinder.height;
        let normal = Vec3::new(x_unit, slope, z_unit).normalize();

        for h in 0..=cylinder.height_segments {
            let height_percent = h as f32 / cylinder.height_segments as f32;
            let y = height_percent * cylinder.height - cylinder.height / 2.0;
            let radius = (1.0 - height_percent) * cylinder.radius_bottom
                + height_percent * cylinder.radius_top;

            let pos = Vec3::new(x_unit * radius, y, z_unit * radius);
            let uv = Vec2::new(i as f32 / cylinder.radial_segments as f32, height_percent);

            mesh.positions.push(pos);
            mesh.normals.push(normal);
            mesh.uvs.push(uv);
        }
    }

    // Indices
    for i in 0..cylinder.radial_segments {
        for h in 0..cylinder.height_segments {
            let segment_base = base_index + (i * (cylinder.height_segments + 1)) + h;
            let indices = FlatTrapezeIndices {
                lower_left: segment_base,
                upper_left: segment_base + 1,
                lower_right: segment_base + cylinder.height_segments + 1,
                upper_right: segment_base + cylinder.height_segments + 2,
            };
            indices.generate_triangles(&mut mesh.indices);
        }
    }
}

impl From<Cylinder> for Mesh {
    fn from(cylinder: Cylinder) -> Self {
        // Input parameter validation
        assert_ne!(
            cylinder.radius_top, 0.0,
            "Radius must not be 0. Use a cone instead."
        );
        assert_ne!(
            cylinder.radius_bottom, 0.0,
            "Radius must not be 0. Use a cone instead."
        );
        assert!(cylinder.radius_bottom > 0.0, "Must have positive radius.");
        assert!(cylinder.radius_top > 0.0, "Must have positive radius.");
        assert!(
            cylinder.radial_segments > 2,
            "Must have at least 3 subdivisions to close the surface."
        );
        assert!(
            cylinder.height_segments >= 1,
            "Must have at least one height segment."
        );
        assert!(cylinder.height > 0.0, "Must have positive height");

        let num_vertices = (cylinder.radial_segments + 1) * (cylinder.height_segments + 3) + 2;
        // top&bottom + body
        let num_indices = cylinder.radial_segments * 3 * 2
            + cylinder.radial_segments * cylinder.height_segments * 6;

        let mut mesh = MeshData::new(num_vertices as usize, num_indices as usize);

        add_top(&mut mesh, &cylinder);
        add_bottom(&mut mesh, &cylinder);
        add_body(&mut mesh, &cylinder);

        let mut m = Mesh::new(PrimitiveTopology::TriangleList);
        m.insert_attribute(Mesh::ATTRIBUTE_POSITION, mesh.positions);
        m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, mesh.normals);
        m.insert_attribute(Mesh::ATTRIBUTE_UV_0, mesh.uvs);
        m.set_indices(Some(Indices::U32(mesh.indices)));
        m
    }
}
