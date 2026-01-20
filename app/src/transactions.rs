use leptos::*;
use ethers_core::types::{TransactionRequest, U256};
use ethers_signers::{LocalWallet, Signer};
use alloy_primitives::hex;
use crate::rpc::Network;

#[derive(Clone)]
pub struct TxFeedback {
    set_status: WriteSignal<String>,
}

impl TxFeedback {
    pub fn new(set_status: WriteSignal<String>) -> Self {
        Self { set_status }
    }

    pub fn set(&self, msg: &str) {
        self.set_status.set(msg.to_string());
    }
}

pub struct LatencyTracker;

impl LatencyTracker {
    pub fn now() -> f64 {
        web_sys::window()
            .and_then(|w| w.performance())
            .map(|p| p.now())
            .unwrap_or(0.0)
    }
}

pub async fn send_with_feedback(
    wallet: &LocalWallet,
    tx: TransactionRequest,
    feedback: TxFeedback,
    conf_msg: &str
) -> Option<f64> { // Returns latency in ms
    let start = LatencyTracker::now();
    feedback.set("Preparing...");

    // 1. Get Nonce
    let nonce_res = crate::rpc::get_transaction_count(&format!("{:?}", wallet.address()), Network::BaseSepolia).await;
    let nonce = match nonce_res {
        Ok(n) => n,
        Err(e) => { feedback.set(&format!("Nonce Error: {}", e)); return None; }
    };

    // 2. Sign
    feedback.set("Signing...");
    
    // Ensure Nonce is set
    let mut tx = tx.clone();
    tx.nonce = Some(U256::from(nonce));
    tx.chain_id = Some(ethers_core::types::U64::from(84532)); // Hardcoded Base Sepolia

    // Set Gas Price if not set (Fix "Transaction Underpriced")
    if tx.gas_price.is_none() {
        feedback.set("Fetching Gas Price...");
        match crate::rpc::get_gas_price(Network::BaseSepolia).await {
            Ok(gp) => {
                // Add 20% buffer to ensure inclusion
                let effective = gp + (gp / 5);
                tx.gas_price = Some(U256::from(effective));
            },
            Err(e) => {
                 feedback.set(&format!("Gas Price Error: {}", e));
                 return None;
            }
        }
    }



    // Estimate Gas if not set
    if tx.gas.is_none() {
        feedback.set("Estimating Gas...");
        let tx_json = serde_json::to_value(&tx).unwrap_or(serde_json::json!({}));
        match crate::rpc::estimate_gas(tx_json, Network::BaseSepolia).await {
            Ok(est) => {
                 // Add 20% buffer
                 let gas_limit = est + (est / 5);
                 tx.gas = Some(gas_limit);
            },
            Err(e) => {
                feedback.set(&format!("Gas Est Error: {}", e));
                return None; 
            }
        }

    }
    
    // Actually, properly handling borrowing/mutability:
    // We already signed at line 57. If we updated gas, that signature is invalid.
    // Let's just re-sign if we estimated gas.
    
    // Sign
    let signature = match wallet.sign_transaction(&tx.clone().into()).await {
        Ok(s) => s,
        Err(e) => { feedback.set(&format!("Sign Error: {}", e)); return None; }
    };

    let rlp = tx.rlp_signed(&signature);
    let rlp_hex = format!("0x{}", hex::encode(rlp));

    // 3. Send
    feedback.set("Sending...");
    match crate::rpc::send_raw_transaction(&rlp_hex, Network::BaseSepolia).await {
        Ok(hash) => {
            feedback.set(&format!("Sent! Tx: {}. Waiting...", hash));
            
            // 4. Poll for Receipt
            let mut attempts = 0;
            loop {
                gloo_timers::future::TimeoutFuture::new(2000).await; // 2s
                let receipt = crate::rpc::get_transaction_receipt(&hash, Network::BaseSepolia).await;
                if let Ok(r) = receipt {
                    if !r.is_null() {
                        if r["status"].as_str() == Some("0x1") {
                            let end = LatencyTracker::now();
                            let latency = end - start;
                            feedback.set(&format!("{} ({:.0}ms)", conf_msg, latency));
                            return Some(latency);
                        } else {
                            feedback.set("Failed on-chain.");
                            break;
                        }
                    }
                }
                attempts += 1;
                if attempts > 30 { // 60s
                    feedback.set("Timeout.");
                    break;
                }
            }
        },
        Err(e) => feedback.set(&format!("Send Error: {}", e)),
    }
    None
}
