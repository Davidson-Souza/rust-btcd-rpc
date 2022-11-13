pub mod client;
pub mod error;

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
