use serde::{Deserialize, Serialize};

// ============================================================================
// VERTCOIN DOCUMENTATION PROVIDER
// ============================================================================
//
// Vertcoin (VTC) is a peer-to-peer cryptocurrency and software project.
// It is a Bitcoin fork that uses the Verthash proof-of-work algorithm,
// which is designed to be ASIC-resistant and GPU-friendly, promoting
// decentralized mining by individual miners.
//
// Key Features:
// - Verthash Algorithm: Memory-bound, ASIC-resistant mining (requires 1.2GB verthash.dat)
// - 2.5 Minute Blocks: Faster confirmations than Bitcoin
// - 84M Total Supply: Same as Litecoin, with periodic halvings
// - SegWit Enabled: Lightning Network compatible since 2017
// - No Premine: Fair distribution through mining only
// - P2Pool Support: Decentralized pool mining
//
// Network Parameters:
// - RPC Port: 5888 (mainnet)
// - P2P Port: 5889 (mainnet)
// - Address Prefix: V (mainnet), t (testnet)
// - Bech32 Prefix: vtc1 (native SegWit)
//
// ============================================================================

/// Vertcoin technology representation (RPC categories)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertcoinTechnology {
    pub identifier: String,
    pub title: String,
    pub description: String,
    pub url: String,
    pub item_count: usize,
}

/// Category of Vertcoin documentation (RPC, Wallet, Mining)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertcoinCategory {
    pub identifier: String,
    pub title: String,
    pub description: String,
    pub items: Vec<VertcoinCategoryItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertcoinCategoryItem {
    pub name: String,
    pub description: String,
    pub kind: VertcoinMethodKind,
    pub url: String,
}

/// Kind of Vertcoin documentation item
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VertcoinMethodKind {
    /// JSON-RPC method for blockchain operations
    RpcMethod,
    /// Wallet-related RPC method (gRPC/legacy)
    WalletMethod,
    /// Mining-related documentation
    MiningMethod,
    /// General specification or concept
    Specification,
}

impl std::fmt::Display for VertcoinMethodKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RpcMethod => write!(f, "RPC Method"),
            Self::WalletMethod => write!(f, "Wallet Method"),
            Self::MiningMethod => write!(f, "Mining"),
            Self::Specification => write!(f, "Specification"),
        }
    }
}

/// Detailed method documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertcoinMethod {
    pub name: String,
    pub description: String,
    pub kind: VertcoinMethodKind,
    pub url: String,
    pub parameters: Vec<VertcoinParameter>,
    pub returns: Option<VertcoinReturnType>,
    pub examples: Vec<VertcoinExample>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertcoinParameter {
    pub name: String,
    pub param_type: String,
    pub required: bool,
    pub description: String,
    pub default_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertcoinReturnType {
    pub type_name: String,
    pub description: String,
    pub fields: Vec<VertcoinReturnField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertcoinReturnField {
    pub name: String,
    pub field_type: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertcoinExample {
    pub language: String,
    pub code: String,
    pub description: Option<String>,
}

/// Static method index entry (pre-defined for all Vertcoin RPC methods)
#[derive(Debug, Clone)]
pub struct VertcoinMethodIndex {
    pub name: &'static str,
    pub description: &'static str,
    pub kind: VertcoinMethodKind,
    pub category: &'static str,
}

// ============================================================================
// VERTCOIN BLOCKCHAIN RPC METHODS
// Based on Bitcoin Core RPC (Vertcoin is a Bitcoin fork)
// ============================================================================

/// Blockchain-related RPC methods
pub const VERTCOIN_BLOCKCHAIN_METHODS: &[VertcoinMethodIndex] = &[
    VertcoinMethodIndex { name: "getbestblockhash", description: "Returns the hash of the best (tip) block in the longest blockchain", kind: VertcoinMethodKind::RpcMethod, category: "blockchain" },
    VertcoinMethodIndex { name: "getblock", description: "Returns block data for the specified block hash with varying verbosity", kind: VertcoinMethodKind::RpcMethod, category: "blockchain" },
    VertcoinMethodIndex { name: "getblockchaininfo", description: "Returns blockchain state info including chain, blocks, headers, bestblockhash, difficulty, and verification progress", kind: VertcoinMethodKind::RpcMethod, category: "blockchain" },
    VertcoinMethodIndex { name: "getblockcount", description: "Returns the number of blocks in the longest blockchain", kind: VertcoinMethodKind::RpcMethod, category: "blockchain" },
    VertcoinMethodIndex { name: "getblockhash", description: "Returns hash of block at the specified height in the longest blockchain", kind: VertcoinMethodKind::RpcMethod, category: "blockchain" },
    VertcoinMethodIndex { name: "getblockheader", description: "Returns information about a block header", kind: VertcoinMethodKind::RpcMethod, category: "blockchain" },
    VertcoinMethodIndex { name: "getchaintips", description: "Returns information about all known tips in the blockchain, including main chain and orphaned branches", kind: VertcoinMethodKind::RpcMethod, category: "blockchain" },
    VertcoinMethodIndex { name: "getdifficulty", description: "Returns the current Verthash mining difficulty as a multiple of minimum difficulty", kind: VertcoinMethodKind::RpcMethod, category: "blockchain" },
    VertcoinMethodIndex { name: "getmempoolancestors", description: "Returns all in-mempool ancestors of a transaction if it is in the mempool", kind: VertcoinMethodKind::RpcMethod, category: "blockchain" },
    VertcoinMethodIndex { name: "getmempooldescendants", description: "Returns all in-mempool descendants of a transaction if it is in the mempool", kind: VertcoinMethodKind::RpcMethod, category: "blockchain" },
    VertcoinMethodIndex { name: "getmempoolentry", description: "Returns mempool data for given transaction in the mempool", kind: VertcoinMethodKind::RpcMethod, category: "blockchain" },
    VertcoinMethodIndex { name: "getmempoolinfo", description: "Returns details on the active state of the TX memory pool", kind: VertcoinMethodKind::RpcMethod, category: "blockchain" },
    VertcoinMethodIndex { name: "getrawmempool", description: "Returns all transaction IDs in the memory pool", kind: VertcoinMethodKind::RpcMethod, category: "blockchain" },
    VertcoinMethodIndex { name: "gettxout", description: "Returns details about an unspent transaction output", kind: VertcoinMethodKind::RpcMethod, category: "blockchain" },
    VertcoinMethodIndex { name: "gettxoutproof", description: "Returns a hex-encoded proof that a transaction was included in a block", kind: VertcoinMethodKind::RpcMethod, category: "blockchain" },
    VertcoinMethodIndex { name: "gettxoutsetinfo", description: "Returns statistics about the unspent transaction output set", kind: VertcoinMethodKind::RpcMethod, category: "blockchain" },
    VertcoinMethodIndex { name: "preciousblock", description: "Treats a block as if it were received before others with the same work", kind: VertcoinMethodKind::RpcMethod, category: "blockchain" },
    VertcoinMethodIndex { name: "pruneblockchain", description: "Prunes the blockchain up to the specified height or timestamp", kind: VertcoinMethodKind::RpcMethod, category: "blockchain" },
    VertcoinMethodIndex { name: "verifychain", description: "Verifies blockchain database integrity", kind: VertcoinMethodKind::RpcMethod, category: "blockchain" },
    VertcoinMethodIndex { name: "verifytxoutproof", description: "Verifies that a proof points to a transaction in a block", kind: VertcoinMethodKind::RpcMethod, category: "blockchain" },
];

/// Control RPC methods
pub const VERTCOIN_CONTROL_METHODS: &[VertcoinMethodIndex] = &[
    VertcoinMethodIndex { name: "getmemoryinfo", description: "Returns information about memory usage by the node", kind: VertcoinMethodKind::RpcMethod, category: "control" },
    VertcoinMethodIndex { name: "getrpcinfo", description: "Returns details of the RPC server", kind: VertcoinMethodKind::RpcMethod, category: "control" },
    VertcoinMethodIndex { name: "help", description: "Lists all commands or gets help for a specified command", kind: VertcoinMethodKind::RpcMethod, category: "control" },
    VertcoinMethodIndex { name: "logging", description: "Gets and sets the logging configuration", kind: VertcoinMethodKind::RpcMethod, category: "control" },
    VertcoinMethodIndex { name: "stop", description: "Safely stops the Vertcoin server", kind: VertcoinMethodKind::RpcMethod, category: "control" },
    VertcoinMethodIndex { name: "uptime", description: "Returns the total uptime of the server in seconds", kind: VertcoinMethodKind::RpcMethod, category: "control" },
];

/// Mining RPC methods (Verthash specific)
pub const VERTCOIN_MINING_METHODS: &[VertcoinMethodIndex] = &[
    VertcoinMethodIndex { name: "getblocktemplate", description: "Returns data needed to construct a block for Verthash mining", kind: VertcoinMethodKind::MiningMethod, category: "mining" },
    VertcoinMethodIndex { name: "getmininginfo", description: "Returns mining-related information including difficulty, networkhashps, and pooledtx", kind: VertcoinMethodKind::MiningMethod, category: "mining" },
    VertcoinMethodIndex { name: "getnetworkhashps", description: "Returns the estimated network hashes per second for Verthash", kind: VertcoinMethodKind::MiningMethod, category: "mining" },
    VertcoinMethodIndex { name: "prioritisetransaction", description: "Accepts a transaction into the memory pool with a priority/fee delta", kind: VertcoinMethodKind::MiningMethod, category: "mining" },
    VertcoinMethodIndex { name: "submitblock", description: "Submits a new block to the network after mining", kind: VertcoinMethodKind::MiningMethod, category: "mining" },
    VertcoinMethodIndex { name: "submitheader", description: "Decodes and submits the given hexdata as a header to the chain", kind: VertcoinMethodKind::MiningMethod, category: "mining" },
];

/// Network RPC methods
pub const VERTCOIN_NETWORK_METHODS: &[VertcoinMethodIndex] = &[
    VertcoinMethodIndex { name: "addnode", description: "Attempts to add or remove a node from the addnode list, or try a connection once", kind: VertcoinMethodKind::RpcMethod, category: "network" },
    VertcoinMethodIndex { name: "clearbanned", description: "Clears all banned IPs", kind: VertcoinMethodKind::RpcMethod, category: "network" },
    VertcoinMethodIndex { name: "disconnectnode", description: "Disconnects from a specified peer node", kind: VertcoinMethodKind::RpcMethod, category: "network" },
    VertcoinMethodIndex { name: "getaddednodeinfo", description: "Returns information about nodes added using addnode", kind: VertcoinMethodKind::RpcMethod, category: "network" },
    VertcoinMethodIndex { name: "getconnectioncount", description: "Returns the number of connections to other nodes", kind: VertcoinMethodKind::RpcMethod, category: "network" },
    VertcoinMethodIndex { name: "getnettotals", description: "Returns information about network traffic, including total bytes received and sent", kind: VertcoinMethodKind::RpcMethod, category: "network" },
    VertcoinMethodIndex { name: "getnetworkinfo", description: "Returns various state info regarding P2P networking", kind: VertcoinMethodKind::RpcMethod, category: "network" },
    VertcoinMethodIndex { name: "getnodeaddresses", description: "Returns known addresses for potential peer connections", kind: VertcoinMethodKind::RpcMethod, category: "network" },
    VertcoinMethodIndex { name: "getpeerinfo", description: "Returns data about each connected network node", kind: VertcoinMethodKind::RpcMethod, category: "network" },
    VertcoinMethodIndex { name: "listbanned", description: "Lists all banned IPs/Subnets", kind: VertcoinMethodKind::RpcMethod, category: "network" },
    VertcoinMethodIndex { name: "ping", description: "Requests a ping be sent to all other nodes to measure latency", kind: VertcoinMethodKind::RpcMethod, category: "network" },
    VertcoinMethodIndex { name: "setban", description: "Adds or removes an IP/Subnet from the banned list", kind: VertcoinMethodKind::RpcMethod, category: "network" },
    VertcoinMethodIndex { name: "setnetworkactive", description: "Enables or disables all P2P network activity", kind: VertcoinMethodKind::RpcMethod, category: "network" },
];

/// Raw transaction RPC methods
pub const VERTCOIN_RAWTRANSACTION_METHODS: &[VertcoinMethodIndex] = &[
    VertcoinMethodIndex { name: "combinepsbt", description: "Combines multiple partially signed Vertcoin transactions into one", kind: VertcoinMethodKind::RpcMethod, category: "rawtransactions" },
    VertcoinMethodIndex { name: "combinerawtransaction", description: "Combines multiple partially signed transactions into one", kind: VertcoinMethodKind::RpcMethod, category: "rawtransactions" },
    VertcoinMethodIndex { name: "converttopsbt", description: "Converts a network serialized transaction to a PSBT", kind: VertcoinMethodKind::RpcMethod, category: "rawtransactions" },
    VertcoinMethodIndex { name: "createpsbt", description: "Creates a PSBT with the given inputs and outputs", kind: VertcoinMethodKind::RpcMethod, category: "rawtransactions" },
    VertcoinMethodIndex { name: "createrawtransaction", description: "Creates a transaction spending given inputs and creating new outputs", kind: VertcoinMethodKind::RpcMethod, category: "rawtransactions" },
    VertcoinMethodIndex { name: "decodepsbt", description: "Returns a JSON object representing the serialized, base64-encoded PSBT", kind: VertcoinMethodKind::RpcMethod, category: "rawtransactions" },
    VertcoinMethodIndex { name: "decoderawtransaction", description: "Returns a JSON object representing the serialized, hex-encoded transaction", kind: VertcoinMethodKind::RpcMethod, category: "rawtransactions" },
    VertcoinMethodIndex { name: "decodescript", description: "Decodes a hex-encoded script", kind: VertcoinMethodKind::RpcMethod, category: "rawtransactions" },
    VertcoinMethodIndex { name: "finalizepsbt", description: "Finalizes the inputs of a PSBT", kind: VertcoinMethodKind::RpcMethod, category: "rawtransactions" },
    VertcoinMethodIndex { name: "fundrawtransaction", description: "Adds inputs to a transaction until it has enough value to meet its out value", kind: VertcoinMethodKind::RpcMethod, category: "rawtransactions" },
    VertcoinMethodIndex { name: "getrawtransaction", description: "Returns the raw transaction data for a given transaction ID", kind: VertcoinMethodKind::RpcMethod, category: "rawtransactions" },
    VertcoinMethodIndex { name: "sendrawtransaction", description: "Submits a raw transaction to the network", kind: VertcoinMethodKind::RpcMethod, category: "rawtransactions" },
    VertcoinMethodIndex { name: "signrawtransactionwithkey", description: "Signs inputs for a raw transaction using provided private keys", kind: VertcoinMethodKind::RpcMethod, category: "rawtransactions" },
    VertcoinMethodIndex { name: "testmempoolaccept", description: "Tests whether raw transactions would be accepted by mempool", kind: VertcoinMethodKind::RpcMethod, category: "rawtransactions" },
];

/// Wallet RPC methods
pub const VERTCOIN_WALLET_METHODS: &[VertcoinMethodIndex] = &[
    VertcoinMethodIndex { name: "abandontransaction", description: "Marks an in-wallet transaction as abandoned, allowing its inputs to be respent", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "abortrescan", description: "Stops the current wallet rescan", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "addmultisigaddress", description: "Adds a multisignature address to the wallet", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "backupwallet", description: "Safely copies the wallet file to the destination path", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "bumpfee", description: "Bumps the fee of a transaction, replacing it with a new transaction", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "createwallet", description: "Creates and loads a new wallet", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "dumpprivkey", description: "Reveals the private key corresponding to a Vertcoin address", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "dumpwallet", description: "Dumps all wallet keys in a human-readable format to a file", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "encryptwallet", description: "Encrypts the wallet with a passphrase", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "getaddressesbylabel", description: "Returns the list of addresses assigned to a label", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "getaddressinfo", description: "Returns information about a Vertcoin address", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "getbalance", description: "Returns the total available balance in the wallet", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "getbalances", description: "Returns an object with all balances in VTC", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "getnewaddress", description: "Returns a new Vertcoin address for receiving payments", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "getrawchangeaddress", description: "Returns a new address for receiving change", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "getreceivedbyaddress", description: "Returns the total amount received by an address", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "getreceivedbylabel", description: "Returns the total amount received by addresses with a specific label", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "gettransaction", description: "Returns detailed information about an in-wallet transaction", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "getunconfirmedbalance", description: "Returns the server's total unconfirmed balance", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "getwalletinfo", description: "Returns information about the loaded wallet", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "importaddress", description: "Adds an address to watch for incoming transactions", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "importmulti", description: "Imports addresses/scripts with rescan support", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "importprivkey", description: "Imports a private key and optionally rescans the wallet", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "importprunedfunds", description: "Imports funds without rescan", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "importpubkey", description: "Adds a public key to watch for incoming transactions", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "importwallet", description: "Imports keys from a wallet dump file", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "keypoolrefill", description: "Fills the keypool with new keys", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "listaddressgroupings", description: "Lists groups of addresses with common ownership", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "listlabels", description: "Returns a list of all labels in the wallet", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "listlockunspent", description: "Returns a list of temporarily unspendable outputs", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "listreceivedbyaddress", description: "Lists balances by receiving address", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "listreceivedbylabel", description: "Lists received transactions grouped by label", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "listsinceblock", description: "Returns all transactions since a specific block", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "listtransactions", description: "Returns up to 'count' most recent transactions", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "listunspent", description: "Returns array of unspent transaction outputs", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "listwalletdir", description: "Returns a list of wallets in the wallet directory", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "listwallets", description: "Returns a list of currently loaded wallets", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "loadwallet", description: "Loads a wallet from a wallet file or directory", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "lockunspent", description: "Temporarily locks or unlocks specified transaction outputs", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "removeprunedfunds", description: "Deletes the specified transaction from the wallet", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "rescanblockchain", description: "Rescans the blockchain for wallet transactions", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "sendmany", description: "Sends VTC to multiple addresses in a single transaction", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "sendtoaddress", description: "Sends VTC to a given address", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "sethdseed", description: "Sets the HD seed for the wallet", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "setlabel", description: "Sets the label associated with an address", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "settxfee", description: "Sets the transaction fee per kB", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "setwalletflag", description: "Changes the state of wallet flags", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "signmessage", description: "Signs a message with the private key of an address", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "signrawtransactionwithwallet", description: "Signs inputs for a raw transaction using wallet keys", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "unloadwallet", description: "Unloads a wallet", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "walletcreatefundedpsbt", description: "Creates and funds a transaction in PSBT format", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "walletlock", description: "Removes the wallet encryption key from memory", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "walletpassphrase", description: "Unlocks the wallet for the specified time", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "walletpassphrasechange", description: "Changes the wallet passphrase", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
    VertcoinMethodIndex { name: "walletprocesspsbt", description: "Updates a PSBT with input information and signs inputs", kind: VertcoinMethodKind::WalletMethod, category: "wallet" },
];

/// Utility RPC methods
pub const VERTCOIN_UTIL_METHODS: &[VertcoinMethodIndex] = &[
    VertcoinMethodIndex { name: "createmultisig", description: "Creates a multi-signature address with n signatures of m keys required", kind: VertcoinMethodKind::RpcMethod, category: "util" },
    VertcoinMethodIndex { name: "deriveaddresses", description: "Derives one or more addresses from a descriptor", kind: VertcoinMethodKind::RpcMethod, category: "util" },
    VertcoinMethodIndex { name: "estimatesmartfee", description: "Estimates the fee per kilobyte for a transaction to be confirmed within a target number of blocks", kind: VertcoinMethodKind::RpcMethod, category: "util" },
    VertcoinMethodIndex { name: "getdescriptorinfo", description: "Analyses a descriptor", kind: VertcoinMethodKind::RpcMethod, category: "util" },
    VertcoinMethodIndex { name: "signmessagewithprivkey", description: "Signs a message with a private key", kind: VertcoinMethodKind::RpcMethod, category: "util" },
    VertcoinMethodIndex { name: "validateaddress", description: "Returns information about a given Vertcoin address", kind: VertcoinMethodKind::RpcMethod, category: "util" },
    VertcoinMethodIndex { name: "verifymessage", description: "Verifies a signed message", kind: VertcoinMethodKind::RpcMethod, category: "util" },
];

// ============================================================================
// VERTCOIN SPECIFICATIONS
// ============================================================================

/// Vertcoin network specifications and concepts
pub const VERTCOIN_SPECIFICATIONS: &[VertcoinMethodIndex] = &[
    VertcoinMethodIndex { name: "verthash", description: "Verthash is Vertcoin's GPU-optimized, ASIC-resistant mining algorithm. It requires a 1.2GB verthash.dat file and is memory-bound for fair GPU mining.", kind: VertcoinMethodKind::Specification, category: "specs" },
    VertcoinMethodIndex { name: "block-time", description: "Vertcoin has a 2.5 minute block time (same as Litecoin), allowing faster confirmations than Bitcoin's 10 minutes.", kind: VertcoinMethodKind::Specification, category: "specs" },
    VertcoinMethodIndex { name: "total-supply", description: "Maximum supply of 84,000,000 VTC (same as Litecoin), with a halving every 4 years.", kind: VertcoinMethodKind::Specification, category: "specs" },
    VertcoinMethodIndex { name: "difficulty-adjustment", description: "Difficulty adjusts every block using Kimoto Gravity Well (KGW) algorithm, allowing rapid response to hashrate changes.", kind: VertcoinMethodKind::Specification, category: "specs" },
    VertcoinMethodIndex { name: "segwit", description: "Vertcoin activated Segregated Witness (SegWit) on May 7, 2017, enabling faster transactions and Lightning Network compatibility.", kind: VertcoinMethodKind::Specification, category: "specs" },
    VertcoinMethodIndex { name: "no-premine", description: "Vertcoin had no premine, no ICO, and no airdrop - all coins are distributed through fair mining.", kind: VertcoinMethodKind::Specification, category: "specs" },
    VertcoinMethodIndex { name: "asic-resistance", description: "Vertcoin is committed to ASIC resistance to ensure mining remains accessible to individuals with consumer GPUs.", kind: VertcoinMethodKind::Specification, category: "specs" },
    VertcoinMethodIndex { name: "one-click-miner", description: "Vertcoin provides One Click Miner (OCM), a user-friendly application for easy GPU mining setup without command-line knowledge.", kind: VertcoinMethodKind::Specification, category: "specs" },
    VertcoinMethodIndex { name: "p2pool", description: "Vertcoin supports P2Pool decentralized mining pools, allowing miners to mine without trusting a central pool operator.", kind: VertcoinMethodKind::Specification, category: "specs" },
    VertcoinMethodIndex { name: "lightning-network", description: "With SegWit support, Vertcoin is compatible with the Lightning Network for instant, low-fee micropayments.", kind: VertcoinMethodKind::Specification, category: "specs" },
];
