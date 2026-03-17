#!/bin/bash
# Downloads Kenney city building model packs (CC0 license)
# and organizes GLB files into the SlideCity asset structure.
#
# Packs:
#   - City Kit (Suburban)    - houses, small residential
#   - City Kit (Commercial)  - shops, offices, skyscrapers
#   - City Kit (Industrial)  - factories, warehouses
#
# All CC0 licensed: https://kenney.nl

set -e
cd "$(dirname "$0")/.."

ASSET_DIR="assets/models/buildings"
TMP_DIR="/tmp/kenney_city_kits"

mkdir -p "$TMP_DIR"
mkdir -p "$ASSET_DIR/residential"
mkdir -p "$ASSET_DIR/commercial"
mkdir -p "$ASSET_DIR/industrial"
mkdir -p "$ASSET_DIR/infrastructure"

echo "=== Downloading Kenney City Kits ==="

# City Kit (Suburban) - residential buildings
echo ""
echo "[1/3] City Kit (Suburban) - 40 models..."
SUBURBAN_URL="https://kenney.nl/media/pages/assets/city-kit-suburban/167f6dbc31-1745479373/kenney_city-kit-suburban_20.zip"
SUBURBAN_ZIP="$TMP_DIR/suburban.zip"
if [ ! -f "$SUBURBAN_ZIP" ]; then
    curl -L -o "$SUBURBAN_ZIP" "$SUBURBAN_URL"
fi
unzip -o -q "$SUBURBAN_ZIP" -d "$TMP_DIR/suburban"

# City Kit (Commercial) - commercial buildings
echo "[2/3] City Kit (Commercial) - 50 models..."
COMMERCIAL_URL="https://kenney.nl/media/pages/assets/city-kit-commercial/16eb35d771-1753115042/kenney_city-kit-commercial_2.1.zip"
COMMERCIAL_ZIP="$TMP_DIR/commercial.zip"
if [ ! -f "$COMMERCIAL_ZIP" ]; then
    curl -L -o "$COMMERCIAL_ZIP" "$COMMERCIAL_URL"
fi
unzip -o -q "$COMMERCIAL_ZIP" -d "$TMP_DIR/commercial"

# City Kit (Industrial) - industrial buildings
echo "[3/3] City Kit (Industrial) - 25 models..."
INDUSTRIAL_URL="https://kenney.nl/media/pages/assets/city-kit-industrial/1c9d714428-1750838303/kenney_city-kit-industrial_1.0.zip"
INDUSTRIAL_ZIP="$TMP_DIR/industrial.zip"
if [ ! -f "$INDUSTRIAL_ZIP" ]; then
    curl -L -o "$INDUSTRIAL_ZIP" "$INDUSTRIAL_URL"
fi
unzip -o -q "$INDUSTRIAL_ZIP" -d "$TMP_DIR/industrial"

echo ""
echo "=== Organizing GLB files ==="

# Copy GLB files to the right directories
# Kenney packs have GLB files in a Models/GLTF format/ subdirectory
find "$TMP_DIR/suburban" -name "*.glb" -exec cp {} "$ASSET_DIR/residential/" \;
find "$TMP_DIR/commercial" -name "*.glb" -exec cp {} "$ASSET_DIR/commercial/" \;
find "$TMP_DIR/industrial" -name "*.glb" -exec cp {} "$ASSET_DIR/industrial/" \;

echo ""
echo "Residential models: $(ls "$ASSET_DIR/residential/"*.glb 2>/dev/null | wc -l)"
echo "Commercial models:  $(ls "$ASSET_DIR/commercial/"*.glb 2>/dev/null | wc -l)"
echo "Industrial models:  $(ls "$ASSET_DIR/industrial/"*.glb 2>/dev/null | wc -l)"
echo ""
echo "=== Done! Models saved to $ASSET_DIR ==="
echo ""
echo "Cleaning up temp files..."
rm -rf "$TMP_DIR"
echo "Complete."
