use gloo_net::http::Request;

use serde_json::json;
// use leptos::*;

pub enum Network {
    BaseSepolia,

}

impl Network {
    pub fn rpc_url(&self) -> &'static str {
        match self {
            Network::BaseSepolia => "https://sepolia.base.org",

        }
    }
}

pub async fn eth_call(network: Network, method: &str, params: serde_json::Value) -> Result<serde_json::Value, String> {
    let rpc_url = network.rpc_url();
    
    let body = json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        // Random ID to avoid caching potentially
        "id": (js_sys::Math::random() * 1000.0) as u32
    });

    let resp = Request::post(rpc_url)
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    
    if let Some(err) = json.get("error") {
        return Err(err.to_string());
    }

    Ok(json)
}

pub async fn get_balance(address: &str, network: Network) -> String {
    let params = json!([address, "latest"]);
    match eth_call(network, "eth_getBalance", params).await {
        Ok(val) => {
            let hex = val["result"].as_str().unwrap_or("0x0");
            let wei = u128::from_str_radix(hex.trim_start_matches("0x"), 16).unwrap_or(0);
            format!("{:.4} ETH", wei as f64 / 1e18)
        },
        Err(_) => "Error".to_string()
    }
}

pub async fn get_transaction_count(address: &str, network: Network) -> Result<u64, String> {
    let params = json!([address, "latest"]);
    let val = eth_call(network, "eth_getTransactionCount", params).await?;
    let hex = val["result"].as_str().unwrap_or("0x0");
    u64::from_str_radix(hex.trim_start_matches("0x"), 16).map_err(|e| e.to_string())
}

pub async fn get_gas_price(network: Network) -> Result<u128, String> {
    let val = eth_call(network, "eth_gasPrice", json!([])).await?;
    let hex = val["result"].as_str().unwrap_or("0x0");
    u128::from_str_radix(hex.trim_start_matches("0x"), 16).map_err(|e| e.to_string())
}

pub async fn send_raw_transaction(hex_tx: &str, network: Network) -> Result<String, String> {
    let params = json!([hex_tx]);
    let val = eth_call(network, "eth_sendRawTransaction", params).await?;
    val["result"].as_str().map(|s| s.to_string()).ok_or("No tx hash returned".to_string())
}

pub async fn get_transaction_receipt(tx_hash: &str, network: Network) -> Result<serde_json::Value, String> {
    let params = json!([tx_hash]);
    let val = eth_call(network, "eth_getTransactionReceipt", params).await?;
    Ok(val["result"].clone())
}


