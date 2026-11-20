use crate::GltfModelAssetId;
use super::Component;

#[derive(Clone, Debug)]
pub struct GltfModelRoot{
    pub model_id: GltfModelAssetId
}

impl GltfModelRoot{
    pub fn new(model_id: GltfModelAssetId) -> Self{
        Self {
            model_id
        }
    }
}

impl Component for GltfModelRoot{}