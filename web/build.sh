#!/usr/bin/env bash
# Build the site into dist/: the landing page at dist/index.html and each
# game under dist/<game>/. Upload the whole directory to any static host.
#
#   web/build.sh                 # every game
#   web/build.sh undercroft      # rebuild one game; the landing page is always refreshed
#
# Needs `rustup target add wasm32-unknown-unknown` and `cargo install
# wasm-bindgen-cli` at the version of the wasm-bindgen crate in Cargo.lock.
set -euo pipefail
cd "$(dirname "$0")/.."

games=("$@")
if [ ${#games[@]} -eq 0 ]; then
  games=(flappy_bird flappy_bird_3d undercroft cat_powncer)
fi

want=$(grep -A1 '^name = "wasm-bindgen"$' Cargo.lock | sed -n 's/^version = "\(.*\)"/\1/p')
have=$(wasm-bindgen --version | awk '{print $2}')
if [ "$want" != "$have" ]; then
  echo "wasm-bindgen-cli $have is installed but Cargo.lock uses $want." >&2
  echo "Run: cargo install wasm-bindgen-cli --version $want" >&2
  exit 1
fi

for game in "${games[@]}"; do
  # One game per cargo invocation: the games share their dependencies, but
  # linking three fat-LTO binaries at once needs more memory than a CI
  # runner has.
  cargo build --target wasm32-unknown-unknown --profile wasm-release -p "$game"
  out="dist/$game"
  rm -rf "$out"
  mkdir -p "$out"
  wasm-bindgen --target web --no-typescript --out-dir "$out" --out-name "$game" \
    "target/wasm32-unknown-unknown/wasm-release/$game.wasm"
  if command -v wasm-opt >/dev/null; then
    # The feature flags match what rustc enables for wasm32-unknown-unknown;
    # older Binaryen releases reject them unless told they are allowed.
    wasm-opt -Oz \
      --enable-bulk-memory --enable-mutable-globals --enable-multivalue \
      --enable-nontrapping-float-to-int --enable-reference-types --enable-sign-ext \
      -o "$out/${game}_bg.wasm" "$out/${game}_bg.wasm"
  fi
  # The window title, written either as `title: "..."` or `window("...")`.
  title=$(grep -ohE 'title: "[^"]+"|window\("[^"]+"\)' "$game/src/main.rs" | head -1 | sed 's/.*"\(.*\)".*/\1/')
  sed -e "s/{{ TITLE }}/${title:-$game}/" -e "s/{{ NAME }}/$game/" web/index.html > "$out/index.html"
  echo "$out: $(du -h "$out/${game}_bg.wasm" | cut -f1) wasm"
done

# The landing page and its cover images.
mkdir -p dist/covers
cp web/site.html dist/index.html
cp web/covers/*.jpg dist/covers/
echo "dist/index.html: landing page"
