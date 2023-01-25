/// This crate implements all types used by the RPC calls, both parameters and results.
/// Author: Davidson Souza
/// License: MIT
pub mod blockchain;
pub mod general;
pub mod transaction;

#[derive(Debug)]
pub enum VerbosityOutput<T> {
    /// Only a hex-encoded result
    Simple(String),
    /// A full output represented as [T]
    Verbose(T),
}

impl<T> VerbosityOutput<T> {
    pub fn get_verbose(self) -> T {
        match self {
            VerbosityOutput::Verbose(output) => output,
            Self::Simple(_) => panic!("get_verbose called in a simple output"),
        }
    }
    pub fn get_simple(self) -> String {
        match self {
            VerbosityOutput::Verbose(_) => panic!("get_simple called in a verbose output"),
            Self::Simple(output) => output,
        }
    }
}
