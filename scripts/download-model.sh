#!/usr/bin/env bash
# Download the all-MiniLM-L6-v2 ONNX model for WeftOS embedding inference.
#
# Usage:
#   scripts/download-model.sh              # Downloads to .weftos/models/
#   scripts/download-model.sh --global     # Downloads to ~/.weftos/models/
#
# The model is ~90MB and provides 384-dimensional sentence embeddings.

set -euo pipefail

MODEL_NAME="all-MiniLM-L6-v2"
MODEL_FILE="${MODEL_NAME}.onnx"
# HuggingFace ONNX model URL (sentence-transformers/all-MiniLM-L6-v2)
MODEL_URL="https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/onnx/model.onnx"

if [[ "${1:-}" == "--global" ]]; then
    MODEL_DIR="$HOME/.weftos/models"
else
    MODEL_DIR=".weftos/models"
fi

TARGET="${MODEL_DIR}/${MODEL_FILE}"

if [[ -f "$TARGET" ]]; then
    echo "Model already exists: $TARGET"
    echo "Size: $(du -h "$TARGET" | cut -f1)"
    exit 0
fi

echo "Downloading ${MODEL_NAME} ONNX model..."
echo "  Source: ${MODEL_URL}"
echo "  Target: ${TARGET}"

mkdir -p "$MODEL_DIR"

if command -v curl &>/dev/null; then
    curl -L --progress-bar -o "$TARGET" "$MODEL_URL"
elif command -v wget &>/dev/null; then
    wget --show-progress -O "$TARGET" "$MODEL_URL"
else
    echo "Error: neither curl nor wget found. Install one and retry."
    exit 1
fi

echo "Downloaded: $TARGET ($(du -h "$TARGET" | cut -f1))"
echo ""
echo "To enable ONNX embeddings, build with:"
echo "  scripts/build.sh native --features onnx-embeddings"
