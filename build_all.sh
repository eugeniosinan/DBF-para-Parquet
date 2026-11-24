#!/usr/bin/env bash
set -e
./build_linux.sh
./build_windows.sh
echo ""
echo "🎉 Executáveis Linux e Windows prontos em: dist/"
ls -lh dist/
