#!/usr/bin/env bash
# Regenerate the landing-page cover images in web/covers/ from each game's
# screenshot mode. Uses `sips`, so macOS only.
set -euo pipefail
cd "$(dirname "$0")/.."
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

FLAPPY_SCREENSHOT_DIR="$tmp/fb" cargo run -q -p flappy_bird
UNDERCROFT_SCREENSHOT_DIR="$tmp/uc" cargo run -q -p undercroft
cargo run -q -p cat_powncer -- --level 6 --screenshot "$tmp/cat.png" --after 2.5

jpg() { sips -s format jpeg -s formatOptions 82 --resampleWidth "$2" "$1" --out "$3" >/dev/null; }
jpg "$tmp/fb/playing.png" 720  web/covers/flappy_bird.jpg
jpg "$tmp/uc/floor.png"   1280 web/covers/undercroft.jpg
jpg "$tmp/cat.png"        1280 web/covers/cat_powncer.jpg
ls -la web/covers
