#!/bin/bash

echo "Building WGPU examples..."

# WASM 타겟 추가
rustup target add wasm32-unknown-unknown

# wasm-pack 설치 (필요한 경우)
if ! command -v wasm-pack &> /dev/null; then
    curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
fi

# WASM 빌드
cd examples/wgpu-triangle
wasm-pack build --target web --out-dir ../../src/assets/wasm

echo "Building mdBook..."
cd ../..
mdbook build

# echo "Copying WASM files..."
# cp -r src/wgpu-examples/pkg book/wgpu-examples/

echo "Build complete!"
