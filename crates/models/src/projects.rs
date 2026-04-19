use poem_openapi::Object;
use serde::{Deserialize, Serialize};

#[derive(Object, Serialize, Deserialize, Clone, Debug)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: String,
    pub strategy: String,
    pub created_at: String,
    pub updated_at: String,
    pub symbols: Vec<String>,
    pub interval: String,
    pub range: String,
    pub prepost: bool,
}
