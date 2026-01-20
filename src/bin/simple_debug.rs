
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use ethers_core::types::{U256, Address};
    use ethers_core::abi::{encode, Token};
    use ethers_core::utils::keccak256;
    use std::str::FromStr;

    println!("Starting Simple Debug...");

    let registry_addr = "0x000000006551c19487814612e58FE06813775758";
    let implementation_addr = "0xfb28ae9ffc69dd62718a780cb657a59c0b4e7aae8";
    let nft_addr = "0x66994e547cb9014191f50c7c7ee8cf5e80d3b89e"; 
    let chain_id = 84532u64;
    let token_id = 1u64;

    println!("1. Parsing Addresses...");
    let impl_a = Address::from_str(implementation_addr).expect("Impl parse failed");
    let nft_a = Address::from_str(nft_addr).expect("NFT parse failed");
    println!("Addresses parsed OK.");

    println!("2. Hex Decode...");
    let sel = hex::decode("c6bdc908").expect("Selector decode failed");
    println!("Hex decoded OK.");

    println!("3. ABI Encode...");
    let salt_bytes = encode(&[
        Token::Uint(U256::from(chain_id)),
        Token::Address(nft_a),
        Token::Uint(U256::from(token_id))
    ]);
    let salt = keccak256(salt_bytes);
    println!("Salt derived OK.");

    println!("4. Encode Final Payload...");
    let call_data = encode(&[
        Token::Address(impl_a),
        Token::FixedBytes(salt.to_vec()), 
        Token::Uint(U256::from(chain_id)),
        Token::Address(nft_a),
        Token::Uint(U256::from(token_id))
    ]);
    println!("Payload encoded OK.");

    println!("ALL GOOD.");
}

#[cfg(target_arch = "wasm32")]
fn main() {}
