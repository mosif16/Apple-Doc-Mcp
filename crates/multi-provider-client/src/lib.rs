pub mod claude_agent_sdk;
pub mod cocoon;
pub mod cuda;
pub mod huggingface;
pub mod mdn;
pub mod mlx;
pub mod quicknode;
pub mod rust;
pub mod telegram;
pub mod ton;
pub mod types;
pub mod vertcoin;
pub mod web_frameworks;

use std::collections::HashMap;

use anyhow::Result;
use docs_mcp_client::AppleDocsClient;

use claude_agent_sdk::ClaudeAgentSdkClient;
use cocoon::CocoonClient;
use cuda::CudaClient;
use huggingface::HuggingFaceClient;
use mdn::MdnClient;
use mlx::MlxClient;
use quicknode::QuickNodeClient;
use rust::RustClient;
use telegram::TelegramClient;
use ton::TonClient;
use types::{ProviderType, UnifiedFrameworkData, UnifiedSymbolData, UnifiedTechnology};
use vertcoin::VertcoinClient;
use web_frameworks::WebFrameworksClient;

/// All provider clients for simultaneous access
#[derive(Debug)]
pub struct ProviderClients {
    pub apple: AppleDocsClient,
    pub telegram: TelegramClient,
    pub ton: TonClient,
    pub cocoon: CocoonClient,
    pub rust: RustClient,
    pub mdn: MdnClient,
    pub web_frameworks: WebFrameworksClient,
    pub mlx: MlxClient,
    pub huggingface: HuggingFaceClient,
    pub quicknode: QuickNodeClient,
    pub claude_agent_sdk: ClaudeAgentSdkClient,
    pub vertcoin: VertcoinClient,
    pub cuda: CudaClient,
}

impl Default for ProviderClients {
    fn default() -> Self {
        Self::new()
    }
}

impl ProviderClients {
    #[must_use]
    pub fn new() -> Self {
        Self {
            apple: AppleDocsClient::new(),
            telegram: TelegramClient::new(),
            ton: TonClient::new(),
            cocoon: CocoonClient::new(),
            rust: RustClient::new(),
            mdn: MdnClient::new(),
            web_frameworks: WebFrameworksClient::new(),
            mlx: MlxClient::new(),
            huggingface: HuggingFaceClient::new(),
            quicknode: QuickNodeClient::new(),
            claude_agent_sdk: ClaudeAgentSdkClient::new(),
            vertcoin: VertcoinClient::new(),
            cuda: CudaClient::new(),
        }
    }

    /// Get technologies from all providers
    pub async fn get_all_technologies(
        &self,
    ) -> Result<HashMap<ProviderType, Vec<UnifiedTechnology>>> {
        let (apple, telegram, ton, cocoon, rust, mdn, webfw, mlx, hf, qn, agent_sdk, vtc, cuda) = tokio::join!(
            self.apple.get_technologies(),
            self.telegram.get_technologies(),
            self.ton.get_technologies(),
            self.cocoon.get_technologies(),
            self.rust.get_technologies(),
            self.mdn.get_technologies(),
            self.web_frameworks.get_technologies(),
            self.mlx.get_technologies(),
            self.huggingface.get_technologies(),
            self.quicknode.get_technologies(),
            self.claude_agent_sdk.get_technologies(),
            self.vertcoin.get_technologies(),
            self.cuda.get_technologies()
        );

        let mut result = HashMap::new();

        if let Ok(techs) = apple {
            result.insert(
                ProviderType::Apple,
                techs
                    .into_values()
                    .map(UnifiedTechnology::from_apple)
                    .collect(),
            );
        }

        if let Ok(techs) = telegram {
            result.insert(
                ProviderType::Telegram,
                techs.into_iter().map(UnifiedTechnology::from_telegram).collect(),
            );
        }

        if let Ok(techs) = ton {
            result.insert(
                ProviderType::TON,
                techs.into_iter().map(UnifiedTechnology::from_ton).collect(),
            );
        }

        if let Ok(techs) = cocoon {
            result.insert(
                ProviderType::Cocoon,
                techs.into_iter().map(UnifiedTechnology::from_cocoon).collect(),
            );
        }

        if let Ok(techs) = rust {
            result.insert(
                ProviderType::Rust,
                techs.into_iter().map(UnifiedTechnology::from_rust).collect(),
            );
        }

        if let Ok(techs) = mdn {
            result.insert(
                ProviderType::Mdn,
                techs.into_iter().map(UnifiedTechnology::from_mdn).collect(),
            );
        }

        if let Ok(techs) = webfw {
            result.insert(
                ProviderType::WebFrameworks,
                techs.into_iter().map(UnifiedTechnology::from_web_framework).collect(),
            );
        }

        if let Ok(techs) = mlx {
            result.insert(
                ProviderType::Mlx,
                techs.into_iter().map(UnifiedTechnology::from_mlx).collect(),
            );
        }

        if let Ok(techs) = hf {
            result.insert(
                ProviderType::HuggingFace,
                techs.into_iter().map(UnifiedTechnology::from_huggingface).collect(),
            );
        }

        if let Ok(techs) = qn {
            result.insert(
                ProviderType::QuickNode,
                techs.into_iter().map(UnifiedTechnology::from_quicknode).collect(),
            );
        }

        if let Ok(techs) = agent_sdk {
            result.insert(
                ProviderType::ClaudeAgentSdk,
                techs
                    .into_iter()
                    .map(UnifiedTechnology::from_claude_agent_sdk)
                    .collect(),
            );
        }

        if let Ok(techs) = vtc {
            result.insert(
                ProviderType::Vertcoin,
                techs
                    .into_iter()
                    .map(UnifiedTechnology::from_vertcoin)
                    .collect(),
            );
        }

        if let Ok(techs) = cuda {
            result.insert(
                ProviderType::Cuda,
                techs
                    .into_iter()
                    .map(UnifiedTechnology::from_cuda)
                    .collect(),
            );
        }

        Ok(result)
    }

    /// Get technologies for a specific provider
    pub async fn get_technologies_for(
        &self,
        provider: ProviderType,
    ) -> Result<Vec<UnifiedTechnology>> {
        match provider {
            ProviderType::Apple => {
                let techs = self.apple.get_technologies().await?;
                Ok(techs
                    .into_values()
                    .map(UnifiedTechnology::from_apple)
                    .collect())
            }
            ProviderType::Telegram => {
                let techs = self.telegram.get_technologies().await?;
                Ok(techs.into_iter().map(UnifiedTechnology::from_telegram).collect())
            }
            ProviderType::TON => {
                let techs = self.ton.get_technologies().await?;
                Ok(techs.into_iter().map(UnifiedTechnology::from_ton).collect())
            }
            ProviderType::Cocoon => {
                let techs = self.cocoon.get_technologies().await?;
                Ok(techs.into_iter().map(UnifiedTechnology::from_cocoon).collect())
            }
            ProviderType::Rust => {
                let techs = self.rust.get_technologies().await?;
                Ok(techs.into_iter().map(UnifiedTechnology::from_rust).collect())
            }
            ProviderType::Mdn => {
                let techs = self.mdn.get_technologies().await?;
                Ok(techs.into_iter().map(UnifiedTechnology::from_mdn).collect())
            }
            ProviderType::WebFrameworks => {
                let techs = self.web_frameworks.get_technologies().await?;
                Ok(techs.into_iter().map(UnifiedTechnology::from_web_framework).collect())
            }
            ProviderType::Mlx => {
                let techs = self.mlx.get_technologies().await?;
                Ok(techs.into_iter().map(UnifiedTechnology::from_mlx).collect())
            }
            ProviderType::HuggingFace => {
                let techs = self.huggingface.get_technologies().await?;
                Ok(techs.into_iter().map(UnifiedTechnology::from_huggingface).collect())
            }
            ProviderType::QuickNode => {
                let techs = self.quicknode.get_technologies().await?;
                Ok(techs.into_iter().map(UnifiedTechnology::from_quicknode).collect())
            }
            ProviderType::ClaudeAgentSdk => {
                let techs = self.claude_agent_sdk.get_technologies().await?;
                Ok(techs
                    .into_iter()
                    .map(UnifiedTechnology::from_claude_agent_sdk)
                    .collect())
            }
            ProviderType::Vertcoin => {
                let techs = self.vertcoin.get_technologies().await?;
                Ok(techs
                    .into_iter()
                    .map(UnifiedTechnology::from_vertcoin)
                    .collect())
            }
            ProviderType::Cuda => {
                let techs = self.cuda.get_technologies().await?;
                Ok(techs
                    .into_iter()
                    .map(UnifiedTechnology::from_cuda)
                    .collect())
            }
        }
    }

    /// Get framework data for a specific provider and identifier
    pub async fn get_framework(
        &self,
        provider: ProviderType,
        identifier: &str,
    ) -> Result<UnifiedFrameworkData> {
        match provider {
            ProviderType::Apple => {
                let data = self.apple.get_framework(identifier).await?;
                Ok(UnifiedFrameworkData::from_apple(data))
            }
            ProviderType::Telegram => {
                let data = self.telegram.get_category(identifier).await?;
                Ok(UnifiedFrameworkData::from_telegram(data))
            }
            ProviderType::TON => {
                let data = self.ton.get_category(identifier).await?;
                Ok(UnifiedFrameworkData::from_ton(data))
            }
            ProviderType::Cocoon => {
                let data = self.cocoon.get_section(identifier).await?;
                Ok(UnifiedFrameworkData::from_cocoon(data))
            }
            ProviderType::Rust => {
                let data = self.rust.get_category(identifier).await?;
                Ok(UnifiedFrameworkData::from_rust(data))
            }
            ProviderType::Mdn | ProviderType::WebFrameworks => {
                // MDN and WebFrameworks don't have a framework/category structure
                // like other providers - they work directly with articles
                anyhow::bail!(
                    "Provider {} does not support framework/category browsing. Use get_symbol for article access.",
                    provider.name()
                )
            }
            ProviderType::Mlx => {
                let data = self.mlx.get_category(identifier).await?;
                Ok(UnifiedFrameworkData::from_mlx(data))
            }
            ProviderType::HuggingFace => {
                let data = self.huggingface.get_category(identifier).await?;
                Ok(UnifiedFrameworkData::from_huggingface(data))
            }
            ProviderType::QuickNode => {
                let data = self.quicknode.get_category(identifier).await?;
                Ok(UnifiedFrameworkData::from_quicknode(data))
            }
            ProviderType::ClaudeAgentSdk => {
                let data = self.claude_agent_sdk.get_category(identifier).await?;
                Ok(UnifiedFrameworkData::from_claude_agent_sdk(data))
            }
            ProviderType::Vertcoin => {
                let data = self.vertcoin.get_category(identifier).await?;
                Ok(UnifiedFrameworkData::from_vertcoin(data))
            }
            ProviderType::Cuda => {
                let data = self.cuda.get_category(identifier).await?;
                Ok(UnifiedFrameworkData::from_cuda(data))
            }
        }
    }

    /// Get symbol/item data for a specific provider and path
    pub async fn get_symbol(
        &self,
        provider: ProviderType,
        path: &str,
    ) -> Result<UnifiedSymbolData> {
        match provider {
            ProviderType::Apple => {
                let data = self.apple.get_symbol(path).await?;
                Ok(UnifiedSymbolData::from_apple(data))
            }
            ProviderType::Telegram => {
                let data = self.telegram.get_item(path).await?;
                Ok(UnifiedSymbolData::from_telegram(data))
            }
            ProviderType::TON => {
                let data = self.ton.get_endpoint(path).await?;
                Ok(UnifiedSymbolData::from_ton(data))
            }
            ProviderType::Cocoon => {
                let data = self.cocoon.get_document(path).await?;
                Ok(UnifiedSymbolData::from_cocoon(data))
            }
            ProviderType::Rust => {
                let data = self.rust.get_item(path).await?;
                Ok(UnifiedSymbolData::from_rust(data))
            }
            ProviderType::Mdn => {
                let data = self.mdn.get_article(path).await?;
                Ok(UnifiedSymbolData::from_mdn(data))
            }
            ProviderType::WebFrameworks => {
                // Parse the path to determine framework (e.g., "react/reference/useState")
                let parts: Vec<&str> = path.splitn(2, '/').collect();
                let framework = web_frameworks::types::WebFramework::from_str_opt(parts[0])
                    .unwrap_or(web_frameworks::types::WebFramework::React);
                let slug = parts.get(1).unwrap_or(&path);
                let data = self.web_frameworks.get_article(framework, slug).await?;
                Ok(UnifiedSymbolData::from_web_framework(data))
            }
            ProviderType::Mlx => {
                // Parse the path to determine language (e.g., "swift/MLXArray" or "python/mlx.core.array")
                let parts: Vec<&str> = path.splitn(2, '/').collect();
                let language = if parts[0].to_lowercase().contains("python") {
                    mlx::types::MlxLanguage::Python
                } else {
                    mlx::types::MlxLanguage::Swift
                };
                let slug = parts.get(1).unwrap_or(&path);
                let data = self.mlx.get_article(slug, language).await?;
                Ok(UnifiedSymbolData::from_mlx(data))
            }
            ProviderType::HuggingFace => {
                // Parse the path to determine technology (e.g., "transformers/AutoModel" or "swift-transformers/Hub")
                let parts: Vec<&str> = path.splitn(2, '/').collect();
                let technology = if parts[0].to_lowercase().contains("swift") {
                    huggingface::types::HfTechnologyKind::SwiftTransformers
                } else {
                    huggingface::types::HfTechnologyKind::Transformers
                };
                let slug = parts.get(1).unwrap_or(&path);
                let data = self.huggingface.get_article(slug, technology).await?;
                Ok(UnifiedSymbolData::from_huggingface(data))
            }
            ProviderType::QuickNode => {
                let data = self.quicknode.get_method(path).await?;
                Ok(UnifiedSymbolData::from_quicknode(data))
            }
            ProviderType::ClaudeAgentSdk => {
                // Parse the path to determine language (e.g., "typescript/query" or "python/ClaudeSDKClient")
                let parts: Vec<&str> = path.splitn(2, '/').collect();
                let language = if parts[0].to_lowercase().contains("python") {
                    claude_agent_sdk::types::AgentSdkLanguage::Python
                } else {
                    claude_agent_sdk::types::AgentSdkLanguage::TypeScript
                };
                let slug = parts.get(1).unwrap_or(&path);
                let data = self.claude_agent_sdk.get_article(slug, language).await?;
                Ok(UnifiedSymbolData::from_claude_agent_sdk(data))
            }
            ProviderType::Vertcoin => {
                let data = self.vertcoin.get_method(path).await?;
                Ok(UnifiedSymbolData::from_vertcoin(data))
            }
            ProviderType::Cuda => {
                let data = self.cuda.get_method(path).await?;
                Ok(UnifiedSymbolData::from_cuda(data))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_clients_creation() {
        let _clients = ProviderClients::new();
    }
}
