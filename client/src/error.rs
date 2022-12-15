use jsonrpc::simple_http;
#[derive(Debug)]
pub enum UtreexodError {
    JsonRpcError(jsonrpc::Error),
    SimpleHttpError(simple_http::Error),
    DeserializationError(serde_json::Error),
    EmptyResponseFromServer,
}

impl From<jsonrpc::Error> for UtreexodError {
    fn from(error: jsonrpc::Error) -> Self {
        UtreexodError::JsonRpcError(error)
    }
}

impl From<simple_http::Error> for UtreexodError {
    fn from(error: simple_http::Error) -> Self {
        UtreexodError::SimpleHttpError(error)
    }
}
impl From<serde_json::Error> for UtreexodError {
    fn from(error: serde_json::Error) -> Self {
        UtreexodError::DeserializationError(error)
    }
}
