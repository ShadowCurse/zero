pub mod camera;
#[cfg(feature = "egui")]
pub mod egui;
pub mod gbuffer;
pub mod light;
pub mod material;
pub mod mesh;
pub mod model;
pub mod render;
pub mod shadow_map;
pub mod shapes;
pub mod skybox;
pub mod texture;
pub mod texture_buffer;
pub mod transform;
pub mod utils;

pub mod prelude {
    use super::*;

    pub use camera::*;
    pub use gbuffer::*;
    pub use light::*;
    pub use material::*;
    pub use mesh::*;
    pub use model::*;
    pub use render::prelude::*;
    pub use shadow_map::*;
    pub use shapes::*;
    pub use skybox::*;
    pub use texture::*;
    pub use texture_buffer::*;
    pub use transform::*;
    pub use utils::*;

    pub use cgmath_imports::*;
    pub use wgpu;
    pub use winit;
}

pub mod cgmath_imports {
    pub use cgmath::{
        ortho, perspective, Deg, InnerSpace, Matrix3, Matrix4, Point3, Quaternion, Rad, Rotation3,
        Vector2, Vector3,
    };
}
