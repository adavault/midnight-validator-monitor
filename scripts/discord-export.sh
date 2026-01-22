#!/bin/bash
#
# Discord Export Script for Midnight Channels
# Run on MacBook with DiscordChatExporter installed
#
# Usage: ./discord-export.sh
# Requires: DISCORD_TOKEN environment variable
#

set -e

# Configuration
OUTPUT_DIR="${DISCORD_EXPORT_DIR:-$HOME/discord-exports/midnight}"
DATE=$(date +%Y-%m-%d)

# Check for token
if [ -z "$DISCORD_TOKEN" ]; then
    echo "Error: DISCORD_TOKEN environment variable not set"
    echo ""
    echo "Run: export DISCORD_TOKEN='your_token_here'"
    exit 1
fi

# Check for DiscordChatExporter
if ! command -v DiscordChatExporter.Cli &> /dev/null; then
    echo "Error: DiscordChatExporter.Cli not found"
    echo "Install with: brew install discordchatexporter"
    exit 1
fi

# Create output directory
mkdir -p "$OUTPUT_DIR"

echo "Exporting Midnight Discord channels..."
echo "Output: $OUTPUT_DIR"
echo ""

# Export function
export_channel() {
    local name=$1
    local id=$2
    local output_file="$OUTPUT_DIR/${name}_${DATE}.json"

    echo "Exporting #${name} (${id})..."

    if DiscordChatExporter.Cli export \
        -t "$DISCORD_TOKEN" \
        -c "$id" \
        -f Json \
        -o "$output_file" \
        2>/dev/null; then
        if [ -f "$output_file" ]; then
            size=$(du -h "$output_file" | cut -f1)
            echo "  Saved: $output_file ($size)"
        fi
    else
        echo "  Warning: Failed to export #${name}"
    fi
}

# Midnight Discord Channels
export_channel "block-producers" "1328720589548032032"
export_channel "dev-chat" "1209887476290682910"
export_channel "pool-ids" "1328721931343499345"

echo ""
echo "Export complete!"
echo ""
echo "To sync to server:"
echo "  scp -r $OUTPUT_DIR/* midnight@server:~/midnight-validator-monitor/discord-context/"
