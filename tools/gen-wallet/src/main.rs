use ethers::prelude::*;
use ethers::abi::{Abi, Tokenize};
use std::convert::TryFrom;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use anyhow::Result;

const RPC_URL: &str = "https://sepolia.base.org";
const PRIVATE_KEY: &str = "95d8e530156677e0837cd040764e1b8feb54e4a7845e9fbe8937fa0a58f1ea67";

// Adjust path relative to tools/gen-wallet
const OUT_DIR: &str = "../../contracts/out"; 

async fn deploy_contract(
    client: Arc<SignerMiddleware<Provider<Http>, LocalWallet>>,
    name: &str,
    abi_file: &str,
    bin_file: &str,
    constructor_args: impl Tokenize,
) -> Result<Address> {
    println!("Deploying {}...", name);
    
    let abi_path = Path::new(OUT_DIR).join(abi_file);
    let bin_path = Path::new(OUT_DIR).join(bin_file);

    let abi_json = fs::read_to_string(&abi_path)
        .map_err(|e| anyhow::anyhow!("Failed into read ABI {:?}: {}", abi_path, e))?;
    let abi: Abi = serde_json::from_str(&abi_json)?;
    
    let bytecode_hex = fs::read_to_string(&bin_path)
        .map_err(|e| anyhow::anyhow!("Failed into read BIN {:?}: {}", bin_path, e))?;
    let bytecode = Bytes::from(hex::decode(&bytecode_hex)?);

    let factory = ContractFactory::new(abi, bytecode, client.clone());
    
    // Check current gas price
    let provider = client.provider();
    let (max_fee, max_priority) = provider.estimate_eip1559_fees(None).await?;
    println!("Gas Prices - Max Fee: {:?}, Priority: {:?}", max_fee, max_priority);

    let mut deployer = factory.deploy(constructor_args)?;
    // deployer.tx.set_gas(500_000); // Let Sepolia estimate or set generous limit
    
    // Use conservative fees
    deployer.tx.set_gas_price(max_fee);
    
    let contract = deployer.send().await?;
    println!("{} deployed at: {:?}", name, contract.address());
    
    // Sleep to let RPC catch up with nonce
    std::thread::sleep(Duration::from_secs(2));
    
    Ok(contract.address())
}

#[tokio::main]
async fn main() -> Result<()> {
    let provider = Provider::<Http>::try_from(RPC_URL)?
        .interval(Duration::from_millis(100)); // Polling interval
    let wallet: LocalWallet = PRIVATE_KEY.parse()?;
    let chain_id = provider.get_chainid().await?;
    let wallet = wallet.with_chain_id(chain_id.as_u64());
    
    println!("Connected to chain ID: {}", chain_id);
    println!("Deployer Address: {:?}", wallet.address());

    let balance = provider.get_balance(wallet.address(), None).await?;
    println!("Balance: {} Wei", balance);

    let client = Arc::new(SignerMiddleware::new(provider, wallet));

    // 1. Deploy Facets
    let cut_addr = deploy_contract(
        client.clone(),
        "DiamondCutFacet",
        "facets_DiamondCutFacet_sol_DiamondCutFacet.abi",
        "facets_DiamondCutFacet_sol_DiamondCutFacet.bin",
        (),
    ).await?;

    let loupe_addr = deploy_contract(
        client.clone(),
        "DiamondLoupeFacet",
        "facets_DiamondLoupeFacet_sol_DiamondLoupeFacet.abi",
        "facets_DiamondLoupeFacet_sol_DiamondLoupeFacet.bin",
        (),
    ).await?;

    let ownership_addr = deploy_contract(
        client.clone(),
        "OwnershipFacet",
        "facets_OwnershipFacet_sol_OwnershipFacet.abi",
        "facets_OwnershipFacet_sol_OwnershipFacet.bin",
        (),
    ).await?;

    let wallet_facet_addr = deploy_contract(
        client.clone(),
        "WalletFacet",
        "facets_WalletFacet_sol_WalletFacet.abi",
        "facets_WalletFacet_sol_WalletFacet.bin",
        (),
    ).await?;

    // 2. Deploy Factory
    let factory_addr = deploy_contract(
        client.clone(),
        "WalletFactory",
        "WalletFactory_sol_WalletFactory.abi",
        "WalletFactory_sol_WalletFactory.bin",
        (cut_addr, loupe_addr, ownership_addr, wallet_facet_addr),
    ).await?;

    println!("\nDEPLOYMENT COMPLETE!");
    println!("----------------------------------------");
    println!("DiamondCutFacet: {:?}", cut_addr);
    println!("DiamondLoupeFacet: {:?}", loupe_addr);
    println!("OwnershipFacet: {:?}", ownership_addr);
    println!("WalletFacet: {:?}", wallet_facet_addr);
    println!("WalletFactory: {:?}", factory_addr);
    println!("----------------------------------------");

    Ok(())
}
