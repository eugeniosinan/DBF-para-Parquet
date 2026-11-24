#!/usr/bin/env bash
set -e
echo "Compilando versão Linux (Release)..."
cargo build --release
mkdir -p dist
cp target/release/dbf-para-parquet dist/dbf-para-parquet-linux
chmod +x dist/dbf-para-parquet-linux
echo "✅ Binário Linux gerado em: dist/dbf-para-parquet-linux"
