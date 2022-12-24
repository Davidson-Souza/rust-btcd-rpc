use serde_json::Value;

#[derive(Debug)]
pub enum UtreexodError {
    JsonRpcError(Value),
    ReqwestError(reqwest::Error),
    DeserializationError(serde_json::Error),
    EmptyResponseFromServer,
}

impl From<reqwest::Error> for UtreexodError {
    fn from(error: reqwest::Error) -> Self {
        UtreexodError::ReqwestError(error)
    }
}
impl From<serde_json::Error> for UtreexodError {
    fn from(error: serde_json::Error) -> Self {
        UtreexodError::DeserializationError(error)
    }
}
