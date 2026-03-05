use ethers::prelude::*;
use std::sync::Arc;
use eyre::{Result, WrapErr};
use dotenv::dotenv;
use std::env;

mod constants;
mod scanner;
mod decoder;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Load Environment Variables (.env file check)
    dotenv().ok();
    
    // 2. Initialize Layer 1: The Nerve System
    let rpc_url = env::var("RPC_WSS_URL")
        .wrap_err("❌ ERROR: .env file mein RPC_WSS_URL nahi mila!")?;

    println!("📡 Connecting to Ethereum Mempool via QuickNode...");

    // 3. Establish WebSocket Connection
    let provider = Provider::<Ws>::connect(&rpc_url)
        .await
        .wrap_err("❌ ERROR: QuickNode se connect nahi ho pa raha. URL check karein!")?;
    
    let provider = Arc::new(provider);
    println!("✅ Layer 1 Active: Nerve System Connected!");
    println!("🕵️ Surveillance Started: Watching for Shadow Trades...\n");

    // 4. Start the Scanning Loop
    let mut stream = provider.subscribe_pending_txs().await
        .wrap_err("❌ ERROR: Mempool subscription failed!")?;

    while let Some(tx_hash) = stream.next().await {
        // Filhal hum sirf hash dekh rahe hain, yahan se Layer 2 ka kaam shuru hoga
        println!("🚀 New Transaction Spotted: {:?}", tx_hash);
    }

    Ok(())
}