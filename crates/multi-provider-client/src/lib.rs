pub mod cocoon;
pub mod telegram;
pub mod ton;
pub mod types;

use std::collections::HashMap;

use anyhow::Result;
use apple_docs_client::AppleDocsClient;

use cocoon::CocoonClient;
use telegram::TelegramClient;
use ton::TonClient;
use types::{ProviderType, UnifiedFrameworkData, UnifiedSymbolData, UnifiedTechnology};

/// All provider clients for simultaneous access
#[derive(Debug)]
pub struct ProviderClients {
    pub apple: AppleDocsClient,
    pub telegram: TelegramClient,
    pub ton: TonClient,
    pub cocoon: CocoonClient,
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
        }
    }

    /// Get technologies from all providers
    pub async fn get_all_technologies(
        &self,
    ) -> Result<HashMap<ProviderType, Vec<UnifiedTechnology>>> {
        let (apple, telegram, ton, cocoon) = tokio::join!(
            self.apple.get_technologies(),
            self.telegram.get_technologies(),
            self.ton.get_technologies(),
            self.cocoon.get_technologies()
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
