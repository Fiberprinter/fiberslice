use glam::{Vec3, Vec4};

use crate::render::Vertex;

pub trait IntoArrayColor {
    fn to_array(&self) -> [f32; 4];
}

impl IntoArrayColor for Vec4 {
    fn to_array(&self) -> [f32; 4] {
        self.to_array()
    }
}

impl IntoArrayColor for [f32; 4] {
    fn to_array(&self) -> [f32; 4] {
        *self
    }
}

impl IntoArrayColor for [f32; 3] {
    fn to_array(&self) -> [f32; 4] {
        [self[0], self[1], self[2], 1.0]
    }
}

impl IntoArrayColor for wgpu::Color {
    fn to_array(&self) -> [f32; 4] {
        [self.r as f32, self.g as f32, self.b as f32, self.a as f32]
    }
}

pub trait Mesh<const V: usize> {
    fn to_triangle_vertices(&self) -> [Vertex; V];

    fn to_triangle_vertices_flipped(&self) -> [Vertex; V] {
        panic!("Not implemented")
    }
}

pub fn construct_triangle_vertices<const T: usize, C: IntoArrayColor>(
    raw: [Vec3; T],
    color: C,
) -> [Vertex; T] {
    let mut vertices = [Vertex::default(); T];
    let color = color.to_array();

    for i in (0..vertices.len()).step_by(3) {
        let v0 = raw[i];
        let v1 = raw[i + 1];
        let v2 = raw[i + 2];

        let normal = (v1 - v0).cross(v2 - v0).normalize();

        vertices[i].position = v0.to_array();
        vertices[i + 1].position = v1.to_array();
        vertices[i + 2].position = v2.to_array();

        vertices[i].color = color;
        vertices[i + 1].color = color;
        vertices[i + 2].color = color;

        vertices[i].normal = normal.to_array();
        vertices[i + 1].normal = normal.to_array();
        vertices[i + 2].normal = normal.to_array();
    }

    vertices
}

pub fn vec3s_into_vertices<C: IntoArrayColor>(v: Vec<Vec3>, color: C) -> Vec<Vertex> {
    let mut vertices = Vec::with_capacity(v.len());

    let color: [f32; 4] = color.to_array();

    for i in (0..v.len()).step_by(3) {
        let v0 = v[i + 2];
        let v1 = v[i + 1];
        let v2 = v[i];

        let normal = (v1 - v0).cross(v2 - v0).normalize();

        vertices.push(Vertex {
            position: v0.to_array(),
            color,
            normal: normal.to_array(),
        });

        vertices.push(Vertex {
            position: v1.to_array(),
            color,
            normal: normal.to_array(),
        });

        vertices.push(Vertex {
            position: v2.to_array(),
            color,
            normal: normal.to_array(),
        });
    }

    vertices
}

#[allow(dead_code)]
pub trait IndexedMesh<const V: usize, const I: usize>: Mesh<V> {
    fn to_triangle_indices(&self) -> [u32; I];
    fn to_triangle_indices_flipped(&self) -> [u32; I] {
        panic!("Not implemented")
    }
}

pub trait WireMesh<const V: usize> {
    fn to_wire_vertices(&self) -> [Vertex; V];
}

pub fn construct_wire_vertices<const T: usize>(raw: [Vec3; T], color: Vec4) -> [Vertex; T] {
    let mut vertices = [Vertex::default(); T];

    for index in 0..vertices.len() {
        let color = color.to_array();

        vertices[index] = Vertex {
            position: raw[index].to_array(),
            color,
            normal: [0.0, 0.0, 0.0],
        };
    }

    vertices
}

#[allow(dead_code)]
pub trait IndexedWireMesh<const V: usize, const I: usize>: WireMesh<V> {
    fn to_wire_indices(&self) -> [u32; I];
}
