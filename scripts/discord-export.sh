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

# Midnight Discord Channels
declare -A CHANNELS=(
    ["block-producers"]="1328720589548032032"
    ["dev-chat"]="1209887476290682910"
    ["pool-ids"]="1328721931343499345"
)

# Check for token
if [ -z "$DISCORD_TOKEN" ]; then
    echo "Error: DISCORD_TOKEN environment variable not set"
    echo ""
    echo "To get your token:"
    echo "1. Open Discord in browser (discord.com/app)"
    echo "2. Open DevTools (Cmd+Option+I)"
    echo "3. Go to Network tab"
    echo "4. Click any channel, find a request"
    echo "5. Look for 'authorization' header"
    echo ""
    echo "Then run: export DISCORD_TOKEN='your_token_here'"
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

# Export each channel
for channel_name in "${!CHANNELS[@]}"; do
    channel_id="${CHANNELS[$channel_name]}"
    output_file="$OUTPUT_DIR/${channel_name}_${DATE}.json"

    echo "Exporting #${channel_name} (${channel_id})..."

    DiscordChatExporter.Cli export \
        -t "$DISCORD_TOKEN" \
        -c "$channel_id" \
        -f Json \
        -o "$output_file" \
        --after "$(date -v-30d +%Y-%m-%d)" \
        2>/dev/null || echo "  Warning: Failed to export #${channel_name}"

    if [ -f "$output_file" ]; then
        size=$(du -h "$output_file" | cut -f1)
        echo "  Saved: $output_file ($size)"
    fi
done

echo ""
echo "Export complete!"
echo ""
echo "To sync to server:"
echo "  scp -r $OUTPUT_DIR midnight@server:~/midnight-validator-monitor/discord-context/"
