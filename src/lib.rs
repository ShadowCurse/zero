pub mod camera;
pub mod deffered_rendering;
pub mod light;
pub mod material;
pub mod mesh;
pub mod model;
pub mod render;
pub mod shadow_map;
pub mod shapes;
pub mod skybox;
pub mod texture;
pub mod transform;
pub mod utils;

pub mod prelude {
    use super::*;

    pub use camera::*;
    pub use deffered_rendering::*;
    pub use light::*;
    pub use material::*;
    pub use mesh::*;
    pub use model::*;
    pub use render::prelude::*;
    pub use shadow_map::*;
    pub use shapes::*;
    pub use skybox::*;
    pub use texture::*;
    pub use transform::*;

    pub use cgmath_imports::*;
}

pub mod cgmath_imports {
    pub use cgmath::{
        ortho, perspective, Deg, InnerSpace, Matrix3, Matrix4, Point3, Quaternion, Rad, Rotation3,
        Vector2, Vector3,
    };
}
