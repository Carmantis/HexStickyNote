use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiDigest {
    pub id: String,
    pub period_type: String,
    pub period_key: String,
    pub content: String,
    pub model: Option<String>,
    pub created_at: i64,
}
