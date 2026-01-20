
#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
pub async fn main() {
    use ethers_core::types::U256;
    use ethers_core::abi::{encode, Token};
    use ethers_core::utils::keccak256;
    use serde_json::json; 
    use reqwest::Client;

    println!("Starting Debug...");
    
    let registry_addr = "0x000000006551c19487814612e58FE06813775758";
    let implementation_addr = "0xfb28ae9ffc69dd62718a780cb657a59c0b4e7aae";
    let nft_addr = "0x66994e547cb9014191f50c7c7ee8cf5e80d3b89e"; 
    let chain_id = 84532u64;
    let token_id = 1u64;

    // 1. Check Implementation Code
    println!("Checking Implementation Code...");
    {
        let client = Client::new();
        let res = client.post("https://sepolia.base.org")
            .json(&json!({
                "jsonrpc": "2.0",
                "method": "eth_getCode",
                "params": [implementation_addr, "latest"],
                "id": 1
            }))
            .send().await.expect("RPC failed");
        let text = res.text().await.expect("Text failed");
        println!("Impl Code Check: {}", text);
    }

    // 2. Simulate createAccount with Random Salt
    println!("Simulating createAccount (Random Salt)...");
    let random_salt = keccak256("random_salt_999");
    let selector = hex::decode("025e3789").unwrap();
    let mut full_data = selector;
    full_data.extend(encode(&[
         Token::Address(implementation_addr.parse().unwrap()),
         Token::FixedBytes(random_salt.to_vec()),
         Token::Uint(U256::from(chain_id)),
         Token::Address(nft_addr.parse().unwrap()),
         Token::Uint(U256::from(token_id))
    ]));
    let full_data_hex = format!("0x{}", hex::encode(full_data));

    {
        let client = Client::new();
        let res = client.post("https://sepolia.base.org")
            .json(&json!({
                "jsonrpc": "2.0",
                "method": "eth_call",
                "params": [{
                    "to": registry_addr,
                    "data": full_data_hex
                }, "latest"],
                "id": 1
            }))
            .send().await.expect("RPC failed");
        let text = res.text().await.expect("Text failed");
        println!("createAccount Result: {}", text);
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {}
