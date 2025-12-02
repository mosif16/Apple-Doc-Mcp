//! Test script for multi-provider clients
//!
//! Run with: cargo run --example test_providers

use multi_provider_client::{ProviderClients, telegram::TelegramClient, ton::TonClient, cocoon::CocoonClient};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Multi-Provider Documentation Test ===\n");

    // Test Telegram Client
    println!("📱 Testing Telegram Bot API...");
    let telegram = TelegramClient::new();
    match telegram.get_technologies().await {
        Ok(techs) => {
            println!("   ✅ Found {} Telegram categories:", techs.len());
            for tech in &techs {
                println!("      - {} ({} items)", tech.title, tech.item_count);
            }
        }
        Err(e) => println!("   ❌ Error: {e}"),
    }

    // Test search
    match telegram.search("send").await {
        Ok(results) => {
            println!("   🔍 Search 'send': {} results", results.len());
            for item in results.iter().take(5) {
                println!("      - {} ({})", item.name, item.kind);
            }
        }
        Err(e) => println!("   ❌ Search error: {e}"),
    }

    println!();

    // Test TON Client
    println!("💎 Testing TON Blockchain API...");
    let ton = TonClient::new();
    match ton.get_technologies().await {
        Ok(techs) => {
            println!("   ✅ Found {} TON categories:", techs.len());
            for tech in techs.iter().take(10) {
                println!("      - {} ({} endpoints)", tech.title, tech.endpoint_count);
            }
            if techs.len() > 10 {
                println!("      ... and {} more", techs.len() - 10);
            }
        }
        Err(e) => println!("   ❌ Error: {e}"),
    }

    // Test search
    match ton.search("account").await {
        Ok(results) => {
            println!("   🔍 Search 'account': {} results", results.len());
            for ep in results.iter().take(5) {
                println!("      - {} {} ({})", ep.method.to_uppercase(), ep.path, ep.operation_id);
            }
        }
        Err(e) => println!("   ❌ Search error: {e}"),
    }

    println!();

    // Test Cocoon Client
    println!("🥥 Testing Cocoon Documentation...");
    let cocoon = CocoonClient::new();
    match cocoon.get_technologies().await {
        Ok(techs) => {
            println!("   ✅ Found {} Cocoon sections:", techs.len());
            for tech in &techs {
                println!("      - {}", tech.title);
            }
        }
        Err(e) => println!("   ❌ Error: {e}"),
    }

    println!();

    // Test unified ProviderClients
    println!("🌐 Testing Unified ProviderClients...");
    let clients = ProviderClients::new();
    match clients.get_all_technologies().await {
        Ok(all_techs) => {
            println!("   ✅ All providers loaded:");
            for (provider, techs) in &all_techs {
                println!("      - {}: {} technologies", provider, techs.len());
            }
        }
        Err(e) => println!("   ❌ Error: {e}"),
    }

    println!("\n=== Test Complete ===");
    Ok(())
}
