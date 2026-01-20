fn main() {
    // Load .env file if it exists
    dotenv::dotenv().ok();

    println!("cargo:rerun-if-changed=.env");

    if let Ok(val) = std::env::var("FAUCET_KEY") {
        println!("cargo:rustc-env=FAUCET_KEY={}", val);
    } else {
         println!("cargo:warning=FAUCET_KEY not found in .env or environment");
    }
}
