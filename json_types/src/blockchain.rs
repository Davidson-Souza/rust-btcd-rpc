use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct GetUtreexoProofResult {
    pub proofhashes: Vec<String>,
    pub targethashes: Vec<String>,
    pub prooftargets: Vec<u64>,
    pub hex: String,
}
