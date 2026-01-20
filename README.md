# Diamond Wallet Monorepo

This repository contains the full stack for the Diamond Wallet embedded smart account system.

## Structure

- **`/app`**: The Frontend Application (Rust/Leptos WASM).
- **`/contracts`**: The Smart Contracts (Solidity/Foundry).
- **`/tools`**: Helper CLIs and Scripts.

## Setup

### Prerequisites
- Rust (Cargo)
- Foundry (Forge/Cast)
- Node.js (for Vercel/Trunk tools)

### Environment Variables
You must set up `.env` files in the respective directories.
- **App**: `app/.env` (Requires `FAUCET_KEY`).
- **Contracts**: `contracts/.env` (Requires `PRIVATE_KEY` for deployment).

## Usage
### Run App
```bash
cd app
trunk serve --open
```

### Deploy Contracts
```bash
cd contracts
forge script script/Deploy.s.sol --broadcast --rpc-url <YOUR_RPC>
```
