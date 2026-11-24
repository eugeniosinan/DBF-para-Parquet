#!/usr/bin/env bash
set -e
echo "Compilando versão Windows (.exe) via Cross..."
CROSS_CONTAINER_ENGINE=podman cross build --target x86_64-pc-windows-gnu --release
mkdir -p dist
cp target/x86_64-pc-windows-gnu/release/dbf-para-parquet.exe dist/dbf-para-parquet-windows.exe
echo "✅ Binário Windows gerado em: dist/dbf-para-parquet-windows.exe"
