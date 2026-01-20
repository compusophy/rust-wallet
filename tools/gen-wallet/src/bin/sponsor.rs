use ethers::prelude::*;
use std::convert::TryFrom;
use std::env;
use std::sync::Arc;
use std::time::Duration;
use anyhow::{Result, anyhow};

const RPC_URL: &str = "https://sepolia.base.org";
const PRIVATE_KEY: &str = "95d8e530156677e0837cd040764e1b8feb54e4a7845e9fbe8937fa0a58f1ea67";

#[tokio::main]
async fn main() -> Result<()> {
    // Parse args
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: cargo run --bin sponsor <TARGET_ADDRESS>");
        return Ok(());
    }
    let target_addr: Address = args[1].parse().map_err(|e| anyhow!("Invalid address: {}", e))?;

    let provider = Provider::<Http>::try_from(RPC_URL)?
        .interval(Duration::from_millis(100));
    let wallet: LocalWallet = PRIVATE_KEY.parse()?;
    let chain_id = provider.get_chainid().await?;
    let wallet = wallet.with_chain_id(chain_id.as_u64());

    println!("Sponsor (Deployer): {:?}", wallet.address());
    println!("Target (User):      {:?}", target_addr);

    let client = SignerMiddleware::new(provider, wallet);

    // Amount to sponsor: 0.005 ETH
    let amount_to_send = U256::from(5000000000000000u64); // 0.005 ETH

    let balance = client.get_balance(client.address(), None).await?;
    println!("Sponsor Balance: {} Wei", balance);

    if balance < amount_to_send {
        return Err(anyhow!("Insufficient details to sponsor funds."));
    }

    // EIP-1559 fees
    let (max_fee, max_priority) = client.provider().estimate_eip1559_fees(None).await?;
    
    // Construct Tx
    let tx = Eip1559TransactionRequest::new()
        .to(target_addr)
        .value(amount_to_send)
        .max_fee_per_gas(max_fee)
        .max_priority_fee_per_gas(max_priority)
        .chain_id(chain_id.as_u64());

    println!("Sending 0.005 ETH...");
    let pending_tx = client.send_transaction(tx, None).await?;
    let receipt = pending_tx.await?.ok_or(anyhow!("No receipt"))?;

    println!("Success! Hash: {:?}", receipt.transaction_hash);
    println!("User {:?} has been sponsored.", target_addr);

    Ok(())
}
