
#[cfg(not(target_arch = "wasm32"))]
mod host_debug {
    use ethers_core::types::{U256, Bytes};
    use ethers_core::abi::{encode, Token};
    use ethers_core::utils::keccak256;
    use hex;
    use reqwest::Client;
    use serde_json::json;

    async fn eth_call(to: &str, data: &str) -> String {
        let client = Client::new();
        let res = client.post("https://sepolia.base.org")
            .json(&json!({
                "jsonrpc": "2.0",
                "method": "eth_call",
                "params": [{
                    "to": to,
                    "data": data
                }, "latest"],
                "id": 1
            }))
            .send()
            .await
            .expect("RPC Request failed");
        
        let status = res.status();
        let text = res.text().await.expect("Get text failed");
        println!("RPC Status: {}", status);
        println!("RPC Body: {}", text);

        let body: serde_json::Value = serde_json::from_str(&text).expect("Parse JSON failed");
        if let Some(err) = body.get("error") {
             println!("RPC Error field: {:?}", err);
             // panic!("RPC Error: {:?}", err); // Don't panic, just print
             return "0x".to_string();
        }
        body["result"].as_str().unwrap_or("0x").to_string()
    }

    #[tokio::main]
    pub async fn main() {
        println!("Debugging ERC-6551 Derivation...");
        
        let registry_addr = "0x000000006551c19487814612e58FE06813775758";
        let implementation_addr = "0xfb28ae9ffc69dd62718a780cb657a59c0b4e7aae8";
        let nft_addr = "0x66994e547cb9014191f50c7c7ee8cf5e80d3b89e"; 
        let chain_id = 84532u64;
        let token_id = 1u64;

        println!("Step 1: Encoding Salt...");
        // 1. Calculate Salt (Same logic as wallet.rs)
        let salt_bytes = encode(&[
            Token::Uint(U256::from(chain_id)),
            Token::Address(nft_addr.parse().expect("NFT Addr Parse Failed")),
            Token::Uint(U256::from(token_id))
        ]);
        let salt = keccak256(salt_bytes);
        println!("Salt: 0x{}", hex::encode(salt));

        println!("Step 2: Encoding Calldata...");
        // 2. Encode Call
        let call_data = encode(&[
            Token::Address(implementation_addr.parse().expect("Impl Addr Parse Failed")),
            Token::FixedBytes(salt.to_vec()), 
            Token::Uint(U256::from(chain_id)),
            Token::Address(nft_addr.parse().unwrap()),
            Token::Uint(U256::from(token_id))
        ]);
        
        println!("Step 3: Appending Selector...");
        let selector = hex::decode("c6bdc908").expect("Decode selector failed");
        let mut full_data = selector;
        full_data.extend(call_data);
        let full_data_hex = format!("0x{}", hex::encode(full_data));
        
        println!("Calldata: {}", full_data_hex);
        
        println!("Step 4: Making RPC Call...");
        let result = eth_call(registry_addr, &full_data_hex).await;
        println!("Result: {}", result);
        
        if result == "0x" {
             println!("FAILED: Registry returned empty.");
        } else {
             let len = result.len();
             let addr = &result[len-40..];
             println!("Computed TBA: 0x{}", addr);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    host_debug::main();
}

#[cfg(target_arch = "wasm32")]
fn main() {}
