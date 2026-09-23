#!/bin/bash
set -e

ARCH=$(uname -m)
if [ "$ARCH" = "arm64" ]; then
  NIX_SYSTEM="aarch64-linux"
else
  NIX_SYSTEM="x86_64-linux"
fi

NIX_COMMON_FLAGS="--extra-experimental-features 'nix-command flakes'"

docker volume create nix-store 2>/dev/null || true

docker run --rm \
  -v "$(pwd):/workspace" \
  -v nix-store:/nix \
  -w /workspace \
  -e PROJECT_NAME="${PROJECT_NAME}" \
  nixos/nix \
  sh -c "
    git config --global --add safe.directory /workspace
    nix $NIX_COMMON_FLAGS build --impure --no-link .#packages.${NIX_SYSTEM}.default
    cat \$(nix $NIX_COMMON_FLAGS path-info --impure .#packages.${NIX_SYSTEM}.default)
  " \
  | docker load
