use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct GetUtreexoProofResult {
    pub proofhashes: Vec<String>,
    #[serde(rename = "rememberindexes")]
    pub remember_indexes: Vec<String>,
    pub targethashes: Vec<String>,
    #[serde(rename = "targetpreimages")]
    pub target_preimages: Vec<String>,
    pub prooftargets: Vec<u64>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct GetBlockResult {
    hash: String,
    confirmations: u64,
    strippedsize: u32,
    size: u32,
    weight: u32,
    height: u32,
    version: u32,
    #[serde(rename = "versionHex")]
    version_hex: String,
    merkleroot: String,
    tx: Vec<String>,
    time: u32,
    nonce: u32,
    bits: String,
    previousblockhash: String,
    nextblockhash: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct GetBlockHeaderResult {
    hash: String,
    confirmations: u32,
    height: u32,
    version: u32,
    #[serde(rename = "versionHex")]
    version_hex: String,
    #[serde(rename = "merkleroot")]
    merkle_root: String,
    time: u32,
    nonce: u32,
    bits: String,
    difficulty: f32,
    previousblockhash: String,
    nextblockhash: Option<String>,
}
