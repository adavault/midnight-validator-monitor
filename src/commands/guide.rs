//! Troubleshooting guide command
//!
//! Provides built-in documentation for common validator issues.

use clap::{Args, Subcommand};

#[derive(Args, Debug)]
pub struct GuideArgs {
    #[command(subcommand)]
    pub topic: Option<GuideTopic>,
}

#[derive(Subcommand, Debug)]
pub enum GuideTopic {
    /// Troubleshooting for validators not producing blocks
    NotProducing,
    /// Troubleshooting for registration issues
    Registration,
    /// Troubleshooting for peer/network connectivity issues
    Peers,
    /// Troubleshooting for memory/resource issues
    Memory,
    /// Troubleshooting for key-related issues
    Keys,
    /// General setup and configuration guide
    Setup,
    /// List all available topics
    List,
}

pub async fn run(args: GuideArgs) -> anyhow::Result<()> {
    match args.topic {
        Some(GuideTopic::NotProducing) => print_not_producing_guide(),
        Some(GuideTopic::Registration) => print_registration_guide(),
        Some(GuideTopic::Peers) => print_peers_guide(),
        Some(GuideTopic::Memory) => print_memory_guide(),
        Some(GuideTopic::Keys) => print_keys_guide(),
        Some(GuideTopic::Setup) => print_setup_guide(),
        Some(GuideTopic::List) | None => print_topic_list(),
    }
    Ok(())
}

fn print_topic_list() {
    println!("Midnight Validator Monitor - Troubleshooting Guides");
    println!("====================================================");
    println!();
    println!("Available topics:");
    println!();
    println!("  mvm guide not-producing   Registered but not producing blocks");
    println!("  mvm guide registration    Registration verification issues");
    println!("  mvm guide peers           Network connectivity problems");
    println!("  mvm guide memory          Memory leaks and high usage");
    println!("  mvm guide keys            Session key issues");
    println!("  mvm guide setup           Initial validator setup");
    println!();
    println!("Use 'mvm guide <topic>' for detailed troubleshooting steps.");
}

fn print_not_producing_guide() {
    println!("Troubleshooting: Registered But Not Producing Blocks");
    println!("=====================================================");
    println!();
    println!("If your validator is registered but not producing blocks, check:");
    println!();
    println!("1. COMMITTEE SELECTION (Most Common)");
    println!("   --------------------------------");
    println!("   Being registered does NOT guarantee block production.");
    println!("   Committee selection is stake-weighted random each epoch.");
    println!();
    println!("   Check: mvm status --keystore /path/to/keystore");
    println!("   Look for: \"Committee: X Not elected\" vs \"Committee: X Elected\"");
    println!();
    println!("   If not elected:");
    println!("   - This is NORMAL, especially with lower stake");
    println!("   - Wait for next epoch (2h on preview, 10h on mainnet)");
    println!("   - Increase stake to improve selection probability");
    println!();
    println!("2. KEY VERIFICATION");
    println!("   -----------------");
    println!("   Ensure all three keys are loaded in the node's keystore.");
    println!();
    println!("   Check: mvm keys --keystore /path/to/keystore verify");
    println!();
    println!("   All three must show checkmarks:");
    println!("   - Sidechain (crch): X");
    println!("   - AURA: X");
    println!("   - GRANDPA: X");
    println!();
    println!("3. NODE SYNC STATUS");
    println!("   -----------------");
    println!("   Your node must be fully synced to produce blocks.");
    println!();
    println!("   Check: mvm status");
    println!("   Look for: \"Node: X Synced\" (not \"Syncing XX%\")");
    println!();
    println!("4. PEER CONNECTIVITY");
    println!("   ------------------");
    println!("   You need good network connectivity to receive and propagate blocks.");
    println!();
    println!("   Check: mvm view (press 5 for Peers view)");
    println!("   - Should have 10+ peers");
    println!("   - Should have both inbound and outbound connections");
    println!();
    println!("5. REGISTRATION VALIDITY");
    println!("   ----------------------");
    println!("   Check if registration shows 'isValid: true'");
    println!();
    println!("   Check: mvm status --keystore /path/to/keystore");
    println!("   Look for: \"Registration: X Registered (valid)\"");
    println!();
    println!("   If showing 'INVALID':");
    println!("   - Registration may be pending");
    println!("   - Stake may be insufficient");
    println!("   - Re-register with valid parameters");
}

fn print_registration_guide() {
    println!("Troubleshooting: Registration Issues");
    println!("=====================================");
    println!();
    println!("Registration status can be verified with:");
    println!("  mvm status --keystore /path/to/keystore");
    println!("  mvm keys --keystore /path/to/keystore verify");
    println!();
    println!("REGISTRATION STATUSES");
    println!("---------------------");
    println!();
    println!("1. 'Permissioned candidate'");
    println!("   - Midnight Foundation validator");
    println!("   - No stake required");
    println!("   - Automatically in candidate pool");
    println!();
    println!("2. 'Registered (valid)'");
    println!("   - Your registration is active");
    println!("   - Eligible for committee selection");
    println!("   - Selection is stake-weighted random");
    println!();
    println!("3. 'Registered but INVALID'");
    println!("   - Registration exists but not valid");
    println!("   - Common causes:");
    println!("     * Insufficient stake");
    println!("     * Registration still processing");
    println!("     * Keys mismatch");
    println!();
    println!("4. 'Not registered'");
    println!("   - No registration found for this key");
    println!("   - Need to submit registration transaction");
    println!();
    println!("MAINCHAIN VS SIDECHAIN EPOCH");
    println!("----------------------------");
    println!("Registration uses MAINCHAIN epoch (24h on preview, 5 days mainnet).");
    println!("Committee selection uses SIDECHAIN epoch (2h on preview, 10h mainnet).");
    println!();
    println!("When registering:");
    println!("- Use current mainchain epoch for registration");
    println!("- MVM shows both epochs in the dashboard");
}

fn print_peers_guide() {
    println!("Troubleshooting: Network Connectivity");
    println!("======================================");
    println!();
    println!("Check peer status with:");
    println!("  mvm view (press 5 for Peers view)");
    println!();
    println!("HEALTHY PEER COUNTS");
    println!("-------------------");
    println!("- Minimum: 5+ peers");
    println!("- Recommended: 15-30 peers");
    println!("- Maximum: Varies (node config)");
    println!();
    println!("PEER DIVERSITY");
    println!("--------------");
    println!("You need BOTH inbound and outbound peers:");
    println!();
    println!("- Outbound (X): You connecting to others");
    println!("- Inbound (X): Others connecting to you");
    println!();
    println!("NO INBOUND PEERS?");
    println!("-----------------");
    println!("1. Check firewall allows port 30333 (default P2P port)");
    println!("2. Configure port forwarding on router");
    println!("3. If using VPS, check security groups");
    println!();
    println!("Commands to check:");
    println!("  sudo ufw status                    # Ubuntu firewall");
    println!("  sudo iptables -L -n | grep 30333   # IPtables");
    println!();
    println!("NO OUTBOUND PEERS?");
    println!("------------------");
    println!("1. Check internet connectivity");
    println!("2. Verify bootnodes are configured");
    println!("3. Check DNS resolution works");
    println!();
    println!("PEERS BUT STILL ISOLATED?");
    println!("-------------------------");
    println!("If you have peers but blocks aren't propagating:");
    println!("1. Check if peers are synced (X in peer list)");
    println!("2. Restart node to reconnect to fresh peers");
    println!("3. Add additional bootnodes to config");
}

fn print_memory_guide() {
    println!("Troubleshooting: Memory Issues");
    println!("==============================");
    println!();
    println!("KNOWN ISSUE: Memory Leaks");
    println!("-------------------------");
    println!("Midnight nodes may exhibit memory growth over time.");
    println!("This is a known upstream issue being addressed.");
    println!();
    println!("MONITORING MEMORY");
    println!("-----------------");
    println!("MVM tracks memory trends in the TUI:");
    println!("  mvm view");
    println!();
    println!("Look for the System row:");
    println!("  System: Mem 8.2G/16.0GX  (X = trend: rising, stable, falling)");
    println!();
    println!("Memory warnings appear at:");
    println!("  - 85%: Warning displayed");
    println!("  - 90%: Critical warning");
    println!();
    println!("RECOMMENDED ACTIONS");
    println!("-------------------");
    println!();
    println!("1. Set up periodic restarts:");
    println!("   - Restart node every 24-48 hours during low activity");
    println!("   - Use systemd timer or cron job");
    println!();
    println!("2. Monitor with alerts:");
    println!("   - Configure MVM alerts in mvm.toml");
    println!("   - Set up external monitoring (Prometheus/Grafana)");
    println!();
    println!("3. Increase available memory:");
    println!("   - Recommended: 16GB+ RAM");
    println!("   - Add swap space as buffer");
    println!();
    println!("4. Use --db-cache flag:");
    println!("   - Reduce with: --db-cache 512");
    println!("   - Trade-off: slower sync, less memory");
    println!();
    println!("EMERGENCY RESTART");
    println!("-----------------");
    println!("If memory is critical:");
    println!("  sudo systemctl restart midnight-node");
    println!();
    println!("Or if running manually:");
    println!("  pkill -f midnight-node && /path/to/midnight-node [flags]");
}

fn print_keys_guide() {
    println!("Troubleshooting: Session Key Issues");
    println!("====================================");
    println!();
    println!("VERIFYING KEYS");
    println!("--------------");
    println!("Check if keys are loaded and registered:");
    println!("  mvm keys --keystore /path/to/keystore verify");
    println!();
    println!("Three keys are required:");
    println!("  - Sidechain (crch): Identifies your validator");
    println!("  - AURA: Block production authorization");
    println!("  - GRANDPA: Finality voting");
    println!();
    println!("KEY STATUS SYMBOLS");
    println!("------------------");
    println!("  X = Key loaded in node keystore");
    println!("  X = Key NOT loaded or verification failed");
    println!("  ? = Unable to verify (--rpc-methods=unsafe required)");
    println!();
    println!("KEYSTORE LOCATION");
    println!("-----------------");
    println!("Default locations:");
    println!("  - /opt/midnight/keystore/");
    println!("  - ~/.local/share/midnight/chains/*/keystore/");
    println!();
    println!("Keystore files are named: <key_type_hex><public_key_hex>");
    println!("  - 63726368... = sidechain (crch)");
    println!("  - 61757261... = aura");
    println!("  - 6772616e... = grandpa");
    println!();
    println!("KEY NOT FOUND?");
    println!("--------------");
    println!("1. Ensure keystore path is correct");
    println!("2. Check file permissions (node user must read)");
    println!("3. Verify key type prefixes match");
    println!();
    println!("UNABLE TO VERIFY?");
    println!("-----------------");
    println!("If showing '?' for all keys:");
    println!("  - Start node with: --rpc-methods=unsafe");
    println!("  - This enables author_hasKey RPC method");
    println!();
    println!("KEY MISMATCH?");
    println!("-------------");
    println!("If keys don't match registration:");
    println!("1. Re-export keys from your key generation");
    println!("2. Ensure same keys used for registration and node");
    println!("3. Re-register with correct keys if needed");
}

fn print_setup_guide() {
    println!("Validator Setup Guide");
    println!("=====================");
    println!();
    println!("PREREQUISITES");
    println!("-------------");
    println!("- Linux server (Ubuntu 22.04+ recommended)");
    println!("- 16GB+ RAM");
    println!("- 500GB+ SSD storage");
    println!("- Stable internet connection");
    println!("- Port 30333 accessible for P2P");
    println!();
    println!("STEP 1: Install Node");
    println!("--------------------");
    println!("Follow Midnight documentation for node installation.");
    println!();
    println!("STEP 2: Generate Keys");
    println!("---------------------");
    println!("Generate session keys (sidechain, aura, grandpa).");
    println!("Keep backup of keys securely!");
    println!();
    println!("STEP 3: Register Validator");
    println!("--------------------------");
    println!("Submit registration transaction with:");
    println!("- Your public keys");
    println!("- Stake amount (tADA)");
    println!("- Target mainchain epoch");
    println!();
    println!("STEP 4: Install MVM");
    println!("-------------------");
    println!("  sudo ./mvm install");
    println!();
    println!("This installs:");
    println!("  - MVM binary to /usr/local/bin/mvm");
    println!("  - systemd services for sync daemon");
    println!("  - Configuration at /opt/midnight/mvm/config/");
    println!();
    println!("STEP 5: Configure MVM");
    println!("---------------------");
    println!("Edit /opt/midnight/mvm/config/mvm.toml:");
    println!();
    println!("  [rpc]");
    println!("  url = \"http://localhost:9944\"");
    println!();
    println!("  [validator]");
    println!("  keystore = \"/path/to/keystore\"");
    println!();
    println!("  [database]");
    println!("  path = \"/opt/midnight/mvm/data/mvm.db\"");
    println!();
    println!("STEP 6: Start Monitoring");
    println!("------------------------");
    println!("Start the sync daemon:");
    println!("  sudo systemctl enable mvm-sync");
    println!("  sudo systemctl start mvm-sync");
    println!();
    println!("Launch TUI:");
    println!("  mvm view");
    println!();
    println!("STEP 7: Verify Registration");
    println!("---------------------------");
    println!("  mvm status --keystore /path/to/keystore");
    println!();
    println!("Look for:");
    println!("  - Registration: X Registered (valid)");
    println!("  - Keys: all loaded");
    println!("  - Node: synced");
}
