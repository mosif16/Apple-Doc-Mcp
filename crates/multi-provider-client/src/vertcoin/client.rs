use std::path::PathBuf;
use std::time::Duration as StdDuration;

use anyhow::{Context, Result};
use directories::ProjectDirs;
use reqwest::Client;
use tokio::sync::Mutex;
use tracing::{debug, instrument, warn};

use super::types::{
    VertcoinCategory, VertcoinCategoryItem, VertcoinExample, VertcoinMethod,
    VertcoinMethodIndex, VertcoinMethodKind, VertcoinParameter, VertcoinReturnField,
    VertcoinReturnType, VertcoinTechnology,
    VERTCOIN_BLOCKCHAIN_METHODS, VERTCOIN_CONTROL_METHODS, VERTCOIN_MINING_METHODS,
    VERTCOIN_NETWORK_METHODS, VERTCOIN_RAWTRANSACTION_METHODS, VERTCOIN_SPECIFICATIONS,
    VERTCOIN_UTIL_METHODS, VERTCOIN_WALLET_METHODS,
};
use docs_mcp_client::cache::{DiskCache, MemoryCache};

const VERTCOIN_CORE_DOCS_URL: &str = "https://github.com/vertcoin-project/vertcoin-core/blob/master/doc";
const VERTCOIN_WIKI_URL: &str = "https://github.com/vertcoin-project/VertDocs";
const VERTCOIN_MAIN_URL: &str = "https://vertcoin.org";

#[derive(Debug)]
pub struct VertcoinClient {
    http: Client,
    disk_cache: DiskCache,
    memory_cache: MemoryCache<String>,
    fetch_lock: Mutex<()>,
    cache_dir: PathBuf,
}

impl Default for VertcoinClient {
    fn default() -> Self {
        Self::new()
    }
}

impl VertcoinClient {
    #[must_use]
    pub fn new() -> Self {
        let project_dirs = ProjectDirs::from("com", "RecordAndLearn", "multi-docs-mcp")
            .expect("unable to resolve project directories");

        let cache_dir = project_dirs.cache_dir().join("vertcoin");
        if let Err(e) = std::fs::create_dir_all(&cache_dir) {
            warn!(error = %e, "Failed to create Vertcoin cache directory");
        }

        let http = Client::builder()
            .user_agent("MultiDocsMCP/1.0")
            .timeout(StdDuration::from_secs(30))
            .gzip(true)
            .build()
            .expect("failed to build reqwest client");

        Self {
            http,
            disk_cache: DiskCache::new(&cache_dir),
            memory_cache: MemoryCache::new(time::Duration::hours(1)),
            fetch_lock: Mutex::new(()),
            cache_dir,
        }
    }

    /// Get available technologies (Vertcoin categories)
    #[instrument(name = "vertcoin_client.get_technologies", skip(self))]
    pub async fn get_technologies(&self) -> Result<Vec<VertcoinTechnology>> {
        let blockchain_tech = VertcoinTechnology {
            identifier: "vertcoin:blockchain".to_string(),
            title: "Blockchain RPC".to_string(),
            description: format!(
                "Vertcoin blockchain RPC methods - {} methods for querying blockchain state, blocks, and transactions",
                VERTCOIN_BLOCKCHAIN_METHODS.len()
            ),
            url: format!("{VERTCOIN_CORE_DOCS_URL}/JSON-RPC-interface.md"),
            item_count: VERTCOIN_BLOCKCHAIN_METHODS.len(),
        };

        let wallet_tech = VertcoinTechnology {
            identifier: "vertcoin:wallet".to_string(),
            title: "Wallet RPC".to_string(),
            description: format!(
                "Vertcoin wallet RPC methods - {} methods for managing wallets, addresses, and transactions",
                VERTCOIN_WALLET_METHODS.len()
            ),
            url: format!("{VERTCOIN_CORE_DOCS_URL}/JSON-RPC-interface.md"),
            item_count: VERTCOIN_WALLET_METHODS.len(),
        };

        let mining_tech = VertcoinTechnology {
            identifier: "vertcoin:mining".to_string(),
            title: "Mining (Verthash)".to_string(),
            description: format!(
                "Vertcoin Verthash mining - {} methods and specifications for GPU mining",
                VERTCOIN_MINING_METHODS.len()
            ),
            url: format!("{VERTCOIN_MAIN_URL}/specs-explained/"),
            item_count: VERTCOIN_MINING_METHODS.len(),
        };

        let network_tech = VertcoinTechnology {
            identifier: "vertcoin:network".to_string(),
            title: "Network RPC".to_string(),
            description: format!(
                "Vertcoin network RPC methods - {} methods for P2P networking and node management",
                VERTCOIN_NETWORK_METHODS.len()
            ),
            url: format!("{VERTCOIN_CORE_DOCS_URL}/JSON-RPC-interface.md"),
            item_count: VERTCOIN_NETWORK_METHODS.len(),
        };

        let specs_tech = VertcoinTechnology {
            identifier: "vertcoin:specs".to_string(),
            title: "Specifications".to_string(),
            description: format!(
                "Vertcoin specifications - {} core concepts including Verthash, block time, supply, and SegWit",
                VERTCOIN_SPECIFICATIONS.len()
            ),
            url: format!("{VERTCOIN_MAIN_URL}/specs-explained/"),
            item_count: VERTCOIN_SPECIFICATIONS.len(),
        };

        Ok(vec![blockchain_tech, wallet_tech, mining_tech, network_tech, specs_tech])
    }

    /// Get a category of methods
    #[instrument(name = "vertcoin_client.get_category", skip(self))]
    pub async fn get_category(&self, identifier: &str) -> Result<VertcoinCategory> {
        let (methods, title, description): (&[VertcoinMethodIndex], &str, &str) = match identifier {
            "vertcoin:blockchain" | "blockchain" => (
                VERTCOIN_BLOCKCHAIN_METHODS,
                "Blockchain RPC Methods",
                "JSON-RPC methods for querying Vertcoin blockchain state",
            ),
            "vertcoin:wallet" | "wallet" => (
                VERTCOIN_WALLET_METHODS,
                "Wallet RPC Methods",
                "JSON-RPC methods for Vertcoin wallet management",
            ),
            "vertcoin:mining" | "mining" | "verthash" => (
                VERTCOIN_MINING_METHODS,
                "Mining Methods (Verthash)",
                "Mining-related RPC methods for Verthash GPU mining",
            ),
            "vertcoin:network" | "network" => (
                VERTCOIN_NETWORK_METHODS,
                "Network RPC Methods",
                "P2P networking and node management methods",
            ),
            "vertcoin:rawtransactions" | "rawtransactions" | "raw" => (
                VERTCOIN_RAWTRANSACTION_METHODS,
                "Raw Transaction Methods",
                "Methods for creating and signing raw transactions and PSBTs",
            ),
            "vertcoin:control" | "control" => (
                VERTCOIN_CONTROL_METHODS,
                "Control Methods",
                "Node control and management methods",
            ),
            "vertcoin:util" | "util" => (
                VERTCOIN_UTIL_METHODS,
                "Utility Methods",
                "Utility methods for address validation, fee estimation, and signatures",
            ),
            "vertcoin:specs" | "specs" | "specifications" => (
                VERTCOIN_SPECIFICATIONS,
                "Vertcoin Specifications",
                "Core specifications and concepts for Vertcoin blockchain",
            ),
            _ => anyhow::bail!("Unknown Vertcoin category: {identifier}"),
        };

        let items = methods
            .iter()
            .map(|m| VertcoinCategoryItem {
                name: m.name.to_string(),
                description: m.description.to_string(),
                kind: m.kind,
                url: self.get_method_url(m),
            })
            .collect();

        Ok(VertcoinCategory {
            identifier: identifier.to_string(),
            title: title.to_string(),
            description: description.to_string(),
            items,
        })
    }

    /// Get URL for a method
    fn get_method_url(&self, method: &VertcoinMethodIndex) -> String {
        match method.kind {
            VertcoinMethodKind::Specification => {
                format!("{VERTCOIN_MAIN_URL}/specs-explained/")
            }
            VertcoinMethodKind::MiningMethod => {
                format!("{VERTCOIN_WIKI_URL}/blob/master/docs/Mining/")
            }
            _ => {
                format!("{VERTCOIN_CORE_DOCS_URL}/JSON-RPC-interface.md")
            }
        }
    }

    /// Get all methods as a flat list for searching
    fn all_methods() -> impl Iterator<Item = &'static VertcoinMethodIndex> {
        VERTCOIN_BLOCKCHAIN_METHODS
            .iter()
            .chain(VERTCOIN_WALLET_METHODS.iter())
            .chain(VERTCOIN_MINING_METHODS.iter())
            .chain(VERTCOIN_NETWORK_METHODS.iter())
            .chain(VERTCOIN_RAWTRANSACTION_METHODS.iter())
            .chain(VERTCOIN_CONTROL_METHODS.iter())
            .chain(VERTCOIN_UTIL_METHODS.iter())
            .chain(VERTCOIN_SPECIFICATIONS.iter())
    }

    /// Fetch additional documentation from GitHub (cached)
    async fn fetch_github_doc(&self, doc_path: &str) -> Result<String> {
        let cache_key = format!("github_{}.html", doc_path.replace('/', "_"));

        // Check memory cache first
        if let Some(html) = self.memory_cache.get(&cache_key) {
            debug!(path = doc_path, "Vertcoin doc served from memory cache");
            return Ok(html);
        }

        // Check disk cache
        if let Ok(Some(entry)) = self.disk_cache.load::<String>(&cache_key).await {
            debug!(path = doc_path, "Vertcoin doc served from disk cache");
            self.memory_cache.insert(cache_key.clone(), entry.value.clone());
            return Ok(entry.value);
        }

        // Lock to prevent concurrent fetches
        let _lock = self.fetch_lock.lock().await;

        // Double-check after acquiring lock
        if let Some(html) = self.memory_cache.get(&cache_key) {
            return Ok(html);
        }

        // Fetch from GitHub
        let url = format!("https://raw.githubusercontent.com/vertcoin-project/vertcoin-core/master/{doc_path}");
        debug!(url = %url, "Fetching Vertcoin documentation from GitHub");

        let response = self
            .http
            .get(&url)
            .send()
            .await
            .context("Failed to fetch Vertcoin documentation")?;

        if !response.status().is_success() {
            anyhow::bail!(
                "Vertcoin documentation fetch failed for {}: {}",
                doc_path,
                response.status()
            );
        }

        let content = response
            .text()
            .await
            .context("Failed to read Vertcoin response")?;

        // Store in caches
        self.memory_cache.insert(cache_key.clone(), content.clone());
        if let Err(e) = self.disk_cache.store(&cache_key, content.clone()).await {
            warn!(error = %e, "Failed to cache Vertcoin doc to disk");
        }

        Ok(content)
    }

    /// Build detailed method documentation
    fn build_method_doc(&self, index_entry: &VertcoinMethodIndex) -> VertcoinMethod {
        // Build examples based on method type
        let examples = self.generate_examples(index_entry);

        // Build parameters based on common patterns (Bitcoin RPC style)
        let parameters = self.infer_parameters(index_entry);

        VertcoinMethod {
            name: index_entry.name.to_string(),
            description: index_entry.description.to_string(),
            kind: index_entry.kind,
            url: self.get_method_url(index_entry),
            parameters,
            returns: self.infer_return_type(index_entry),
            examples,
        }
    }

    /// Generate example code for a method
    fn generate_examples(&self, method: &VertcoinMethodIndex) -> Vec<VertcoinExample> {
        let mut examples = Vec::new();

        // CLI example
        let cli_example = match method.name {
            "getblockchaininfo" => "vertcoin-cli getblockchaininfo",
            "getbalance" => "vertcoin-cli getbalance",
            "getnewaddress" => "vertcoin-cli getnewaddress \"\" \"bech32\"",
            "sendtoaddress" => "vertcoin-cli sendtoaddress \"VtcAddressHere\" 0.1",
            "getblock" => "vertcoin-cli getblock \"blockhash\" 2",
            "getblockcount" => "vertcoin-cli getblockcount",
            "getdifficulty" => "vertcoin-cli getdifficulty",
            "getmininginfo" => "vertcoin-cli getmininginfo",
            "getnetworkinfo" => "vertcoin-cli getnetworkinfo",
            "getpeerinfo" => "vertcoin-cli getpeerinfo",
            "getconnectioncount" => "vertcoin-cli getconnectioncount",
            "validateaddress" => "vertcoin-cli validateaddress \"VtcAddressHere\"",
            "estimatesmartfee" => "vertcoin-cli estimatesmartfee 6",
            "getblocktemplate" => "vertcoin-cli getblocktemplate '{\"rules\": [\"segwit\"]}'",
            "listunspent" => "vertcoin-cli listunspent 1 9999999",
            "listtransactions" => "vertcoin-cli listtransactions \"*\" 10",
            _ => {
                // Generic CLI example
                return vec![VertcoinExample {
                    language: "bash".to_string(),
                    code: format!("vertcoin-cli {}", method.name),
                    description: Some(format!("Call {} via vertcoin-cli", method.name)),
                }];
            }
        };

        examples.push(VertcoinExample {
            language: "bash".to_string(),
            code: cli_example.to_string(),
            description: Some(format!("Call {} via vertcoin-cli", method.name)),
        });

        // JSON-RPC example for RPC methods
        if matches!(method.kind, VertcoinMethodKind::RpcMethod | VertcoinMethodKind::WalletMethod) {
            let json_example = format!(
                r#"curl --user myusername --data-binary '{{"jsonrpc": "1.0", "id": "curltest", "method": "{}", "params": []}}' -H 'content-type: text/plain;' http://127.0.0.1:5888/"#,
                method.name
            );
            examples.push(VertcoinExample {
                language: "bash".to_string(),
                code: json_example,
                description: Some("JSON-RPC call via curl".to_string()),
            });
        }

        examples
    }

    /// Infer parameters for a method based on common patterns
    fn infer_parameters(&self, method: &VertcoinMethodIndex) -> Vec<VertcoinParameter> {
        // Common parameter patterns for Bitcoin-derived RPC
        match method.name {
            "getblock" => vec![
                VertcoinParameter {
                    name: "blockhash".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                    description: "The block hash (hex string)".to_string(),
                    default_value: None,
                },
                VertcoinParameter {
                    name: "verbosity".to_string(),
                    param_type: "number".to_string(),
                    required: false,
                    description: "0 for hex-encoded data, 1 for block data, 2 for block with tx data".to_string(),
                    default_value: Some("1".to_string()),
                },
            ],
            "getblockhash" => vec![
                VertcoinParameter {
                    name: "height".to_string(),
                    param_type: "number".to_string(),
                    required: true,
                    description: "The block height index".to_string(),
                    default_value: None,
                },
            ],
            "getblockheader" => vec![
                VertcoinParameter {
                    name: "blockhash".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                    description: "The block hash".to_string(),
                    default_value: None,
                },
                VertcoinParameter {
                    name: "verbose".to_string(),
                    param_type: "boolean".to_string(),
                    required: false,
                    description: "true for JSON object, false for hex string".to_string(),
                    default_value: Some("true".to_string()),
                },
            ],
            "sendtoaddress" => vec![
                VertcoinParameter {
                    name: "address".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                    description: "The Vertcoin address to send to (V... or vtc1...)".to_string(),
                    default_value: None,
                },
                VertcoinParameter {
                    name: "amount".to_string(),
                    param_type: "number".to_string(),
                    required: true,
                    description: "The amount in VTC to send (e.g., 0.1)".to_string(),
                    default_value: None,
                },
                VertcoinParameter {
                    name: "comment".to_string(),
                    param_type: "string".to_string(),
                    required: false,
                    description: "A comment stored in wallet for this transaction".to_string(),
                    default_value: None,
                },
                VertcoinParameter {
                    name: "comment_to".to_string(),
                    param_type: "string".to_string(),
                    required: false,
                    description: "A comment to store the recipient name".to_string(),
                    default_value: None,
                },
                VertcoinParameter {
                    name: "subtractfeefromamount".to_string(),
                    param_type: "boolean".to_string(),
                    required: false,
                    description: "Deduct fee from amount being sent".to_string(),
                    default_value: Some("false".to_string()),
                },
            ],
            "getnewaddress" => vec![
                VertcoinParameter {
                    name: "label".to_string(),
                    param_type: "string".to_string(),
                    required: false,
                    description: "The label for the address".to_string(),
                    default_value: Some("\"\"".to_string()),
                },
                VertcoinParameter {
                    name: "address_type".to_string(),
                    param_type: "string".to_string(),
                    required: false,
                    description: "Address type: legacy (V...), p2sh-segwit (3...), or bech32 (vtc1...)".to_string(),
                    default_value: Some("bech32".to_string()),
                },
            ],
            "estimatesmartfee" => vec![
                VertcoinParameter {
                    name: "conf_target".to_string(),
                    param_type: "number".to_string(),
                    required: true,
                    description: "Confirmation target in blocks (1-1008)".to_string(),
                    default_value: None,
                },
                VertcoinParameter {
                    name: "estimate_mode".to_string(),
                    param_type: "string".to_string(),
                    required: false,
                    description: "UNSET, ECONOMICAL, or CONSERVATIVE".to_string(),
                    default_value: Some("CONSERVATIVE".to_string()),
                },
            ],
            "validateaddress" => vec![
                VertcoinParameter {
                    name: "address".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                    description: "The Vertcoin address to validate (V..., 3..., or vtc1...)".to_string(),
                    default_value: None,
                },
            ],
            "getrawtransaction" => vec![
                VertcoinParameter {
                    name: "txid".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                    description: "The transaction ID (64-character hex string)".to_string(),
                    default_value: None,
                },
                VertcoinParameter {
                    name: "verbose".to_string(),
                    param_type: "boolean".to_string(),
                    required: false,
                    description: "If true, return JSON object; if false, return hex string".to_string(),
                    default_value: Some("false".to_string()),
                },
                VertcoinParameter {
                    name: "blockhash".to_string(),
                    param_type: "string".to_string(),
                    required: false,
                    description: "Block hash to look for transaction in".to_string(),
                    default_value: None,
                },
            ],
            "createwallet" => vec![
                VertcoinParameter {
                    name: "wallet_name".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                    description: "Name for the new wallet".to_string(),
                    default_value: None,
                },
                VertcoinParameter {
                    name: "disable_private_keys".to_string(),
                    param_type: "boolean".to_string(),
                    required: false,
                    description: "Disable private keys for watch-only wallet".to_string(),
                    default_value: Some("false".to_string()),
                },
                VertcoinParameter {
                    name: "blank".to_string(),
                    param_type: "boolean".to_string(),
                    required: false,
                    description: "Create blank wallet without HD seed".to_string(),
                    default_value: Some("false".to_string()),
                },
                VertcoinParameter {
                    name: "passphrase".to_string(),
                    param_type: "string".to_string(),
                    required: false,
                    description: "Encrypt wallet with this passphrase".to_string(),
                    default_value: None,
                },
            ],
            "sendmany" => vec![
                VertcoinParameter {
                    name: "dummy".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                    description: "Must be \"\" for backwards compatibility".to_string(),
                    default_value: Some("\"\"".to_string()),
                },
                VertcoinParameter {
                    name: "amounts".to_string(),
                    param_type: "object".to_string(),
                    required: true,
                    description: "JSON object with addresses as keys and amounts as values".to_string(),
                    default_value: None,
                },
                VertcoinParameter {
                    name: "minconf".to_string(),
                    param_type: "number".to_string(),
                    required: false,
                    description: "Minimum confirmations for inputs".to_string(),
                    default_value: Some("1".to_string()),
                },
                VertcoinParameter {
                    name: "comment".to_string(),
                    param_type: "string".to_string(),
                    required: false,
                    description: "A comment for the transaction".to_string(),
                    default_value: None,
                },
            ],
            "listunspent" => vec![
                VertcoinParameter {
                    name: "minconf".to_string(),
                    param_type: "number".to_string(),
                    required: false,
                    description: "Minimum confirmations to filter".to_string(),
                    default_value: Some("1".to_string()),
                },
                VertcoinParameter {
                    name: "maxconf".to_string(),
                    param_type: "number".to_string(),
                    required: false,
                    description: "Maximum confirmations to filter".to_string(),
                    default_value: Some("9999999".to_string()),
                },
                VertcoinParameter {
                    name: "addresses".to_string(),
                    param_type: "array".to_string(),
                    required: false,
                    description: "Filter to specific addresses".to_string(),
                    default_value: None,
                },
            ],
            "listtransactions" => vec![
                VertcoinParameter {
                    name: "label".to_string(),
                    param_type: "string".to_string(),
                    required: false,
                    description: "Filter by label (use \"*\" for all)".to_string(),
                    default_value: Some("\"*\"".to_string()),
                },
                VertcoinParameter {
                    name: "count".to_string(),
                    param_type: "number".to_string(),
                    required: false,
                    description: "Number of transactions to return".to_string(),
                    default_value: Some("10".to_string()),
                },
                VertcoinParameter {
                    name: "skip".to_string(),
                    param_type: "number".to_string(),
                    required: false,
                    description: "Number of transactions to skip".to_string(),
                    default_value: Some("0".to_string()),
                },
                VertcoinParameter {
                    name: "include_watchonly".to_string(),
                    param_type: "boolean".to_string(),
                    required: false,
                    description: "Include watch-only addresses".to_string(),
                    default_value: Some("true".to_string()),
                },
            ],
            "getblocktemplate" => vec![
                VertcoinParameter {
                    name: "template_request".to_string(),
                    param_type: "object".to_string(),
                    required: false,
                    description: "JSON object with \"rules\": [\"segwit\"] for SegWit support".to_string(),
                    default_value: Some("{\"rules\": [\"segwit\"]}".to_string()),
                },
            ],
            "addnode" => vec![
                VertcoinParameter {
                    name: "node".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                    description: "Node address (IP:port or DNS)".to_string(),
                    default_value: None,
                },
                VertcoinParameter {
                    name: "command".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                    description: "add, remove, or onetry".to_string(),
                    default_value: None,
                },
            ],
            "setban" => vec![
                VertcoinParameter {
                    name: "subnet".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                    description: "IP/Subnet with optional netmask".to_string(),
                    default_value: None,
                },
                VertcoinParameter {
                    name: "command".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                    description: "add or remove".to_string(),
                    default_value: None,
                },
                VertcoinParameter {
                    name: "bantime".to_string(),
                    param_type: "number".to_string(),
                    required: false,
                    description: "Ban time in seconds (0 = use default)".to_string(),
                    default_value: Some("0".to_string()),
                },
            ],
            "importprivkey" => vec![
                VertcoinParameter {
                    name: "privkey".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                    description: "The private key (WIF format)".to_string(),
                    default_value: None,
                },
                VertcoinParameter {
                    name: "label".to_string(),
                    param_type: "string".to_string(),
                    required: false,
                    description: "Label for the address".to_string(),
                    default_value: Some("\"\"".to_string()),
                },
                VertcoinParameter {
                    name: "rescan".to_string(),
                    param_type: "boolean".to_string(),
                    required: false,
                    description: "Rescan blockchain for transactions".to_string(),
                    default_value: Some("true".to_string()),
                },
            ],
            "encryptwallet" => vec![
                VertcoinParameter {
                    name: "passphrase".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                    description: "Passphrase to encrypt wallet with".to_string(),
                    default_value: None,
                },
            ],
            "walletpassphrase" => vec![
                VertcoinParameter {
                    name: "passphrase".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                    description: "Wallet passphrase".to_string(),
                    default_value: None,
                },
                VertcoinParameter {
                    name: "timeout".to_string(),
                    param_type: "number".to_string(),
                    required: true,
                    description: "Seconds to keep wallet unlocked".to_string(),
                    default_value: None,
                },
            ],
            "signmessage" => vec![
                VertcoinParameter {
                    name: "address".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                    description: "Vertcoin address whose key to use".to_string(),
                    default_value: None,
                },
                VertcoinParameter {
                    name: "message".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                    description: "Message to sign".to_string(),
                    default_value: None,
                },
            ],
            "verifymessage" => vec![
                VertcoinParameter {
                    name: "address".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                    description: "Vertcoin address that signed the message".to_string(),
                    default_value: None,
                },
                VertcoinParameter {
                    name: "signature".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                    description: "Base64-encoded signature".to_string(),
                    default_value: None,
                },
                VertcoinParameter {
                    name: "message".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                    description: "The original message".to_string(),
                    default_value: None,
                },
            ],
            "getnetworkhashps" => vec![
                VertcoinParameter {
                    name: "nblocks".to_string(),
                    param_type: "number".to_string(),
                    required: false,
                    description: "Blocks to average over (-1 for since last difficulty change)".to_string(),
                    default_value: Some("120".to_string()),
                },
                VertcoinParameter {
                    name: "height".to_string(),
                    param_type: "number".to_string(),
                    required: false,
                    description: "Block height to estimate at (-1 for current)".to_string(),
                    default_value: Some("-1".to_string()),
                },
            ],
            _ => Vec::new(),
        }
    }

    /// Infer return type for a method
    fn infer_return_type(&self, method: &VertcoinMethodIndex) -> Option<VertcoinReturnType> {
        match method.name {
            "getblockchaininfo" => Some(VertcoinReturnType {
                type_name: "object".to_string(),
                description: "Blockchain state information".to_string(),
                fields: vec![
                    VertcoinReturnField { name: "chain".to_string(), field_type: "string".to_string(), description: "Current network name (main, test, regtest)".to_string() },
                    VertcoinReturnField { name: "blocks".to_string(), field_type: "number".to_string(), description: "Number of blocks processed".to_string() },
                    VertcoinReturnField { name: "headers".to_string(), field_type: "number".to_string(), description: "Number of headers validated".to_string() },
                    VertcoinReturnField { name: "bestblockhash".to_string(), field_type: "string".to_string(), description: "Hash of the best block".to_string() },
                    VertcoinReturnField { name: "difficulty".to_string(), field_type: "number".to_string(), description: "Current Verthash mining difficulty".to_string() },
                    VertcoinReturnField { name: "verificationprogress".to_string(), field_type: "number".to_string(), description: "Estimate of verification progress (0-1)".to_string() },
                    VertcoinReturnField { name: "pruned".to_string(), field_type: "boolean".to_string(), description: "Whether blockchain is pruned".to_string() },
                    VertcoinReturnField { name: "size_on_disk".to_string(), field_type: "number".to_string(), description: "Blockchain size in bytes".to_string() },
                ],
            }),
            "getbalance" => Some(VertcoinReturnType {
                type_name: "number".to_string(),
                description: "The total available balance in VTC".to_string(),
                fields: vec![],
            }),
            "getbalances" => Some(VertcoinReturnType {
                type_name: "object".to_string(),
                description: "All wallet balances".to_string(),
                fields: vec![
                    VertcoinReturnField { name: "mine.trusted".to_string(), field_type: "number".to_string(), description: "Trusted balance".to_string() },
                    VertcoinReturnField { name: "mine.untrusted_pending".to_string(), field_type: "number".to_string(), description: "Untrusted pending balance".to_string() },
                    VertcoinReturnField { name: "mine.immature".to_string(), field_type: "number".to_string(), description: "Immature coinbase balance".to_string() },
                ],
            }),
            "getblockcount" => Some(VertcoinReturnType {
                type_name: "number".to_string(),
                description: "The current block count".to_string(),
                fields: vec![],
            }),
            "getdifficulty" => Some(VertcoinReturnType {
                type_name: "number".to_string(),
                description: "The current Verthash mining difficulty".to_string(),
                fields: vec![],
            }),
            "getconnectioncount" => Some(VertcoinReturnType {
                type_name: "number".to_string(),
                description: "The number of connections to other nodes".to_string(),
                fields: vec![],
            }),
            "getnewaddress" => Some(VertcoinReturnType {
                type_name: "string".to_string(),
                description: "A new Vertcoin address (V... for legacy, vtc1... for bech32)".to_string(),
                fields: vec![],
            }),
            "getmininginfo" => Some(VertcoinReturnType {
                type_name: "object".to_string(),
                description: "Mining-related information".to_string(),
                fields: vec![
                    VertcoinReturnField { name: "blocks".to_string(), field_type: "number".to_string(), description: "Current block height".to_string() },
                    VertcoinReturnField { name: "difficulty".to_string(), field_type: "number".to_string(), description: "Current Verthash difficulty".to_string() },
                    VertcoinReturnField { name: "networkhashps".to_string(), field_type: "number".to_string(), description: "Estimated network hash rate".to_string() },
                    VertcoinReturnField { name: "pooledtx".to_string(), field_type: "number".to_string(), description: "Size of mempool".to_string() },
                    VertcoinReturnField { name: "chain".to_string(), field_type: "string".to_string(), description: "Network name".to_string() },
                ],
            }),
            "getnetworkinfo" => Some(VertcoinReturnType {
                type_name: "object".to_string(),
                description: "P2P networking state".to_string(),
                fields: vec![
                    VertcoinReturnField { name: "version".to_string(), field_type: "number".to_string(), description: "Server version".to_string() },
                    VertcoinReturnField { name: "subversion".to_string(), field_type: "string".to_string(), description: "Server subversion string".to_string() },
                    VertcoinReturnField { name: "protocolversion".to_string(), field_type: "number".to_string(), description: "Protocol version".to_string() },
                    VertcoinReturnField { name: "connections".to_string(), field_type: "number".to_string(), description: "Number of connections".to_string() },
                    VertcoinReturnField { name: "connections_in".to_string(), field_type: "number".to_string(), description: "Inbound connections".to_string() },
                    VertcoinReturnField { name: "connections_out".to_string(), field_type: "number".to_string(), description: "Outbound connections".to_string() },
                    VertcoinReturnField { name: "networkactive".to_string(), field_type: "boolean".to_string(), description: "Network is active".to_string() },
                ],
            }),
            "getwalletinfo" => Some(VertcoinReturnType {
                type_name: "object".to_string(),
                description: "Wallet state information".to_string(),
                fields: vec![
                    VertcoinReturnField { name: "walletname".to_string(), field_type: "string".to_string(), description: "Wallet name".to_string() },
                    VertcoinReturnField { name: "walletversion".to_string(), field_type: "number".to_string(), description: "Wallet version".to_string() },
                    VertcoinReturnField { name: "balance".to_string(), field_type: "number".to_string(), description: "Confirmed balance".to_string() },
                    VertcoinReturnField { name: "unconfirmed_balance".to_string(), field_type: "number".to_string(), description: "Unconfirmed balance".to_string() },
                    VertcoinReturnField { name: "txcount".to_string(), field_type: "number".to_string(), description: "Number of transactions".to_string() },
                    VertcoinReturnField { name: "keypoolsize".to_string(), field_type: "number".to_string(), description: "Keypool size".to_string() },
                    VertcoinReturnField { name: "unlocked_until".to_string(), field_type: "number".to_string(), description: "Unlock expiration timestamp".to_string() },
                ],
            }),
            "getpeerinfo" => Some(VertcoinReturnType {
                type_name: "array".to_string(),
                description: "List of connected peer information".to_string(),
                fields: vec![
                    VertcoinReturnField { name: "[].id".to_string(), field_type: "number".to_string(), description: "Peer index".to_string() },
                    VertcoinReturnField { name: "[].addr".to_string(), field_type: "string".to_string(), description: "IP:port".to_string() },
                    VertcoinReturnField { name: "[].subver".to_string(), field_type: "string".to_string(), description: "User agent".to_string() },
                    VertcoinReturnField { name: "[].version".to_string(), field_type: "number".to_string(), description: "Protocol version".to_string() },
                    VertcoinReturnField { name: "[].synced_blocks".to_string(), field_type: "number".to_string(), description: "Last synced block".to_string() },
                ],
            }),
            "getmempoolinfo" => Some(VertcoinReturnType {
                type_name: "object".to_string(),
                description: "Mempool state".to_string(),
                fields: vec![
                    VertcoinReturnField { name: "loaded".to_string(), field_type: "boolean".to_string(), description: "Mempool is loaded".to_string() },
                    VertcoinReturnField { name: "size".to_string(), field_type: "number".to_string(), description: "Number of transactions".to_string() },
                    VertcoinReturnField { name: "bytes".to_string(), field_type: "number".to_string(), description: "Total size in bytes".to_string() },
                    VertcoinReturnField { name: "usage".to_string(), field_type: "number".to_string(), description: "Memory usage".to_string() },
                    VertcoinReturnField { name: "mempoolminfee".to_string(), field_type: "number".to_string(), description: "Minimum fee rate".to_string() },
                ],
            }),
            "validateaddress" => Some(VertcoinReturnType {
                type_name: "object".to_string(),
                description: "Address validation result".to_string(),
                fields: vec![
                    VertcoinReturnField { name: "isvalid".to_string(), field_type: "boolean".to_string(), description: "Address is valid".to_string() },
                    VertcoinReturnField { name: "address".to_string(), field_type: "string".to_string(), description: "The address".to_string() },
                    VertcoinReturnField { name: "scriptPubKey".to_string(), field_type: "string".to_string(), description: "Script public key".to_string() },
                    VertcoinReturnField { name: "isscript".to_string(), field_type: "boolean".to_string(), description: "Is P2SH".to_string() },
                    VertcoinReturnField { name: "iswitness".to_string(), field_type: "boolean".to_string(), description: "Is SegWit".to_string() },
                ],
            }),
            "estimatesmartfee" => Some(VertcoinReturnType {
                type_name: "object".to_string(),
                description: "Fee estimation result".to_string(),
                fields: vec![
                    VertcoinReturnField { name: "feerate".to_string(), field_type: "number".to_string(), description: "Fee rate in VTC/kB".to_string() },
                    VertcoinReturnField { name: "blocks".to_string(), field_type: "number".to_string(), description: "Blocks for estimate".to_string() },
                ],
            }),
            "listunspent" => Some(VertcoinReturnType {
                type_name: "array".to_string(),
                description: "List of unspent transaction outputs".to_string(),
                fields: vec![
                    VertcoinReturnField { name: "[].txid".to_string(), field_type: "string".to_string(), description: "Transaction ID".to_string() },
                    VertcoinReturnField { name: "[].vout".to_string(), field_type: "number".to_string(), description: "Output index".to_string() },
                    VertcoinReturnField { name: "[].address".to_string(), field_type: "string".to_string(), description: "Vertcoin address".to_string() },
                    VertcoinReturnField { name: "[].amount".to_string(), field_type: "number".to_string(), description: "Amount in VTC".to_string() },
                    VertcoinReturnField { name: "[].confirmations".to_string(), field_type: "number".to_string(), description: "Number of confirmations".to_string() },
                    VertcoinReturnField { name: "[].spendable".to_string(), field_type: "boolean".to_string(), description: "Is spendable".to_string() },
                ],
            }),
            "sendtoaddress" | "sendmany" => Some(VertcoinReturnType {
                type_name: "string".to_string(),
                description: "Transaction ID (txid) of the sent transaction".to_string(),
                fields: vec![],
            }),
            "signmessage" => Some(VertcoinReturnType {
                type_name: "string".to_string(),
                description: "Base64-encoded signature".to_string(),
                fields: vec![],
            }),
            "verifymessage" => Some(VertcoinReturnType {
                type_name: "boolean".to_string(),
                description: "True if signature is valid".to_string(),
                fields: vec![],
            }),
            "getbestblockhash" | "getblockhash" => Some(VertcoinReturnType {
                type_name: "string".to_string(),
                description: "Block hash (64-character hex string)".to_string(),
                fields: vec![],
            }),
            "getnetworkhashps" => Some(VertcoinReturnType {
                type_name: "number".to_string(),
                description: "Estimated network hash rate in hashes per second".to_string(),
                fields: vec![],
            }),
            "uptime" => Some(VertcoinReturnType {
                type_name: "number".to_string(),
                description: "Server uptime in seconds".to_string(),
                fields: vec![],
            }),
            _ => None,
        }
    }

    /// Get a specific method by name
    #[instrument(name = "vertcoin_client.get_method", skip(self))]
    pub async fn get_method(&self, name: &str) -> Result<VertcoinMethod> {
        // Find method in index
        let index_entry = Self::all_methods()
            .find(|m| m.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| anyhow::anyhow!("Vertcoin method not found: {name}"))?;

        Ok(self.build_method_doc(index_entry))
    }

    /// Search for methods matching a query
    #[instrument(name = "vertcoin_client.search", skip(self))]
    pub async fn search(&self, query: &str) -> Result<Vec<VertcoinMethod>> {
        let query_lower = query.to_lowercase();

        // Split query into keywords
        let keywords: Vec<&str> = query_lower
            .split(|c: char| c.is_whitespace() || c == '-' || c == '_')
            .filter(|s| !s.is_empty() && s.len() > 1)
            .collect();

        let mut scored_results: Vec<(i32, &VertcoinMethodIndex)> = Vec::new();

        // Search all methods
        for method in Self::all_methods() {
            let name_lower = method.name.to_lowercase();
            let desc_lower = method.description.to_lowercase();
            let category_lower = method.category.to_lowercase();

            let mut score = 0i32;

            for keyword in &keywords {
                // Exact name match
                if name_lower == *keyword {
                    score += 50;
                }
                // Name contains keyword
                else if name_lower.contains(keyword) {
                    score += 20;
                }
                // Category match
                if category_lower.contains(keyword) {
                    score += 10;
                }
                // Description contains keyword
                if desc_lower.contains(keyword) {
                    score += 5;
                }
            }

            // Boost for Vertcoin-specific terms
            if query_lower.contains("verthash") &&
               (method.kind == VertcoinMethodKind::MiningMethod ||
                method.name.contains("mining") ||
                method.description.to_lowercase().contains("verthash")) {
                score += 15;
            }

            if score > 0 {
                scored_results.push((score, method));
            }
        }

        // Sort by score (highest first)
        scored_results.sort_by(|a, b| b.0.cmp(&a.0));

        // Convert to VertcoinMethod
        let results: Vec<VertcoinMethod> = scored_results
            .into_iter()
            .take(20)
            .map(|(_, m)| self.build_method_doc(m))
            .collect();

        Ok(results)
    }

    pub fn cache_dir(&self) -> &PathBuf {
        &self.cache_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let _client = VertcoinClient::new();
    }

    #[test]
    fn test_all_methods_count() {
        let count = VertcoinClient::all_methods().count();
        assert!(count > 50, "Expected at least 50 methods, got {}", count);
    }
}
