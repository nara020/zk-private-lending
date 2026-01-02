#!/bin/bash
# Build WASM package for ZK Private Lending circuits
# Usage: ./scripts/build-wasm.sh

set -e

echo "🔧 Installing wasm-pack..."
cargo install wasm-pack --version 0.12.1

echo "📦 Building WASM package..."
wasm-pack build --target web --features wasm --release

echo "🎉 WASM build complete!"
echo "Output directory: ./pkg/"
echo ""
echo "Usage in JavaScript:"
echo "  import init, { generate_collateral_proof } from './pkg/zk_private_lending_circuits.js';"
echo "  await init();"
echo "  const proof = generate_collateral_proof(amount, salt, commitment);"
