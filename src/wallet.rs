use leptos::*;
use gloo_storage::{LocalStorage, Storage};
use serde::{Deserialize, Serialize};
use crate::rpc::{Network, get_balance};
use rand::rngs::SmallRng;
use rand::{RngCore, SeedableRng};
use alloy_primitives::hex;
use wasm_bindgen::JsCast;

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Keystore {
    pub private_key: String,
    pub address: String,
    pub smart_account: Option<String>,
}

#[component]
pub fn WalletView() -> impl IntoView {
    let (keystore, set_keystore) = create_signal(Keystore::default());
    let (status, set_status) = create_signal("Ready".to_string());
    
    // Latency Tracking
    let (last_latency, set_last_latency) = create_signal(None::<f64>);
    
    // Balances
    let (bal_sepolia, set_bal_sepolia) = create_signal("...".to_string());

    
    // TBA Balances
    let (tba_bal_sepolia, set_tba_bal_sepolia) = create_signal("...".to_string());

    
    // Refresh Trigger
    let (refresh_trigger, set_refresh_trigger) = create_signal(0u64);
    
    // UI State
    let (show_clear_confirm, set_show_clear_confirm) = create_signal(false);

    // Sponsor UI State
    let (show_sponsor_modal, set_show_sponsor_modal) = create_signal(false);
    let (pin_input, set_pin_input) = create_signal("".to_string());
    
    // Wallet Menu State
    let (show_wallet_menu, set_show_wallet_menu) = create_signal(false);
    let (show_brain_menu, set_show_brain_menu) = create_signal(false);
    
    // Send State (Device)
    let (show_device_send, set_show_device_send) = create_signal(false);
    let (device_recipient, set_device_recipient) = create_signal("".to_string());
    let (device_amount, set_device_amount) = create_signal("".to_string());

    // Send State (Smart Account)
    let (show_sa_send, set_show_sa_send) = create_signal(false);
    let (sa_recipient, set_sa_recipient) = create_signal("".to_string());
    let (sa_amount, set_sa_amount) = create_signal("".to_string());

    // Load from local storage on init
    create_effect(move |_| {
        if let Ok(k) = LocalStorage::get::<Keystore>("diamond_wallet_keystore") {
            set_keystore.set(k);
        }
    });

    // Refresh balances when keystore changes OR trigger updates
    create_effect(move |_| {
        let k = keystore.get();
        let _ = refresh_trigger.get(); // Dependency
        
        if !k.address.is_empty() {
            // Fetch Signer Balances
            spawn_local(async move {
                let b_sep = get_balance(&k.address, Network::BaseSepolia).await;
                set_bal_sepolia.set(b_sep);
                

            });
            
            // Fetch TBA Balances
            if let Some(tba) = k.smart_account {
                spawn_local(async move {
                     let b_sep = get_balance(&tba, Network::BaseSepolia).await;
                     set_tba_bal_sepolia.set(b_sep);

                });
            }
        }
    });

    let generate_wallet = move |_| {
        set_status.set("Generating secure key...".to_string());
        
        // Generate real random key
        let mut rng = SmallRng::from_entropy();
        let mut pk_bytes = [0u8; 32];
        rng.fill_bytes(&mut pk_bytes);
        let pk_hex = hex::encode(pk_bytes);
        
        // Properly derive address using ethers
        use ethers_signers::{LocalWallet, Signer};
        
        let wallet = LocalWallet::from_bytes(&pk_bytes).expect("Valid bytes");
        let addr = wallet.address(); // This is the REAL address
        let addr_hex = format!("{:?}", addr);

        let new_ks = Keystore {
            private_key: format!("0x{}", pk_hex),
            address: addr_hex,
            smart_account: None,
        };
        
        let _ = LocalStorage::set("diamond_wallet_keystore", &new_ks);
        set_keystore.set(new_ks);
        set_status.set("New Wallet Generated".to_string());
    };

    // Sweep Funds Logic
    let sweep_funds = move |_| {
        let k = keystore.get();
        if k.private_key.is_empty() { return; }
        
        spawn_local(async move {
            let feedback = crate::transactions::TxFeedback::new(set_status);
            feedback.set("Preparing to sweep funds...");
            
            // Importing ethers types here to construct tx
            use ethers_core::types::{TransactionRequest, U256};
            use ethers_signers::{LocalWallet, Signer};
            
            let pk = k.private_key.trim_start_matches("0x");
            let wallet: LocalWallet = match pk.parse() {
                Ok(w) => w,
                Err(e) => {
                    feedback.set(&format!("Error parsing key: {}", e));
                    return;
                }
            };
            let wallet = wallet.with_chain_id(84532u64);
            
             // Raw balance check via manual RPC call
            let params = serde_json::json!([k.address, "latest"]);
            let bal_res = crate::rpc::eth_call(Network::BaseSepolia, "eth_getBalance", params).await;
            let balance = match bal_res {
                Ok(v) => {
                   let hex = v["result"].as_str().unwrap_or("0");
                   u128::from_str_radix(hex.trim_start_matches("0x"), 16).unwrap_or(0)
                },
                Err(_) => 0
            };

            let gas_price = crate::rpc::get_gas_price(Network::BaseSepolia).await.unwrap_or(0);
            let effective_gas_price = gas_price + (gas_price / 10);
            let gas_limit = 21000u64;
            let cost = effective_gas_price * (gas_limit as u128);
            let safety_buffer = 10_000u128;
            let total_deduct = cost + safety_buffer;

            if balance <= total_deduct {
                feedback.set("Insufficient funds to cover gas.");
                return;
            }
            
            let send_amount = balance - total_deduct;
            let deployer_addr: ethers_core::types::Address = "0x769c18faa2e2e833a262c2ff9f6e1a9e99e52c58".parse().unwrap();
            
            let tx = TransactionRequest::new()
                .to(deployer_addr)
                .value(U256::from(send_amount))
                .gas(U256::from(gas_limit))
                .gas_price(U256::from(effective_gas_price));
                
            let lat = crate::transactions::send_with_feedback(&wallet, tx, feedback, "Swept!").await;
            set_last_latency.set(lat);
            set_refresh_trigger.update(|v| *v += 1);
        });
    };

    // Mint Identity Logic
    let mint_identity = move |_| {
        let k = keystore.get();
        if k.private_key.is_empty() { return; }
        
        spawn_local(async move {
            let feedback = crate::transactions::TxFeedback::new(set_status);
            feedback.set("Initializing NFT Mint...");
            
            let nft_addr: ethers_core::types::Address = "0x66994e547cb9014191f50c7c7ee8cf5e80d3b89e".parse().unwrap();
            
            use ethers_core::types::{TransactionRequest, U256, Bytes};
            use ethers_signers::{LocalWallet, Signer};

            let pk = k.private_key.trim_start_matches("0x");
            let wallet: LocalWallet = pk.parse().unwrap();
            let wallet = wallet.with_chain_id(84532u64);
            
            let gas_price = crate::rpc::get_gas_price(Network::BaseSepolia).await.unwrap_or(0);
            
            // Selector for mint() is 0x1249c58b
            let data = Bytes::from(hex::decode("1249c58b").unwrap());
            
            let tx = TransactionRequest::new()
                .to(nft_addr)
                .value(U256::zero())
                .gas(U256::from(200_000u64)) 
                .gas_price(U256::from(gas_price))
                .data(data);
                
            let lat = crate::transactions::send_with_feedback(&wallet, tx, feedback, "Mint Confirmed! Compute TBA now.").await;
            set_last_latency.set(lat);
            set_refresh_trigger.update(|v| *v += 1);
        });
    };

    let compute_tba = move |_| {
         let k = keystore.get();
         if k.address.is_empty() { return; }
         
         spawn_local(async move {
            leptos::logging::log!("Starting TBA Compute...");
            set_status.set("Locating TBA Address...".to_string());
            
            // PLACEHOLDERS
            let registry_addr = "0x000000006551c19487814612e58FE06813775758";
            let implementation_addr = "0xfb28ae9ffc69dd62718a780cb657a59c0b4e7aae"; 
            let nft_addr = "0x66994e547cb9014191f50c7c7ee8cf5e80d3b89e"; 
            let chain_id = 84532u64;
            let token_id = 1u64; 

            use ethers_core::abi::{encode, Token};
            use ethers_core::utils::keccak256;
            use ethers_core::types::U256;
            
            // 1. Calculate Salt
            let salt_bytes = encode(&[
                Token::Uint(U256::from(chain_id)),
                Token::Address(nft_addr.parse().unwrap()),
                Token::Uint(U256::from(token_id))
            ]);
            let salt = keccak256(salt_bytes);
            
            // 2. Creation Code (ERC-6551 v3 Standard)
            let mut creation_code = Vec::new();
            creation_code.extend(hex::decode("3d60ad80600a3d3981f3363d3d373d3d3d363d73").unwrap());
            creation_code.extend(hex::decode(implementation_addr.trim_start_matches("0x")).unwrap());
            creation_code.extend(hex::decode("5af43d82803e903d91602b57fd5bf3").unwrap()); 
            
            // Footer Data
            let footer_data = encode(&[
                Token::FixedBytes(salt.to_vec()),
                Token::Uint(U256::from(chain_id)),
                Token::Address(nft_addr.parse().unwrap()),
                Token::Uint(U256::from(token_id))
            ]);
            creation_code.extend(footer_data);
            
            // 3. Init Code Hash
            let init_code_hash = keccak256(&creation_code);
            
            // 4. Create2 Address
            let registry_vec = hex::decode(registry_addr.trim_start_matches("0x")).unwrap();
            let mut input = vec![0xff];
            input.extend(registry_vec);
            input.extend(salt); 
            input.extend(init_code_hash);
            
            let raw_addr_hash = keccak256(&input);
            let addr_bytes = &raw_addr_hash[12..32];
            let addr_hex = format!("0x{}", hex::encode(addr_bytes));
            
            let mut new_ks = k.clone();
            new_ks.smart_account = Some(addr_hex.clone());
            let _ = LocalStorage::set("diamond_wallet_keystore", &new_ks);
            set_keystore.set(new_ks);
            set_status.set(format!("TBA Computed: {}", addr_hex));
            set_refresh_trigger.update(|v| *v += 1);
         });
    };

    // Send ETH (Device)
    let send_eth_device = move |_| {
        spawn_local(async move {
            let to = device_recipient.get();
            let amt_str = device_amount.get();
            
            if to.is_empty() || amt_str.is_empty() {
                set_status.set("Invalid Send Inputs".to_string());
                return;
            }
            
            use std::str::FromStr;
            use ethers_core::utils::parse_ether;
            use ethers_core::types::{Address, TransactionRequest};
            use ethers_signers::{LocalWallet, Signer};

            let to_addr = match Address::from_str(&to) {
                Ok(a) => a,
                Err(_) => { set_status.set("Invalid Recipient Address".to_string()); return; }
            };
            
            let val = match parse_ether(&amt_str) {
                Ok(v) => v,
                Err(_) => { set_status.set("Invalid Amount".to_string()); return; }
            };

            let feedback = crate::transactions::TxFeedback::new(set_status);
            feedback.set("Sending ETH...");
            
            let k = keystore.get();
            let pk = k.private_key.trim_start_matches("0x");
            let wallet: LocalWallet = pk.parse().unwrap();
            let wallet = wallet.with_chain_id(84532u64);
            
            // Construct TX (no provider needed here, send_with_feedback handles it via raw RPC)
            let tx = TransactionRequest::new().to(to_addr).value(val);
            
            let lat = crate::transactions::send_with_feedback(&wallet, tx, feedback, "ETH Sent!").await;
            
            if lat.is_some() {
                 set_show_device_send.set(false);
                 set_device_recipient.set("".to_string());
                 set_device_amount.set("".to_string());
                 set_refresh_trigger.update(|v| *v += 1);
            }
        });
    };

    // Send ETH (Smart Account)
    let send_eth_sa = move |_| {
        spawn_local(async move {
            let to = sa_recipient.get();
            let amt_str = sa_amount.get();
            
            if to.is_empty() || amt_str.is_empty() {
                 set_status.set("Invalid Send Inputs".to_string());
                 return;
            }

            use std::str::FromStr;
            use ethers_core::utils::parse_ether;
            use ethers_core::types::Address;

             let to_addr = match Address::from_str(&to) {
                Ok(a) => a,
                Err(_) => { set_status.set("Invalid Recipient Address".to_string()); return; }
            };
            
            let val = match parse_ether(&amt_str) {
                Ok(v) => v,
                Err(_) => { set_status.set("Invalid Amount".to_string()); return; }
            };

            let feedback = crate::transactions::TxFeedback::new(set_status);
            feedback.set("Preparing UserOp (Send ETH)...");

             let k = keystore.get();
             if let Some(tba_addr) = k.smart_account {
                  let tba: Address = tba_addr.parse().unwrap();
                  
                  // ERC-6551 V3 execute(to, value, data, operation)
                  // Function Selector: execute(address,uint256,bytes,uint8) => 0 or specifically 
                  let data = ethers_core::types::Bytes::from(vec![]);
                  let operation = 0u8; // Call

                   use ethers_core::abi::{encode, Token};
                   let func_selector = ethers_core::utils::id("execute(address,uint256,bytes,uint8)");
                   let mut calldata = func_selector[0..4].to_vec();
                   let args = encode(&[
                       Token::Address(to_addr),
                       Token::Uint(val),
                       Token::Bytes(data.to_vec()),
                       Token::Uint(operation.into())
                   ]);
                   calldata.extend(args);

                   // Create wallet
                   use ethers_signers::{LocalWallet, Signer};
                   use ethers_core::types::TransactionRequest;
                   
                   let pk = k.private_key.trim_start_matches("0x");
                   let wallet: LocalWallet = pk.parse().unwrap();
                   let wallet = wallet.with_chain_id(84532u64);

                   // Send transaction to TBA from signer
                   let _ = crate::transactions::send_with_feedback(
                       &wallet,
                       TransactionRequest::new().to(tba).data(calldata).value(0), // 0 value to TBA, TBA sends value to dest
                       feedback,
                       "Sent ETH via TBA!"
                   ).await;
                   
                   set_refresh_trigger.update(|v| *v += 1);
                   set_show_sa_send.set(false);
                   set_sa_recipient.set("".to_string());
                   set_sa_amount.set("".to_string());

             } else {
                 set_status.set("No Smart Account".to_string());
             }
        });
    };

    let clear_wallet = move |_| {
        let k = keystore.get();
        if k.private_key.is_empty() { return; }
        
        spawn_local(async move {
             let feedback = crate::transactions::TxFeedback::new(set_status);
             use ethers_core::types::{TransactionRequest, U256};
             use ethers_signers::{LocalWallet, Signer};
             use ethers_core::abi::{encode, Token};

             let pk = k.private_key.trim_start_matches("0x");
             // If key invalid, just force clear
             if let Ok(w) = pk.parse::<LocalWallet>() {
                let wallet = w.with_chain_id(84532u64);
                let deployer_addr: ethers_core::types::Address = "0x769c18faa2e2e833a262c2ff9f6e1a9e99e52c58".parse().unwrap();
                
                // 1. Drain TBA if it exists
                if let Some(tba) = k.smart_account {
                    feedback.set("Checking TBA balance...");
                    let params = serde_json::json!([tba, "latest"]);
                    if let Ok(v) = crate::rpc::eth_call(Network::BaseSepolia, "eth_getBalance", params).await {
                         let hex = v["result"].as_str().unwrap_or("0");
                         let tba_bal = u128::from_str_radix(hex.trim_start_matches("0x"), 16).unwrap_or(0);
                         
                         if tba_bal > 0 {
                             feedback.set("Draining TBA...");
                             let execute_selector = hex::decode("24857bd4").unwrap();
                             let inner_data = encode(&[
                                Token::Address(deployer_addr),
                                Token::Uint(U256::from(tba_bal)),
                                Token::Bytes(vec![]), 
                                Token::Uint(U256::from(0u8)) 
                            ]);
                            let mut tx_data = execute_selector;
                            tx_data.extend(inner_data);
                            
                            let gas_price = crate::rpc::get_gas_price(Network::BaseSepolia).await.unwrap_or(0);
                            let effective_gas_price = gas_price + (gas_price / 10);
                            
                             let tx = TransactionRequest::new()
                                .to(tba.parse::<ethers_core::types::Address>().unwrap())
                                .value(0) 
                                .data(tx_data)
                                .gas(U256::from(200000u64)) // TBA overhead
                                .gas_price(U256::from(effective_gas_price));
                                
                             let _ = crate::transactions::send_with_feedback(&wallet, tx, feedback.clone(), "TBA Drained!").await;
                         }
                    }
                }

                // 2. Drain Signer
                feedback.set("Checking Signer balance...");
                let params = serde_json::json!([k.address, "latest"]);
                if let Ok(bal_res) = crate::rpc::eth_call(Network::BaseSepolia, "eth_getBalance", params).await {
                     let hex = bal_res["result"].as_str().unwrap_or("0");
                     let balance = u128::from_str_radix(hex.trim_start_matches("0x"), 16).unwrap_or(0);
                     
                     let gas_price = crate::rpc::get_gas_price(Network::BaseSepolia).await.unwrap_or(0);
                     let effective_gas_price = gas_price + (gas_price / 10);
                     let gas_limit = 21000u64;
                     let cost = effective_gas_price * (gas_limit as u128);
                     let safety_buffer = 1000u128; // Tiny buffer
                     
                     if balance > (cost + safety_buffer) {
                          feedback.set("Draining Signer...");
                          let send_amount = balance - (cost + safety_buffer);
                          
                          let tx = TransactionRequest::new()
                             .to(deployer_addr)
                             .value(U256::from(send_amount))
                             .gas(U256::from(gas_limit))
                             .gas_price(U256::from(effective_gas_price));
                             
                         let _ = crate::transactions::send_with_feedback(&wallet, tx, feedback, "Signer Drained!").await;
                     }
                }
             }

            // 3. Perform Clear
            let _ = LocalStorage::delete("diamond_wallet_keystore");
            set_keystore.set(Keystore::default());
            set_bal_sepolia.set("...".to_string());

            set_show_clear_confirm.set(false);
            set_status.set("Wallet Cleared".to_string());
        });
    };

    fn copy_to_clipboard(text: String) {
        if let Some(window) = web_sys::window() {
             let navigator = window.navigator();
             let clipboard = navigator.clipboard();
             let _ = clipboard.write_text(&text);
        }
    }

    let request_sponsor = move |_| {
        let pin = pin_input.get();
        if pin != "1337" {
             set_status.set("Incorrect PIN.".to_string());
             return;
         }
         
         // CHECK FOR KEY
         // Read from compile-time env var
         let faucet_key_env = option_env!("FAUCET_KEY").unwrap_or("");
         if faucet_key_env.is_empty() {
             set_status.set("Demo Faucet Key Missing (Set FAUCET_KEY env)".to_string());
             set_show_sponsor_modal.set(false);
             return;
         }
        
        set_show_sponsor_modal.set(false);
        let k = keystore.get();
        if k.address.is_empty() { return; }
        
        spawn_local(async move {
            let feedback = crate::transactions::TxFeedback::new(set_status);
            feedback.set("Verifying PIN & Sponsoring...");
            
            // Faucet Key (Deployer - DEMO ONLY) - Loaded from Env
            let faucet_pk = option_env!("FAUCET_KEY").unwrap_or("");
            
            use ethers_core::types::{TransactionRequest, U256};
            use ethers_signers::{LocalWallet, Signer};
            
            let wallet: LocalWallet = faucet_pk.parse().unwrap();
            let wallet = wallet.with_chain_id(84532u64);
            
            let gas_price = crate::rpc::get_gas_price(Network::BaseSepolia).await.unwrap_or(0);
            let amount = U256::from(5000000000000000u64); // 0.005 ETH
            let gas_limit = 21000u64;
            let target_addr: ethers_core::types::Address = k.address.parse().unwrap();
            
            let tx = TransactionRequest::new()
                .to(target_addr)
                .value(amount)
                .gas(U256::from(gas_limit))
                .gas_price(U256::from(gas_price));
                
            let lat = crate::transactions::send_with_feedback(&wallet, tx, feedback, "Sponsored!").await;
            set_last_latency.set(lat);
            set_refresh_trigger.update(|v| *v += 1);
        });
    };

    let request_tba_sponsor = move |_| {
        let k = keystore.get();
        if let Some(tba) = k.smart_account {
             spawn_local(async move {
                let feedback = crate::transactions::TxFeedback::new(set_status);
                feedback.set("Sponsoring TBA...");
                
                let faucet_pk = option_env!("FAUCET_KEY").unwrap_or("");
                use ethers_core::types::{TransactionRequest, U256};
                use ethers_signers::{LocalWallet, Signer};
                
                let wallet: LocalWallet = faucet_pk.parse().unwrap();
                let wallet = wallet.with_chain_id(84532u64);
                
                let gas_price = crate::rpc::get_gas_price(Network::BaseSepolia).await.unwrap_or(0);
                let amount = U256::from(5000000000000000u64); // 0.005 ETH
                let gas_limit = 21000u64;
                let target_addr: ethers_core::types::Address = tba.parse().unwrap();
                
                let tx = TransactionRequest::new()
                    .to(target_addr)
                    .value(amount)
                    .gas(U256::from(gas_limit))
                    .gas_price(U256::from(gas_price));
                    
                let lat = crate::transactions::send_with_feedback(&wallet, tx, feedback, "TBA Sponsored!").await;
                set_last_latency.set(lat);
                set_refresh_trigger.update(|v| *v += 1);
             });
        }
    };
    
    let sweep_tba_funds = move |_| {
        let k = keystore.get();
        if k.private_key.is_empty() { return; }
        if let Some(tba) = k.smart_account {
             spawn_local(async move {
                let feedback = crate::transactions::TxFeedback::new(set_status);
                feedback.set("Sweeping TBA Funds...");
                
                // 1. Get TBA Balance
                let params = serde_json::json!([tba, "latest"]);
                let bal_res = crate::rpc::eth_call(Network::BaseSepolia, "eth_getBalance", params).await;
                let balance = match bal_res {
                    Ok(v) => {
                       let hex = v["result"].as_str().unwrap_or("0");
                       u128::from_str_radix(hex.trim_start_matches("0x"), 16).unwrap_or(0)
                    },
                    Err(_) => 0
                };
                
                if balance == 0 {
                    feedback.set("TBA has no funds.");
                    return;
                }
                
                let deployer_addr: ethers_core::types::Address = "0x769c18faa2e2e833a262c2ff9f6e1a9e99e52c58".parse().unwrap();
                let send_amount = balance; 
                
                use ethers_core::abi::{encode, Token};
                use ethers_core::types::U256;
                
                let inner_data = encode(&[
                    Token::Address(deployer_addr),
                    Token::Uint(U256::from(send_amount)),
                    Token::Bytes(vec![]), 
                    Token::Uint(U256::from(0u8)) 
                ]);
                
                let execute_selector = hex::decode("24857bd4").unwrap();
                let mut tx_data = execute_selector;
                tx_data.extend(inner_data);
                
                 use ethers_core::types::{TransactionRequest};
                 use ethers_signers::{LocalWallet, Signer};
                 
                 let pk = k.private_key.trim_start_matches("0x");
                 let wallet: LocalWallet = pk.parse().unwrap();
                 let wallet = wallet.with_chain_id(84532u64);
                 
                 let tba_addr: ethers_core::types::Address = tba.parse().unwrap();
                 
                 let gas_price = crate::rpc::get_gas_price(Network::BaseSepolia).await.unwrap_or(0);
                 let effective_gas_price = gas_price + (gas_price / 10);
                 let gas_limit = 200000u64; 

                 let tx = TransactionRequest::new()
                    .to(tba_addr)
                    .value(0) 
                    .data(tx_data)
                    .gas(U256::from(gas_limit))
                    .gas_price(U256::from(effective_gas_price));
                    
                 let lat = crate::transactions::send_with_feedback(&wallet, tx, feedback, "TBA Funds Swept!").await;
                 set_last_latency.set(lat);
                 set_refresh_trigger.update(|v| *v += 1);
             });
        }
    };

    // Backup Logic
    let download_backup = move |_| {
        let k = keystore.get();
        if k.private_key.is_empty() { return; }
        
        let json = serde_json::to_string_pretty(&k).unwrap();
        // Create Blob and trigger download using web-sys
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let anchor = document.create_element("a").unwrap();
        let anchor_html = anchor.dyn_ref::<web_sys::HtmlAnchorElement>().unwrap();
        
        let blob_parts = js_sys::Array::new();
        blob_parts.push(&json.into());
        
        let blob_props = web_sys::BlobPropertyBag::new();
        blob_props.set_type("application/json");
        
        let blob = web_sys::Blob::new_with_str_sequence_and_options(&blob_parts, &blob_props).unwrap();
        let url = web_sys::Url::create_object_url_with_blob(&blob).unwrap();
        
        anchor_html.set_href(&url);
        anchor_html.set_download("diamond-wallet-backup.json");
        anchor_html.click();
        
        web_sys::Url::revoke_object_url(&url).unwrap();
        set_status.set("Backup Downloaded".to_string());
    };

    let import_input_ref = create_node_ref::<leptos::html::Input>();
    
    let import_opt_click = move |_| {
         let input = import_input_ref.get().expect("input");
         input.click();
    };
    
    let on_file_change = move |ev: leptos::ev::Event| {
        use wasm_bindgen::JsCast;
        let input = ev.target().unwrap().unchecked_into::<web_sys::HtmlInputElement>();
        if let Some(files) = input.files() {
            if let Some(file) = files.get(0) {
                 let reader = web_sys::FileReader::new().unwrap();
                 let reader_c = reader.clone();
                 
                 let onload = wasm_bindgen::closure::Closure::wrap(Box::new(move |_e: web_sys::Event| {
                     if let Ok(res) = reader_c.result() {
                         if let Some(text) = res.as_string() {
                             match serde_json::from_str::<Keystore>(&text) {
                                 Ok(k) => {
                                     let _ = LocalStorage::set("diamond_wallet_keystore", &k);
                                     set_keystore.set(k);
                                     // Reset signals
                                     set_bal_sepolia.set("...".to_string());
 
                                 },
                                 Err(e) => logging::log!("Parse Error: {}", e),
                             }
                         }
                     }
                 }) as Box<dyn FnMut(_)>);
                 
                 reader.set_onload(Some(onload.as_ref().unchecked_ref()));
                 onload.forget(); // Leak memory for simplicity in this closure logic or handle properly
                 reader.read_as_text(&file).unwrap();
            }
        }
    };

    view! {
        <div class="wallet-container" style="position:relative">
            // Sticky Header
            <header class="app-header">
                <div class="header-status">
                    <span style="color:#888; font-size:10px; text-transform:uppercase; letter-spacing:1px;">"Status"</span>
                    <div style="color:#4CAF50; font-size:12px; font-weight:bold;">{move || status.get()}</div>
                </div>
                <div class="header-icons">
                     <button class="wallet-btn" on:click=move |_| set_show_brain_menu.set(true)>
                        <svg class="wallet-icon" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.5 2A2.5 2.5 0 0 1 12 4.5v15a2.5 2.5 0 0 1-4.96.44 2.5 2.5 0 0 1-2.96-3.08 3 3 0 0 1-.34-5.58 2.5 2.5 0 0 1 1.32-4.24 2.5 2.5 0 0 1 1.98-3A2.5 2.5 0 0 1 9.5 2Z"></path>
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14.5 2A2.5 2.5 0 0 0 12 4.5v15a2.5 2.5 0 0 0 4.96.44 2.5 2.5 0 0 0 2.96-3.08 3 3 0 0 0 .34-5.58 2.5 2.5 0 0 0-1.32-4.24 2.5 2.5 0 0 0-1.98-3A2.5 2.5 0 0 0 14.5 2Z"></path>
                        </svg>
                    </button>
                    <button class="wallet-btn" on:click=move |_| set_show_wallet_menu.set(true)>
                        <svg class="wallet-icon" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                            <rect x="2" y="3" width="20" height="14" rx="2" ry="2" stroke-width="2"></rect>
                            <line x1="8" y1="21" x2="16" y2="21" stroke-width="2"></line>
                            <line x1="12" y1="17" x2="12" y2="21" stroke-width="2"></line>
                        </svg>
                    </button>
                </div>
            </header>

            // Wallet Action Modal
            {move || if show_wallet_menu.get() {
                view! {
                    <div class="modal-overlay" on:click=move |_| set_show_wallet_menu.set(false)>
                        <div class="modal-content" on:click=move |ev| ev.stop_propagation()>
                            <h3 class="modal-title">"Device Wallet"</h3>
                            
                             {move || if !keystore.get().address.is_empty() {
                                view! {
                                    <div class="wallet-info" style="margin-bottom:15px; padding-bottom:15px; border-bottom:1px dashed #444;">
                                        <div class="flex-row" style="justify-content:center; align-items:center; gap:5px;">
                                            <p class="tiny-text" style="font-family:monospace; word-break:break-all; margin:0;">{keystore.get().address}</p>
                                            <button class="wallet-btn" style="width:20px; height:20px; padding:0; min-width:20px; border:none;" title="Copy" on:click=move |_| copy_to_clipboard(keystore.get().address)>
                                                <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" style="width:12px; height:12px;">
                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"></path>
                                                </svg>
                                            </button>
                                        </div>
                                        <div class="balance-grid">
                                            <div class="bal-item">
                                                <span class="label">"Sepolia"</span>
                                                <span class="val">{bal_sepolia.get()}</span>
                                            </div>
                                        </div>
                                    </div>
                                }.into_view()
                             } else {
                                view! { <p>"No Wallet Created"</p> }.into_view()
                             }}
                            
                            // Send ETH Button (Toggle)
                            {move || if !show_device_send.get() && !keystore.get().address.is_empty() {
                                view! { <button class="primary-btn" on:click=move |_| set_show_device_send.set(true)>"Send ETH"</button> }.into_view()
                            } else if show_device_send.get() {
                                view! {
                                    <div class="sponsor-box">
                                        <p>"Send ETH"</p>
                                        <input type="text" placeholder="Recipient (0x...)" 
                                            on:input=move |ev| set_device_recipient.set(event_target_value(&ev))
                                            prop:value=device_recipient
                                            style="margin-bottom:5px;" />
                                        <input type="text" placeholder="Amount (ETH)" 
                                            on:input=move |ev| set_device_amount.set(event_target_value(&ev)) 
                                            prop:value=device_amount
                                            style="margin-bottom:5px;" />
                                        <div class="flex-row">
                                            <button class="primary-btn" on:click=send_eth_device>"Send"</button>
                                            <button class="cancel-btn" on:click=move |_| set_show_device_send.set(false)>"Cancel"</button>
                                        </div>
                                    </div>
                                }.into_view()
                            } else {
                                view! { }.into_view()
                            }}

                            <hr style="border-color:#333; width:100%"/>
                            
                            // Sponsor
                            {move || if show_sponsor_modal.get() {
                                view! {
                                    <div class="sponsor-box">
                                        <p>"Enter PIN:"</p>
                                        <input type="password" 
                                            on:input=move |ev| set_pin_input.set(event_target_value(&ev))
                                            placeholder="1337"
                                            style="width: 100%; margin-bottom: 10px;"
                                        />
                                        <div class="flex-row">
                                            <button class="primary-btn" on:click=move |_| request_sponsor(())>"Confirm"</button>
                                            <button class="cancel-btn" on:click=move |_| set_show_sponsor_modal.set(false)>"Back"</button>
                                        </div>
                                    </div>
                                }.into_view()
                            } else {
                                view! {
                                    <button class="sponsor-btn" on:click=move |_| set_show_sponsor_modal.set(true)>"Request Sponsor (Testnet)"</button>
                                }.into_view()
                            }}

                            <button class="primary-btn" on:click=sweep_funds>"Sweep Signer -> Deployer"</button>
                            <button class="primary-btn" on:click=download_backup>"Download Backup"</button>
                            
                            <hr style="border-color:#333; width:100%"/>

                            {move || if show_clear_confirm.get() {
                                view! {
                                    <div class="warning-box">
                                        <p>"DANGEROUS: Wipe Key?"</p>
                                        <div class="flex-row">
                                            <button class="danger-btn" on:click=clear_wallet>"CONFIRM"</button>
                                            <button class="cancel-btn" on:click=move |_| set_show_clear_confirm.set(false)>"Cancel"</button>
                                        </div>
                                    </div>
                                }.into_view()
                            } else {
                                view! {
                                    <button class="danger-btn-outline" on:click=move |_| set_show_clear_confirm.set(true)>"Reset Signer"</button>
                                }.into_view()
                            }}
                            
                            <button class="cancel-btn" style="margin-top:10px" on:click=move |_| set_show_wallet_menu.set(false)>"Close Menu"</button>
                        </div>
                    </div>
                }.into_view()
            } else {
                view! { }.into_view()
            }}
            
                             // Brain Action Modal
            {move || if show_brain_menu.get() {
                view! {
                    <div class="modal-overlay" on:click=move |_| set_show_brain_menu.set(false)>
                        <div class="modal-content" on:click=move |ev| ev.stop_propagation()>
                            <h3 class="modal-title">"Smart Account"</h3>
                            
                             {move || if let Some(sa) = keystore.get().smart_account {
                                view! {
                                    <div class="smart-account-info" style="margin-bottom:15px; padding-bottom:15px; border-bottom:1px dashed #444;">
                                        <p class="success-text" style="text-align:center">"Active"</p>
                                        <div class="flex-row" style="justify-content:center; align-items:center; gap:5px;">
                                            <p style="font-size:10px; font-family:monospace; word-break:break-all; margin:0;">{sa.clone()}</p>
                                            <button class="wallet-btn" style="width:20px; height:20px; padding:0; min-width:20px; border:none;" title="Copy" on:click=move |_| copy_to_clipboard(sa.clone())>
                                                <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" style="width:12px; height:12px;">
                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"></path>
                                                </svg>
                                            </button>
                                        </div>
                                         <div class="balance-grid">
                                            <div class="bal-item">
                                                <span class="label">"Sepolia"</span>
                                                <span class="val">{tba_bal_sepolia.get()}</span>
                                            </div>
                                        </div>
                                    </div>
                                    
                                    // Send ETH (Smart Account)
                                     {move || if !show_sa_send.get() {
                                        view! { <button class="primary-btn" style="width:100%" on:click=move |_| set_show_sa_send.set(true)>"Send ETH"</button> }.into_view()
                                    } else {
                                        view! {
                                            <div class="sponsor-box">
                                                <p>"Send ETH (via TBA)"</p>
                                                <input type="text" placeholder="Recipient (0x...)" 
                                                    on:input=move |ev| set_sa_recipient.set(event_target_value(&ev))
                                                    prop:value=sa_recipient
                                                    style="margin-bottom:5px;" />
                                                <input type="text" placeholder="Amount (ETH)" 
                                                    on:input=move |ev| set_sa_amount.set(event_target_value(&ev)) 
                                                    prop:value=sa_amount
                                                    style="margin-bottom:5px;" />
                                                <div class="flex-row">
                                                    <button class="primary-btn" on:click=send_eth_sa>"Send"</button>
                                                    <button class="cancel-btn" on:click=move |_| set_show_sa_send.set(false)>"Cancel"</button>
                                                </div>
                                            </div>
                                        }.into_view()
                                    }}
                                    
                                    <div class="flex-col" style="gap:10px; margin-top:10px;">
                                        <button class="sponsor-btn" on:click=request_tba_sponsor>"Request Sponsor (TBA)"</button>
                                        <button class="primary-btn" on:click=sweep_tba_funds>"Sweep TBA -> Deployer"</button>
                                    </div>
                                }.into_view()
                             } else {
                                view! {
                                    <div>
                                        <p style="text-align:center; color:#888;">"Not Deployed"</p>
                                        <div class="flex-col" style="gap:10px; margin-top:10px;">
                                            <button class="primary-btn" on:click=mint_identity>"1. Mint Identity NFT"</button>
                                            <button class="text-btn" style="border:1px solid #333;" on:click=compute_tba>"2. Compute TBA Address"</button>
                                        </div>
                                    </div>
                                }.into_view()
                             }}
                            
                            <button class="cancel-btn" style="margin-top:10px" on:click=move |_| set_show_brain_menu.set(false)>"Close Menu"</button>
                        </div>
                    </div>
                }.into_view()
            } else {
                view! { }.into_view()
            }}

            // Scrollable Content
            <div class="app-content">
                // Home Screen (Empty for now, waiting for Apps)
                {move || if keystore.get().address.is_empty() {
                     view! { 
                        <div class="onboarding">
                            <button class="primary-btn" on:click=generate_wallet>"Create New Wallet"</button> 
                            <button class="text-btn" on:click=import_opt_click>"Import Backup JSON"</button>
                            <input type="file" node_ref=import_input_ref style="display:none" on:change=on_file_change accept=".json" />
                        </div>
                    }.into_view()
                } else {
                    view! {
                         <div class="home-apps" style="display:flex; flex-direction:column; align-items:center; justify-content:center; height:100%; color:#444;">
                            <span style="font-size:40px; margin-bottom:10px; opacity:0.2">"❖"</span>
                            <p>"Home Screen"</p>
                         </div>
                    }.into_view()
                }}
                
                {move || if let Some(ms) = last_latency.get() {
                     view! { <div class="latency-meter">{format!("Latency: {:.0}ms", ms)}</div> }.into_view()
                } else {
                     view! { }.into_view()
                }}
            </div> // End app-content

            // Sticky Footer
            <nav class="app-nav">
                <button class="nav-item active">
                    <svg style="width:24px;height:24px;" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z"></path></svg>
                </button>
                <button class="nav-item">
                    <svg style="width:24px;height:24px;" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6V4m0 2a2 2 0 100 4m0-4a2 2 0 110 4m-6 8a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4m6 6v10m6-2a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4"></path></svg>
                </button>
            </nav>
        </div>
    }
}
