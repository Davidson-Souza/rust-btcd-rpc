pub mod client;
pub mod error;
pub use json_types;
/// Some RPCs requires a given block, usually as a hash. But we might only have a height.
/// In order to save some time while programming, instead of asking for a hash and then
/// asking what you need, this API allows asking by hash or by height, and we take care
/// of the round-trips internally.
pub enum QueryBlock {
    /// This means: "I'm referencing block X, where X is the height of an existing block"
    ByHeight(usize),
    /// This means: "I'm referencing block whose hash is Y"
    ByHash(String),
}

#[macro_export]
macro_rules! impl_verbosity_bool {
    ($self: ident, $cmd: literal, $params: expr, $verbosity: ident) => {
        match $verbosity {
            true => {
                let verbosity = serde_json::to_value(true)?;
                let rpc_res = $self.call($cmd, &[$params, verbosity])?;
                Ok(VerbosityOutput::Verbose(rpc_res))
            }
            false => {
                let verbosity = serde_json::to_value(false)?;
                let rpc_res = $self.call($cmd, &[$params, verbosity])?;
                Ok(VerbosityOutput::Simple(rpc_res))
            }
        }
    };
}
#[macro_export]
macro_rules! impl_verbosity_level {
    ($self: ident, $cmd: literal, $params: expr, $verbosity: ident) => {
        match $verbosity {
            true => {
                let verbosity = serde_json::to_value(1)?;
                let rpc_res = $self.call($cmd, &[$params, verbosity])?;
                Ok(VerbosityOutput::Verbose(rpc_res))
            }
            false => {
                let verbosity = serde_json::to_value(0)?;
                let rpc_res = $self.call($cmd, &[$params, verbosity])?;
                Ok(VerbosityOutput::Simple(rpc_res))
            }
        }
    };
}