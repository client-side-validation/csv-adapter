//! Asset valuation.
//!
//! Calculates current value of owned assets.

use csv_adapter_core::Chain;
use reqwest::Client;
use serde::{Serialize, Deserialize};

/// Price data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceData {
    /// Asset symbol (BTC, ETH, SUI, APT)
    pub symbol: String,
    /// Price in USD
    pub price_usd: f64,
    /// 24h change percentage
    pub change_24h: f64,
    /// Last updated timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Asset valuation.
pub struct AssetValuation;

impl AssetValuation {
    /// Get current price for a chain's native token.
    pub async fn get_price(chain: Chain) -> Result<PriceData, String> {
        let symbol = match chain {
            Chain::Bitcoin => "bitcoin",
            Chain::Ethereum => "ethereum",
            Chain::Sui => "sui",
            Chain::Aptos => "aptos",
            Chain::Solana => "solana",
        };

        Self::fetch_price(symbol).await
    }

    /// Fetch price from CoinGecko or similar API.
    async fn fetch_price(symbol: &str) -> Result<PriceData, String> {
        let client = Client::new();
        let url = format!(
            "https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies=usd&include_24hr_change=true",
            symbol
        );

        let response: serde_json::Value = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch price: {}", e))?
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        let data = &response[symbol];
        let price_usd = data["usd"]
            .as_f64()
            .ok_or_else(|| "Invalid price data".to_string())?;

        let change_24h = data["usd_24h_change"]
            .as_f64()
            .unwrap_or(0.0);

        Ok(PriceData {
            symbol: symbol.to_uppercase(),
            price_usd,
            change_24h,
            updated_at: chrono::Utc::now(),
        })
    }

    /// Calculate total portfolio value.
    pub async fn calculate_portfolio_value(
        balances: &[(Chain, u64, u32)], // (chain, balance_in_smallest_unit, decimals)
    ) -> Result<f64, String> {
        let mut total_usd = 0.0;

        for (chain, balance, decimals) in balances {
            let price = Self::get_price(*chain).await?;
            let balance_float = *balance as f64 / 10u64.pow(*decimals as u32) as f64;
            total_usd += balance_float * price.price_usd;
        }

        Ok(total_usd)
    }

    /// Get prices for all chains.
    pub async fn get_all_prices(&self) -> Result<Vec<PriceData>, String> {
        let chains = [Chain::Bitcoin, Chain::Ethereum, Chain::Sui, Chain::Aptos, Chain::Solana];
        let mut prices = Vec::new();

        for chain in chains {
            if let Ok(price) = Self::get_price(chain).await {
                prices.push(price);
            }
        }

        Ok(prices)
    }
}
