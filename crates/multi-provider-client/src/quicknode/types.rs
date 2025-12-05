use serde::{Deserialize, Serialize};

/// QuickNode technology representation (Solana categories)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickNodeTechnology {
    pub identifier: String,
    pub title: String,
    pub description: String,
    pub url: String,
    pub item_count: usize,
}

/// Category of QuickNode methods (HTTP, WebSocket, Marketplace)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickNodeCategory {
    pub identifier: String,
    pub title: String,
    pub description: String,
    pub items: Vec<QuickNodeCategoryItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickNodeCategoryItem {
    pub name: String,
    pub description: String,
    pub kind: QuickNodeMethodKind,
    pub url: String,
}

/// Kind of QuickNode method
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuickNodeMethodKind {
    HttpMethod,
    WebSocketMethod,
    MarketplaceAddon,
}

impl std::fmt::Display for QuickNodeMethodKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HttpMethod => write!(f, "HTTP Method"),
            Self::WebSocketMethod => write!(f, "WebSocket Method"),
            Self::MarketplaceAddon => write!(f, "Marketplace Add-on"),
        }
    }
}

/// Detailed method documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickNodeMethod {
    pub name: String,
    pub description: String,
    pub kind: QuickNodeMethodKind,
    pub url: String,
    pub parameters: Vec<QuickNodeParameter>,
    pub returns: Option<QuickNodeReturnType>,
    pub examples: Vec<QuickNodeExample>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickNodeParameter {
    pub name: String,
    pub param_type: String,
    pub required: bool,
    pub description: String,
    pub default_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickNodeReturnType {
    pub type_name: String,
    pub description: String,
    pub fields: Vec<QuickNodeReturnField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickNodeReturnField {
    pub name: String,
    pub field_type: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickNodeExample {
    pub language: String,
    pub code: String,
    pub description: Option<String>,
}

/// Static method index entry (pre-defined for all Solana methods)
#[derive(Debug, Clone)]
pub struct SolanaMethodIndex {
    pub name: &'static str,
    pub description: &'static str,
    pub kind: QuickNodeMethodKind,
}

/// All known Solana HTTP RPC methods
pub const SOLANA_HTTP_METHODS: &[SolanaMethodIndex] = &[
    SolanaMethodIndex { name: "getAccountInfo", description: "Returns all information associated with the account of provided Pubkey", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getBalance", description: "Returns the balance of the account of provided Pubkey", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getBlock", description: "Returns identity and transaction information about a confirmed block in the ledger", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getBlockCommitment", description: "Returns commitment for particular block", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getBlockHeight", description: "Returns the current block height of the node", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getBlockProduction", description: "Returns recent block production information from the current or previous epoch", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getBlocks", description: "Returns a list of confirmed blocks between two slots", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getBlocksWithLimit", description: "Returns a list of confirmed blocks starting at the given slot", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getBlockTime", description: "Returns the estimated production time of a block", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getClusterNodes", description: "Returns information about all the nodes participating in the cluster", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getEpochInfo", description: "Returns information about the current epoch", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getEpochSchedule", description: "Returns epoch schedule information from this cluster's genesis config", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getFeeForMessage", description: "Returns the fee for a message", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getFirstAvailableBlock", description: "Returns the slot of the lowest confirmed block that has not been purged from the ledger", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getGenesisHash", description: "Returns the genesis hash", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getHealth", description: "Returns the current health of the node", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getHighestSnapshotSlot", description: "Returns the highest slot information that the node has snapshots for", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getIdentity", description: "Returns the identity pubkey for the current node", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getInflationGovernor", description: "Returns the current inflation governor", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getInflationRate", description: "Returns the specific inflation values for the current epoch", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getInflationReward", description: "Returns the inflation / staking reward for a list of addresses for an epoch", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getLargestAccounts", description: "Returns the 20 largest accounts, by lamport balance", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getLatestBlockhash", description: "Returns the latest blockhash", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getLeaderSchedule", description: "Returns the leader schedule for an epoch", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getMaxRetransmitSlot", description: "Get the max slot seen from retransmit stage", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getMaxShredInsertSlot", description: "Get the max slot seen from after shred insert", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getMinimumBalanceForRentExemption", description: "Returns minimum balance required to make account rent exempt", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getMultipleAccounts", description: "Returns the account information for a list of Pubkeys", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getProgramAccounts", description: "Returns all accounts owned by the provided program Pubkey", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getRecentPerformanceSamples", description: "Returns a list of recent performance samples", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getRecentPrioritizationFees", description: "Returns a list of prioritization fees from recent blocks", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getSignaturesForAddress", description: "Returns signatures for confirmed transactions that include the given address", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getSignatureStatuses", description: "Returns the statuses of a list of signatures", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getSlot", description: "Returns the current slot the node is processing", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getSlotLeader", description: "Returns the current slot leader", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getSlotLeaders", description: "Returns the slot leaders for a given slot range", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getStakeMinimumDelegation", description: "Returns the stake minimum delegation, in lamports", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getSupply", description: "Returns information about the current supply", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getTokenAccountBalance", description: "Returns the token balance of an SPL Token account", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getTokenAccountsByDelegate", description: "Returns all SPL Token accounts by approved Delegate", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getTokenAccountsByOwner", description: "Returns all SPL Token accounts by token owner", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getTokenLargestAccounts", description: "Returns the 20 largest accounts of a particular SPL Token type", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getTokenSupply", description: "Returns the total supply of an SPL Token type", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getTransaction", description: "Returns transaction details for a confirmed transaction", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getTransactionCount", description: "Returns the current transaction count from the ledger", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getVersion", description: "Returns the current solana version running on the node", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "getVoteAccounts", description: "Returns the account info and associated stake for all the voting accounts", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "isBlockhashValid", description: "Returns whether a blockhash is still valid or not", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "minimumLedgerSlot", description: "Returns the lowest slot that the node has information about in its ledger", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "requestAirdrop", description: "Requests an airdrop of lamports to a Pubkey", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "sendTransaction", description: "Submits a signed transaction to the cluster for processing", kind: QuickNodeMethodKind::HttpMethod },
    SolanaMethodIndex { name: "simulateTransaction", description: "Simulate sending a transaction", kind: QuickNodeMethodKind::HttpMethod },
];

/// All known Solana WebSocket methods
pub const SOLANA_WEBSOCKET_METHODS: &[SolanaMethodIndex] = &[
    SolanaMethodIndex { name: "accountSubscribe", description: "Subscribe to an account to receive notifications when the lamports or data changes", kind: QuickNodeMethodKind::WebSocketMethod },
    SolanaMethodIndex { name: "accountUnsubscribe", description: "Unsubscribe from account change notifications", kind: QuickNodeMethodKind::WebSocketMethod },
    SolanaMethodIndex { name: "blockSubscribe", description: "Subscribe to receive notification anytime a new block is confirmed or finalized", kind: QuickNodeMethodKind::WebSocketMethod },
    SolanaMethodIndex { name: "blockUnsubscribe", description: "Unsubscribe from block notifications", kind: QuickNodeMethodKind::WebSocketMethod },
    SolanaMethodIndex { name: "logsSubscribe", description: "Subscribe to transaction logging", kind: QuickNodeMethodKind::WebSocketMethod },
    SolanaMethodIndex { name: "logsUnsubscribe", description: "Unsubscribe from transaction logging", kind: QuickNodeMethodKind::WebSocketMethod },
    SolanaMethodIndex { name: "programSubscribe", description: "Subscribe to a program to receive notifications when the lamports or data changes", kind: QuickNodeMethodKind::WebSocketMethod },
    SolanaMethodIndex { name: "programUnsubscribe", description: "Unsubscribe from program-owned account change notifications", kind: QuickNodeMethodKind::WebSocketMethod },
    SolanaMethodIndex { name: "rootSubscribe", description: "Subscribe to receive notification anytime a new root is set by the validator", kind: QuickNodeMethodKind::WebSocketMethod },
    SolanaMethodIndex { name: "rootUnsubscribe", description: "Unsubscribe from root notifications", kind: QuickNodeMethodKind::WebSocketMethod },
    SolanaMethodIndex { name: "signatureSubscribe", description: "Subscribe to a transaction signature to receive notification when the transaction is confirmed", kind: QuickNodeMethodKind::WebSocketMethod },
    SolanaMethodIndex { name: "signatureUnsubscribe", description: "Unsubscribe from signature confirmation notification", kind: QuickNodeMethodKind::WebSocketMethod },
    SolanaMethodIndex { name: "slotSubscribe", description: "Subscribe to receive notification anytime a slot is processed by the validator", kind: QuickNodeMethodKind::WebSocketMethod },
    SolanaMethodIndex { name: "slotUnsubscribe", description: "Unsubscribe from slot notifications", kind: QuickNodeMethodKind::WebSocketMethod },
    SolanaMethodIndex { name: "slotsUpdatesSubscribe", description: "Subscribe to receive a notification from the validator on a variety of updates on every slot", kind: QuickNodeMethodKind::WebSocketMethod },
    SolanaMethodIndex { name: "slotsUpdatesUnsubscribe", description: "Unsubscribe from slot-update notifications", kind: QuickNodeMethodKind::WebSocketMethod },
    SolanaMethodIndex { name: "voteSubscribe", description: "Subscribe to receive notification anytime a new vote is observed in gossip", kind: QuickNodeMethodKind::WebSocketMethod },
    SolanaMethodIndex { name: "voteUnsubscribe", description: "Unsubscribe from vote notifications", kind: QuickNodeMethodKind::WebSocketMethod },
];

/// QuickNode Marketplace add-ons for Solana
pub const SOLANA_MARKETPLACE_ADDONS: &[SolanaMethodIndex] = &[
    SolanaMethodIndex { name: "jito-bundles", description: "JITO Bundles API for MEV protection and atomic transaction bundles", kind: QuickNodeMethodKind::MarketplaceAddon },
    SolanaMethodIndex { name: "metaplex-das-api", description: "Metaplex Digital Asset Standard API for NFT and compressed NFT data", kind: QuickNodeMethodKind::MarketplaceAddon },
    SolanaMethodIndex { name: "priority-fee-api", description: "Priority Fee API for optimal transaction fee estimation", kind: QuickNodeMethodKind::MarketplaceAddon },
    SolanaMethodIndex { name: "metis-trading-api", description: "Metis Jupiter V6 Swap API for DEX trading", kind: QuickNodeMethodKind::MarketplaceAddon },
    SolanaMethodIndex { name: "yellowstone-grpc", description: "Yellowstone Geyser gRPC for real-time blockchain data streaming", kind: QuickNodeMethodKind::MarketplaceAddon },
];
