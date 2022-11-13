use std::fmt::{self, Display};

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct GetInfoResult {
    version: u64,
    protocolversion: u64,
    blocks: u64,
    timeoffset: u64,
    connections: u32,
    proxy: String,
    difficulty: f64,
    testnet: bool,
    relayfee: f64,
    errors: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum DebugLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Critical,
}
impl Display for DebugLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DebugLevel::Critical => write!(f, "critical"),
            DebugLevel::Trace => write!(f, "trace"),
            DebugLevel::Debug => write!(f, "debug"),
            DebugLevel::Info => write!(f, "info"),
            DebugLevel::Warn => write!(f, "warn"),
            DebugLevel::Error => write!(f, "error"),
        }
    }
}

pub enum LevelSpec {
    Global(DebugLevel),
    Subsystem(Vec<(Subsystem, DebugLevel)>),
}
#[derive(Debug, Serialize, Deserialize)]
pub enum Subsystem {
    AMGR,
    ADXR,
    BCDB,
    BMGR,
    BTCD,
    CHAN,
    DISC,
    PEER,
    RPCS,
    SCRP,
    SRVR,
    TXMP,
}
impl Display for Subsystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Subsystem::AMGR => write!(f, "AMGR"),
            Subsystem::ADXR => write!(f, "ADXR"),
            Subsystem::BCDB => write!(f, "BCDB"),
            Subsystem::BMGR => write!(f, "BMGR"),
            Subsystem::BTCD => write!(f, "BTCD"),
            Subsystem::CHAN => write!(f, "CHAN"),
            Subsystem::DISC => write!(f, "DISC"),
            Subsystem::PEER => write!(f, "PEER"),
            Subsystem::RPCS => write!(f, "RPCS"),
            Subsystem::SCRP => write!(f, "SCRP"),
            Subsystem::SRVR => write!(f, "SRVR"),
            Subsystem::TXMP => write!(f, "TXMP"),
        }
    }
}
