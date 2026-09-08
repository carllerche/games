#!/usr/bin/env bash
# Build one or more games for the browser into dist/<game>/, ready to upload
# to any static host.
#
#   web/build.sh                 # every game
#   web/build.sh undercroft      # one game
#
# Needs `rustup target add wasm32-unknown-unknown` and `cargo install
# wasm-bindgen-cli` at the version of the wasm-bindgen crate in Cargo.lock.
set -euo pipefail
cd "$(dirname "$0")/.."

games=("$@")
if [ ${#games[@]} -eq 0 ]; then
  games=(flappy_bird flappy_bird_3d undercroft)
fi

want=$(grep -A1 '^name = "wasm-bindgen"$' Cargo.lock | sed -n 's/^version = "\(.*\)"/\1/p')
have=$(wasm-bindgen --version | awk '{print $2}')
if [ "$want" != "$have" ]; then
  echo "wasm-bindgen-cli $have is installed but Cargo.lock uses $want." >&2
  echo "Run: cargo install wasm-bindgen-cli --version $want" >&2
  exit 1
fi

packages=()
for game in "${games[@]}"; do packages+=(-p "$game"); done
cargo build --target wasm32-unknown-unknown --profile wasm-release "${packages[@]}"

for game in "${games[@]}"; do
  out="dist/$game"
  rm -rf "$out"
  mkdir -p "$out"
  wasm-bindgen --target web --no-typescript --out-dir "$out" --out-name "$game" \
    "target/wasm32-unknown-unknown/wasm-release/$game.wasm"
  if command -v wasm-opt >/dev/null; then
    wasm-opt -Oz -o "$out/${game}_bg.wasm" "$out/${game}_bg.wasm"
  fi
  # The window title, written either as `title: "..."` or `window("...")`.
  title=$(grep -ohE 'title: "[^"]+"|window\("[^"]+"\)' "$game/src/main.rs" | head -1 | sed 's/.*"\(.*\)".*/\1/')
  sed -e "s/{{ TITLE }}/${title:-$game}/" -e "s/{{ NAME }}/$game/" web/index.html > "$out/index.html"
  echo "$out: $(du -h "$out/${game}_bg.wasm" | cut -f1) wasm"
done
