#!/usr/bin/env bash
# Regenerate the landing-page cover images in web/covers/ from each game's
# screenshot mode. Uses `sips`, so macOS only.
set -euo pipefail
cd "$(dirname "$0")/.."
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

FLAPPY_SCREENSHOT_DIR="$tmp/2d" cargo run -q -p flappy_bird
FLAPPY_SCREENSHOT_DIR="$tmp/3d" cargo run -q -p flappy_bird_3d
UNDERCROFT_SCREENSHOT_DIR="$tmp/uc" cargo run -q -p undercroft

jpg() { sips -s format jpeg -s formatOptions 82 --resampleWidth "$2" "$1" --out "$3" >/dev/null; }
jpg "$tmp/2d/late.png"    720  web/covers/flappy_bird.jpg
jpg "$tmp/3d/playing.png" 720  web/covers/flappy_bird_3d.jpg
jpg "$tmp/uc/floor.png"   1280 web/covers/undercroft.jpg
ls -la web/covers
