use serde::{ser::SerializeMap, Deserialize, Serialize};

/// The outpoint used to reference UTXOs.
pub struct Outpoint {
    tx_id: String,
    vout: usize,
}
/// A recipient is a map (address, value) for each destination you need send coins to
pub struct Recipient {
    address: String,
    amount: f64,
}
/// Both outpoint and Recipient are serialized using the map serialization, because it
/// yields a json in the form {"address":amount} and {"hash":outpoint}. This form is required
/// for RPCs like `createrawtransaction`
impl Serialize for Outpoint {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry(&self.tx_id, &self.vout)?;
        map.end()
    }
}

impl Serialize for Recipient {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry(&self.address, &self.amount)?;
        map.end()
    }
}

/// A scriptSig returned form a parsed transaction
#[derive(Debug, Serialize, Deserialize)]
pub struct ParsedScriptSig {
    /// The scriptSig in Bitcoin Script ASM
    pub asm: String,
    /// The same script, but hex-encoded
    pub hex: String,
}
#[derive(Debug, Serialize, Deserialize)]
/// Returned by decoderawtransaction
pub struct RawTxIn {
    pub coinbase: Option<String>,
    pub txid: String,
    pub vout: u64,
    #[serde(rename = "scriptSig")]
    pub script_sig: ParsedScriptSig,
    pub sequence: u64,
    #[serde(rename = "txinwitness")]
    pub tx_in_witness: Vec<String>,
}

/// How we return a scriptPubKey in a input. The script is returned both in asm and hex-encoded
/// form. req_sigs is more useful for multisig setups, and is usually one (single sig). script_type
/// is the well-know name for this script, like pkh (pubkey hash) or wpkh (witness pubkey hash), or
/// non-standard if this is not a well-know script. address is a list of possible bitcoin addresses
/// associated with this script.
#[derive(Debug, Serialize, Deserialize)]
pub struct DecodedScriptPubkey {
    /// Disassembly of the script
    asm: String,
    /// Hex-encoded bytes of the script
    hex: String,
    /// The number of required signatures
    #[serde(rename = "reqSigs")]
    req_sigs: Option<u32>,
    /// The type of the script (e.g. 'pubkeyhash')
    #[serde(rename = "type")]
    script_type: String,
    /// The bitcoin addresses associated with this script
    addresses: Option<Vec<String>>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct ParsedTxOut {
    pub value: f64,
    #[serde(rename = "n")]
    pub index: u64,
    #[serde(rename = "scriptPubKey")]
    pub script: DecodedScriptPubkey,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DecodeRawTransactionResult {
    /// The hash of the transaction
    pub txid: String,
    /// The transaction version
    pub version: u32,
    /// The transaction lock time
    pub locktime: u32,
    /// All transaction's inputs
    pub vin: Vec<RawTxIn>,
    /// All transaction's outputs
    pub vout: Vec<ParsedTxOut>,
}

/// A pair of values referencing the best known block. It contains both a hash and
/// height.
#[derive(Debug, Serialize, Deserialize)]
pub struct BestBlock {
    height: u64,
    hash: String,
}
