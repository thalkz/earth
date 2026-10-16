mod handle;
mod loader;
mod model;
mod server;

pub use handle::ModelHandle;
#[allow(unused_imports)]
pub use handle::{MaterialHandle, MeshHandle};
#[allow(unused_imports)]
pub use loader::load_model;
pub use model::{Model, ModelVertex, Vertex};
#[allow(unused_imports)]
pub use model::{Material, Mesh};
pub use server::AssetServer;
