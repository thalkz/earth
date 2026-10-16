use crate::assets::handle::ModelHandle;
use crate::assets::loader::load_model;
use crate::assets::model::Model;

#[derive(Default)]
pub struct AssetServer {
    models: Vec<Model>,
}

impl AssetServer {
    pub fn new() -> Self {
        Self { models: Vec::new() }
    }

    pub async fn load_model(&mut self, file_name: &str) -> anyhow::Result<ModelHandle> {
        let model = load_model(file_name).await?;
        let handle = ModelHandle(self.models.len());
        self.models.push(model);
        Ok(handle)
    }

    pub fn get_model(&self, handle: ModelHandle) -> Option<&Model> {
        self.models.get(handle.0)
    }
}
