use crate::error::UtreexodError;
use crate::{impl_verbosity_bool, impl_verbosity_level};
use json_types::blockchain::{GetBlockHeaderResult, GetBlockResult};
use json_types::transaction::{BestBlock, VerboseGetRawTransactionResult};
use json_types::{
    self,
    transaction::{DecodeRawTransactionResult, Outpoint, Recipient},
};
use json_types::{general::*, VerbosityOutput};

#[cfg(feature = "utreexod")]
use json_types::blockchain::GetUtreexoProofResult;
use jsonrpc::{self, Client};
use serde_json::{from_value, Value};
pub struct BTCDClient(Client);

impl BTCDClient {
    fn call<T: for<'a> serde::de::Deserialize<'a>>(&self, cmd: &str, args: &[Value]) -> Result<T> {
        let raw_args: Vec<_> = args
            .into_iter()
            .map(|a| from_value(a.clone()))
            .filter(|a| a.is_ok())
            .map(|a| a.unwrap())
            .collect();

        // Builds a request
        let req = self.0.build_request(&cmd, &raw_args);
        // Sends it and collects the response in `resp`
        let resp = self.0.send_request(req)?;
        if let Some(error) = resp.error {
            return Err(UtreexodError::JsonRpcError(jsonrpc::Error::Rpc(error)));
        }
        Ok(serde_json::from_str::<T>(
            resp.result.unwrap_or_default().get(),
        )?)
    }

    pub fn new(cfg: BTCDConfigs) -> Result<BTCDClient> {
        let client = Client::simple_http(
            format!(
                "{}:{}",
                cfg.host.expect("No hostname provided"),
                cfg.port.unwrap_or(8332)
            )
            .as_str(),
            cfg.username,
            cfg.password,
        )?;
        Ok(BTCDClient(client))
    }
}

type Result<T> = std::result::Result<T, UtreexodError>;

pub trait BtcdRpc {
    /// Calls an arbitrary command. `cmd` is a static str ref of the intended rpc.
    /// `Args` is a slice of [serde_json::Value], there may be zero or more args, depending
    /// on the rpc.
    /// Returns a generic type [T], for a [T] implementing [Deserialize](serde::de::Deserialize).
    /// This method is not intended to be used manually, but internally by functions representing
    /// a given rpc.
    fn call<T: for<'a> serde::de::Deserialize<'a>>(
        &self,
        command: &'static str,
        args: &[Value],
    ) -> Result<T>;
    /// Returns a JSON object containing various state info. For exact contents, see [GetInfoResult]
    /// # Example
    /// ```
    /// use btcd_rpc::client::{BTCDConfigs, BTCDClient, BtcdRpc};
    /// let config = BTCDConfigs::new(
    ///     false,
    ///     Some("SomeUsername".into()),
    ///     Some("CorrectHorseBattleStaple".into()),
    ///     Some("localhost".into()),
    ///     Some(38332),
    /// );
    ///
    /// let client = BTCDClient::new(config).unwrap();
    /// assert!(client.getinfo().is_ok());
    /// ```
    fn getinfo(&self) -> Result<GetInfoResult> {
        self.call("getinfo", &[])
    }
    /// Returns the hash of a block, given it's height.
    /// ```
    /// use btcd_rpc::client::{BTCDConfigs, BTCDClient, BtcdRpc};
    /// let config = BTCDConfigs::new(
    ///     false,
    ///     Some("SomeUsername".into()),
    ///     Some("CorrectHorseBattleStaple".into()),
    ///     Some("localhost".into()),
    ///     Some(38332),
    /// );
    ///
    /// let client = BTCDClient::new(config).unwrap();
    /// // This is a signet block
    /// assert_eq!(client.getblockhash(0).unwrap(), String::from("00000008819873e925422c1ff0f99f7cc9bbb232af63a077a480a3633bee1ef6"));
    /// ```
    fn getblockhash(&self, height: usize) -> Result<String> {
        let height = Value::from(height);
        self.call("getblockhash", &[height])
    }
    #[cfg(feature = "utreexod")]
    /// Returns the Batch Proof for a given block
    /// ```
    /// use btcd_rpc::client::{BTCDClient, BtcdRpc};
    /// let client = BTCDClient::new().unwrap();
    /// // This is a signet block
    /// // assert!(client.getblockhash(0).unwrap(), String::from("00000008819873e925422c1ff0f99f7cc9bbb232af63a077a480a3633bee1ef6"));
    /// ```
    fn getutreexoproof(&self, hash: String, verbosity: bool) -> Result<VerbosityOutput<GetUtreexoProofResult>> {
        let hash = Value::from(hash);
        impl_verbosity_level!(self, "getutreexoproof", hash, verbosity)
    }
    /// This command is useful for managing peers in your node. You can add, remove or list
    /// manually added peers. Note that added peers have different rules than automatic ones,
    /// see btcd's documentation for more details.
    /// ```
    /// use btcd_rpc::client::{BTCDConfigs, BTCDClient, BtcdRpc};
    /// let config = BTCDConfigs::new(
    ///     false,
    ///     Some("SomeUsername".into()),
    ///     Some("CorrectHorseBattleStaple".into()),
    ///     Some("localhost".into()),
    ///     Some(38332),
    /// );
    ///
    /// let client = BTCDClient::new(config).unwrap();
    /// // This is a signet block
    /// assert!(client.addnode(&"127.0.0.1", &"add").is_ok());
    /// ```
    fn addnode(&self, addr: &str, cmd: &str) -> Result<()> {
        let addr = Value::from(addr);
        let cmd = Value::from(cmd);
        self.call("addnode", &[addr, cmd])
    }
    /// Creates a new unsigned raw transactions sending funds to `destinations`.
    /// This RPC also asks for inputs. Locktime is the nLocktime which the transaction
    /// must obey to be valid.
    fn createrawtransaction(
        &self,
        inputs: Vec<Outpoint>,
        destinations: Vec<Recipient>,
        locktime: usize,
    ) -> Result<()> {
        let inputs = serde_json::to_value(inputs)?;
        let destinations = serde_json::to_value(destinations)?;
        let locktime = Value::from(locktime);

        self.call("createrawtransaction", &[inputs, destinations, locktime])
    }
    /// Dynamically changes the debug logging level. The levelspec can either a debug level
    /// or of the form: <subsystem>=<level>,<subsystem2>=<level2>,... The valid debug levels are trace,
    /// debug, info, warn, error, and critical.
    /// The valid subsystems are AMGR, ADXR, BCDB, BMGR, BTCD, CHAN, DISC, PEER, RPCS, SCRP, SRVR, and TXMP.
    /// Finally the keyword 'show' will return a list of the available subsystems.
    fn debuglevel(&self, levelspec: LevelSpec) -> Result<()> {
        let levelspec = match levelspec {
            LevelSpec::Global(val) => serde_json::to_value(val),
            LevelSpec::Subsystem(values) => {
                let mut spec = String::new();
                for (system, level) in values {
                    spec = format!("{}={},", system.to_string(), level.to_string());
                }
                spec.pop(); // This removes any trailing comma
                serde_json::to_value(spec)
            }
        }?;

        self.call("debuglevel", &[levelspec])
    }
    /// Returns an object representing the provided serialized, hex-encoded transaction.
    /// For documentation of the parsed type, see [DecodeRawTransactionResult].
    fn decoderawtransaction(&self, hextx: String) -> Result<DecodeRawTransactionResult> {
        let hextx = serde_json::to_value(hextx)?;
        self.call("decoderawtransaction", &[hextx])
    }
    /// Returns the current best-known block' hash and height
    fn getbestblock(&self) -> Result<BestBlock> {
        self.call("getbestblock", &[])
    }
    /// Returns only the hash of the best known block
    fn getbestblockhash(&self) -> Result<String> {
        self.call("getbestblockhash", &[])
    }
    /// Returns how many blocks we known about
    fn getblockcount(&self) -> Result<usize> {
        self.call("getblockcount", &[])
    }
    /// Broadcast a hex-encoded transaction to the network
    fn sendrawtransaction(&self, rawtx: String) -> Result<String> {
        let rawtx = serde_json::to_value(rawtx)?;
        self.call("sendrawtransaction", &[rawtx])
    }
    /// Estimates the required fee for a given expected confirmation time, in blocks
    fn estimatefee(&self, blocks: u32) -> Result<f64> {
        let blocks = serde_json::to_value(blocks)?;
        self.call("estimatefee", &[blocks])
    }
    /// Returns the raw transaction, given it's hash
    /// The verbosity level determines which information is returned. True means all transaction
    /// data plus some additional information, like hash and data of the block this transaction got included.
    /// False only returns a hex-encoded transaction.
    fn getrawtransaction(
        &self,
        transaction_hash: String,
        verbosity: bool,
    ) -> Result<VerbosityOutput<VerboseGetRawTransactionResult>> {
        let transaction_hash = serde_json::to_value(transaction_hash)?;

        impl_verbosity_level!(self, "getrawtransaction", transaction_hash, verbosity)
    }
    /// Returns a block, given it's hash
    fn getblock(&self, hash: String, verbosity: bool) -> Result<VerbosityOutput<GetBlockResult>> {
        let hash = serde_json::to_value(hash)?;
        impl_verbosity_level!(self, "getblock", hash, verbosity)
    }
    /// Returns the block's header
    fn getblockheader(
        &self,
        hash: String,
        verbosity: bool,
    ) -> Result<VerbosityOutput<GetBlockHeaderResult>> {
        let hash = serde_json::to_value(hash)?;
        impl_verbosity_bool!(self, "getblockheader", hash, verbosity)
    }
}
impl BtcdRpc for BTCDClient {
    fn call<T: for<'a> serde::de::Deserialize<'a>>(
        &self,
        command: &'static str,
        args: &[Value],
    ) -> Result<T> {
        self.call(command, args)
    }
}

pub struct BTCDConfigs {
    tls: bool,
    username: Option<String>,
    password: Option<String>,
    host: Option<String>,
    port: Option<usize>,
}

impl BTCDConfigs {
    pub fn new(
        tls: bool,
        username: Option<String>,
        password: Option<String>,
        host: Option<String>,
        port: Option<usize>,
    ) -> BTCDConfigs {
        BTCDConfigs {
            tls,
            username,
            password,
            host,
            port,
        }
    }
    pub fn set(mut self, opt: Options) {
        match opt {
            Options::Username(username) => self.username = Some(username),
            Options::Password(password) => self.password = Some(password),
            Options::Hostname(hostname) => self.host = Some(hostname),
            Options::Port(port) => self.port = Some(port),
            Options::TLS(tls) => self.tls = tls,
        }
    }
}

pub enum Options {
    Username(String),
    Password(String),
    Hostname(String),
    Port(usize),
    TLS(bool),
}
pub enum Network {
    Bitcoin,
    Testnet,
    Signet,
    Regtest,
}
#[cfg(test)]
mod test {
    #[test]
    fn test_basic_command() {
        use super::{BTCDClient, BTCDConfigs, BtcdRpc};
        let config = BTCDConfigs::new(
            false,
            Some("SomeUsername".into()),
            Some("CorrectHorseBattleStaple".into()),
            Some("localhost".into()),
            Some(38332),
        );

        let client = BTCDClient::new(config).unwrap();
        let res = client.getinfo();

        assert!(res.is_ok());
    }
    #[test]
    fn test_get_block_hash() {
        use super::{BTCDClient, BTCDConfigs, BtcdRpc};
        let config = BTCDConfigs::new(
            false,
            Some("SomeUsername".into()),
            Some("CorrectHorseBattleStaple".into()),
            Some("localhost".into()),
            Some(38332),
        );

        let client = BTCDClient::new(config).unwrap();
        let hash = client.getblockhash(0);
        assert_eq!(
            hash.unwrap(),
            String::from("00000008819873e925422c1ff0f99f7cc9bbb232af63a077a480a3633bee1ef6")
        );
    }
    #[test]
    fn test_decoderawtransaction() {
        use super::{BTCDClient, BTCDConfigs, BtcdRpc};

        let config = BTCDConfigs::new(
            false,
            Some("SomeUsername".into()),
            Some("CorrectHorseBattleStaple".into()),
            Some("localhost".into()),
            Some(38332),
        );

        let client = BTCDClient::new(config).unwrap();
        let raw_transaction = "020000000001014224f25afde5dd27f5b6c2ebaa6732ba0d2ceabd1e4046eb1c1e59eecda554fa0100000000feffffff0240420f0000000000225120ac2270ec1eac011410b7b0cab2022bcaba9061fe62008fed43b884b4ba1db783dd077e975d0600001600145e1d306e58c306e5e84cfddb4152a73a12d33d4602453042021f19f6cc924d0bed97024a7eac8357a1ddc30443819e1317a4d474757a61fee4021f0b1fcbe9dbb52016f660acae8b61f73cb003cd16119b0a8a453ec01a74231e012102d56ee7e2a5122db55bed1b21ed76078441a8075b27ffe13a8577a93d81c8ccc0f3c70100";
        let raw_transaction = client
            .decoderawtransaction(raw_transaction.into())
            .expect("RPC fail");
        assert_eq!(
            raw_transaction.txid,
            "cff4b318750d00516dbcdc19694a66a377bd024ee2d8a07cec0e9326cb602285".to_string()
        );
        assert_eq!(raw_transaction.version, 2);
        assert_eq!(raw_transaction.locktime, 116723);
        assert_eq!(raw_transaction.vin.len(), 1);
        assert_eq!(raw_transaction.vout.len(), 2);
    }
    #[cfg(feature = "utreexod")]
    #[test]
    fn test_getutreexoproof() {
        use super::{BTCDClient, BtcdRpc, BTCDConfigs};

        let config = BTCDConfigs::new(
            false,
            Some("SomeUsername".into()),
            Some("CorrectHorseBattleStaple".into()),
            Some("localhost".into()),
            Some(38332),
        );

        let client = BTCDClient::new(config).unwrap();
        let hash = client.getblockhash(10);
        let proof = client.getutreexoproof(hash.unwrap(), true);
        assert!(proof.is_ok())
    }
    #[test]
    fn test_create_config() {
        use super::BTCDConfigs;

        let config = BTCDConfigs::new(
            false,
            Some("SomeUsername".into()),
            Some("CorrectHorseBattleStaple".into()),
            Some("localhost".into()),
            Some(38332),
        );

        assert_eq!(config.tls, false);
        assert_eq!(config.username, Some("SomeUsername".into()));
        assert_eq!(config.password, Some("CorrectHorseBattleStaple".into()));
        assert_eq!(config.host, Some("localhost".into()));
        assert_eq!(config.port, Some(38332));
    }
}
