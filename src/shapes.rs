use crate::model;

#[derive(Debug, Copy, Clone)]
pub struct Box {
    pub min_x: f32,
    pub max_x: f32,

    pub min_y: f32,
    pub max_y: f32,

    pub min_z: f32,
    pub max_z: f32,
}

impl Box {
    pub fn new(length: f32, width: f32, height: f32) -> Self {
        Self {
            max_x: length / 2.0,
            min_x: -length / 2.0,
            max_y: width / 2.0,
            min_y: -width / 2.0,
            max_z: height / 2.0,
            min_z: -height / 2.0,
        }
    }
}

impl From<Box> for model::Mesh {
    fn from(b: Box) -> Self {
        let mut vertices: Vec<model::ModelVertex> = [
            // Top
            ([b.min_x, b.min_y, b.max_z], [0.0, 0.0], [0.0, 0.0, 1.0]),
            ([b.max_x, b.min_y, b.max_z], [1.0, 0.0], [0.0, 0.0, 1.0]),
            ([b.max_x, b.max_y, b.max_z], [1.0, 1.0], [0.0, 0.0, 1.0]),
            ([b.min_x, b.max_y, b.max_z], [0.0, 1.0], [0.0, 0.0, 1.0]),
            // Bottom
            ([b.min_x, b.max_y, b.min_z], [1.0, 0.0], [0.0, 0.0, -1.0]),
            ([b.max_x, b.max_y, b.min_z], [0.0, 0.0], [0.0, 0.0, -1.0]),
            ([b.max_x, b.min_y, b.min_z], [0.0, 1.0], [0.0, 0.0, -1.0]),
            ([b.min_x, b.min_y, b.min_z], [1.0, 1.0], [0.0, 0.0, -1.0]),
            // Right
            ([b.max_x, b.min_y, b.min_z], [0.0, 0.0], [1.0, 0.0, 0.0]),
            ([b.max_x, b.max_y, b.min_z], [1.0, 0.0], [1.0, 0.0, 0.0]),
            ([b.max_x, b.max_y, b.max_z], [1.0, 1.0], [1.0, 0.0, 0.0]),
            ([b.max_x, b.min_y, b.max_z], [0.0, 1.0], [1.0, 0.0, 0.0]),
            // Left
            ([b.min_x, b.min_y, b.max_z], [1.0, 0.0], [-1.0, 0.0, 0.0]),
            ([b.min_x, b.max_y, b.max_z], [0.0, 0.0], [-1.0, 0.0, 0.0]),
            ([b.min_x, b.max_y, b.min_z], [0.0, 1.0], [-1.0, 0.0, 0.0]),
            ([b.min_x, b.min_y, b.min_z], [1.0, 1.0], [-1.0, 0.0, 0.0]),
            // Front
            ([b.max_x, b.max_y, b.min_z], [1.0, 0.0], [0.0, 1.0, 0.0]),
            ([b.min_x, b.max_y, b.min_z], [0.0, 0.0], [0.0, 1.0, 0.0]),
            ([b.min_x, b.max_y, b.max_z], [0.0, 1.0], [0.0, 1.0, 0.0]),
            ([b.max_x, b.max_y, b.max_z], [1.0, 1.0], [0.0, 1.0, 0.0]),
            // Back
            ([b.max_x, b.min_y, b.max_z], [0.0, 0.0], [0.0, -1.0, 0.0]),
            ([b.min_x, b.min_y, b.max_z], [1.0, 0.0], [0.0, -1.0, 0.0]),
            ([b.min_x, b.min_y, b.min_z], [1.0, 1.0], [0.0, -1.0, 0.0]),
            ([b.max_x, b.min_y, b.min_z], [0.0, 1.0], [0.0, -1.0, 0.0]),
        ]
        .into_iter()
        .map(Into::into)
        .collect();

        let indices = vec![
            0, 1, 2, 2, 3, 0, // top
            4, 5, 6, 6, 7, 4, // bottom
            8, 9, 10, 10, 11, 8, // right
            12, 13, 14, 14, 15, 12, // left
            16, 17, 18, 18, 19, 16, // front
            20, 21, 22, 22, 23, 20, // back
        ];

        model::ModelVertex::calc_tangents_and_bitangents(&mut vertices, &indices);

        Self {
            name: "box".to_string(),
            vertices,
            indices,
            material: 0,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Quad {
    pub size: (f32, f32),
    pub flip: bool,
}

impl Quad {
    pub fn new(size: (f32, f32)) -> Self {
        Self { size, flip: false }
    }

    pub fn flipped(size: (f32, f32)) -> Self {
        Self { size, flip: true }
    }
}

impl From<Quad> for model::Mesh {
    fn from(quad: Quad) -> Self {
        let extent_x = quad.size.0 / 2.0;
        let extent_y = quad.size.1 / 2.0;

        let top_left = (-extent_x, extent_y);
        let top_right = (extent_x, extent_y);
        let bot_left = (-extent_x, -extent_y);
        let bot_right = (extent_x, -extent_y);
        let vertices = if quad.flip {
            [
                ([bot_right.0, bot_right.1, 0.0], [1.0, 1.0], [0.0, 0.0, 1.0]),
                ([top_right.0, top_right.1, 0.0], [1.0, 0.0], [0.0, 0.0, 1.0]),
                ([top_left.0, top_left.1, 0.0], [0.0, 0.0], [0.0, 0.0, 1.0]),
                ([bot_left.0, bot_left.1, 0.0], [0.0, 1.0], [0.0, 0.0, 1.0]),
            ]
        } else {
            [
                ([bot_left.0, bot_left.1, 0.0], [0.0, 1.0], [0.0, 0.0, 1.0]),
                ([top_left.0, top_left.1, 0.0], [0.0, 0.0], [0.0, 0.0, 1.0]),
                ([top_right.0, top_right.1, 0.0], [1.0, 0.0], [0.0, 0.0, 1.0]),
                ([bot_right.0, bot_right.1, 0.0], [1.0, 1.0], [0.0, 0.0, 1.0]),
            ]
        };
        let mut vertices: Vec<model::ModelVertex> = vertices.into_iter().map(Into::into).collect();

        let indices = vec![0, 2, 1, 0, 3, 2];

        model::ModelVertex::calc_tangents_and_bitangents(&mut vertices, &indices);

        Self {
            name: "quad".to_string(),
            vertices,
            indices,
            material: 0,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Plane {
    pub size: f32,
}

impl Plane {
    pub fn new(size: f32) -> Self {
        Self { size }
    }
}

impl From<Plane> for model::Mesh {
    fn from(plane: Plane) -> Self {
        let extent = plane.size / 2.0;

        let mut vertices: Vec<model::ModelVertex> = [
            ([extent, 0.0, -extent], [1.0, 1.0], [0.0, 1.0, 0.0]),
            ([extent, 0.0, extent], [1.0, 0.0], [0.0, 1.0, 0.0]),
            ([-extent, 0.0, extent], [0.0, 0.0], [0.0, 1.0, 0.0]),
            ([-extent, 0.0, -extent], [0.0, 1.0], [0.0, 1.0, 0.0]),
        ]
        .into_iter()
        .map(Into::into)
        .collect();

        let indices = vec![0, 2, 1, 0, 3, 2];

        model::ModelVertex::calc_tangents_and_bitangents(&mut vertices, &indices);

        Self {
            name: "plane".to_string(),
            vertices,
            indices,
            material: 0,
        }
    }
}
