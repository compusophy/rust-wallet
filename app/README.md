# Diamond Facet Wallet

A mobile-first, smart-contract-powered Ethereum wallet built with Rust (Leptos) and running entirely in the browser (WASM).

## Features
- **Device Wallet**: Local browser keystore (Signer).
- **Smart Account**: ERC-6551 Token Bound Account (TBA) derived from an Identity NFT.
- **Diamond Facets**: Modular wallet architecture.
- **Mobile UI**: "Pixel Buffer" aesthetic with strict 9:16 layout.

## Tech Stack
- **Frontend**: Leptos (Rust)
- **Build Tool**: Trunk
- **Styling**: Vanilla CSS (Terminal/Dark Theme)
- **Chain**: Base Sepolia

## Setup
1. Install Rust & WASM target:
   ```bash
   rustup target add wasm32-unknown-unknown
   ```
2. Install Trunk:
   ```bash
   cargo install --locked trunk
   ```
3. Run locally:
   ```bash
   cd app
   trunk serve --open
   ```

## Security
- Private keys are stored in `localStorage` inside the browser.
- **Never** use this with real funds on Mainnet without a full audit.
- Demo faucet keys are loaded via `.env` (not committed).
