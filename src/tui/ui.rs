//! UI rendering for TUI

use crate::tui::layout::ResponsiveLayout;
use crate::tui::{App, ScreenSize, ViewMode};
use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

/// Convert a slice of values to Unicode sparkline bars
/// Uses block characters: ▁▂▃▄▅▆▇█ (8 levels)
fn sparkline_bars(values: &[u64]) -> String {
    const BARS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

    if values.is_empty() {
        return String::new();
    }

    let max = *values.iter().max().unwrap_or(&1);
    if max == 0 {
        // All zeros - show flat line
        return values.iter().map(|_| BARS[0]).collect();
    }

    values
        .iter()
        .map(|&v| {
            let idx = if v == 0 {
                0
            } else {
                // Scale to 0-7 range, with max value getting index 7
                ((v as f64 / max as f64) * 7.0).round() as usize
            };
            BARS[idx.min(7)]
        })
        .collect()
}

/// Render the UI with responsive layout
pub fn render(f: &mut Frame, app: &App) {
    let layout = ResponsiveLayout::new(f.size());
    let chunks = layout.main_layout(f.size());

    // Render title bar (compact for small screens)
    render_title_bar(f, app, chunks[0], &layout);

    // Render main content based on view mode
    match app.view_mode {
        ViewMode::Dashboard => render_dashboard(f, app, chunks[1], &layout),
        ViewMode::Blocks => render_blocks(f, app, chunks[1], &layout),
        ViewMode::Validators => render_validators(f, app, chunks[1], &layout),
        ViewMode::Performance => render_performance(f, app, chunks[1], &layout),
        ViewMode::Peers => render_peers(f, app, chunks[1], &layout),
        ViewMode::Help => render_help(f, app, chunks[1]),
    }

    // Render status bar (compact for small screens)
    render_status_bar(f, app, chunks[2], &layout);
}

fn render_title_bar(f: &mut Frame, app: &App, area: Rect, _layout: &ResponsiveLayout) {
    use ratatui::layout::{Constraint, Direction, Layout};

    let theme = app.theme;

    // Create inner area for the title bar content
    let inner_area = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border()))
        .inner(area);

    // Render the border
    let border_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border()));
    f.render_widget(border_block, area);

    // Split into left (title + view) and right (chain @ hostname)
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(40)])
        .split(inner_area);

    // Left side: title and current view
    let left_text = vec![
        Span::styled("Midnight Validator Monitor", Style::default().fg(theme.title()).add_modifier(Modifier::BOLD)),
        Span::styled(format!(" v{}", env!("CARGO_PKG_VERSION")), Style::default().fg(theme.muted())),
        Span::raw("  |  "),
        Span::styled(
            match app.view_mode {
                ViewMode::Dashboard => "[1] Dashboard",
                ViewMode::Blocks => "[2] Blocks",
                ViewMode::Validators => "[3] Validators",
                ViewMode::Performance => "[4] Performance",
                ViewMode::Peers => "[5] Peers",
                ViewMode::Help => "[?] Help",
            },
            Style::default().fg(theme.primary()).add_modifier(Modifier::BOLD),
        ),
    ];

    let left_paragraph = Paragraph::new(Line::from(left_text))
        .alignment(Alignment::Left);
    f.render_widget(left_paragraph, chunks[0]);

    // Right side: chain name and hostname (bold)
    let right_text = if app.state.chain_name.is_empty() {
        Line::from(vec![
            Span::styled(&app.state.node_name, Style::default().fg(theme.secondary()).add_modifier(Modifier::BOLD)),
            Span::raw(" "),
        ])
    } else {
        Line::from(vec![
            Span::styled(&app.state.chain_name, Style::default().fg(theme.epoch())),
            Span::styled(" @ ", Style::default().fg(theme.muted())),
            Span::styled(&app.state.node_name, Style::default().fg(theme.secondary()).add_modifier(Modifier::BOLD)),
            Span::raw(" "),
        ])
    };
    let right_paragraph = Paragraph::new(right_text)
        .alignment(Alignment::Right);
    f.render_widget(right_paragraph, chunks[1]);
}

fn render_status_bar(f: &mut Frame, app: &App, area: Rect, layout: &ResponsiveLayout) {
    use ratatui::layout::{Constraint, Direction, Layout};

    let theme = app.theme;
    let since_update = app.last_update.elapsed().as_secs();

    // Standard status bar: left-aligned status with right-aligned theme name
    let inner_area = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border()))
        .inner(area);

    // Render the border block
    let border_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border()));
    f.render_widget(border_block, area);

    // Split inner area: left for status, right for theme name (narrower for small screens)
    let theme_width = match layout.size {
        ScreenSize::Medium => 10,  // Just "Midnight" or "Midday"
        ScreenSize::Large => 18,   // Full theme name with padding
    };
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(theme_width)])
        .split(inner_area);

    // Left side: status info - compact for narrow screens
    let left_text = if app.state.is_loading {
        vec![
            Span::styled("◌ ", Style::default().fg(theme.warning())),
            Span::styled("Loading...", Style::default().fg(theme.text())),
        ]
    } else if let Some(ref err) = app.state.last_error {
        vec![
            Span::styled("ERR: ", Style::default().fg(theme.error()).add_modifier(Modifier::BOLD)),
            Span::styled(err.clone(), Style::default().fg(theme.error())),
        ]
    } else {
        match layout.size {
            ScreenSize::Medium => {
                // Compact status for narrow screens (fits in ~65 chars)
                vec![
                    Span::styled("●", Style::default().fg(theme.success())),
                    Span::styled(format!(" {}s ago | ", since_update), Style::default().fg(theme.text())),
                    Span::styled("Q", Style::default().fg(theme.primary())),
                    Span::styled("uit ", Style::default().fg(theme.muted())),
                    Span::styled("1-5", Style::default().fg(theme.primary())),
                    Span::styled(" Views ", Style::default().fg(theme.muted())),
                    Span::styled("T", Style::default().fg(theme.primary())),
                    Span::styled("heme ", Style::default().fg(theme.muted())),
                    Span::styled("?", Style::default().fg(theme.primary())),
                    Span::styled(" Help", Style::default().fg(theme.muted())),
                ]
            }
            ScreenSize::Large => {
                // Full status for wide screens
                vec![
                    Span::styled("●", Style::default().fg(theme.success())),
                    Span::styled(format!(" Connected  |  Updated {}s ago  |  ", since_update), Style::default().fg(theme.text())),
                    Span::styled("[Q]", Style::default().fg(theme.primary())),
                    Span::styled(" Quit  ", Style::default().fg(theme.muted())),
                    Span::styled("[1-5]", Style::default().fg(theme.primary())),
                    Span::styled(" Views  ", Style::default().fg(theme.muted())),
                    Span::styled("[T]", Style::default().fg(theme.primary())),
                    Span::styled(" Theme  ", Style::default().fg(theme.muted())),
                    Span::styled("[?]", Style::default().fg(theme.primary())),
                    Span::styled(" Help", Style::default().fg(theme.muted())),
                ]
            }
        }
    };

    let left_paragraph = Paragraph::new(Line::from(left_text))
        .alignment(Alignment::Left);
    f.render_widget(left_paragraph, chunks[0]);

    // Right side: theme name (shorter for narrow screens)
    let theme_display = match layout.size {
        ScreenSize::Medium => if theme.name().contains("Midnight") { "Night" } else { "Day" },
        ScreenSize::Large => theme.name(),
    };
    let right_text = Line::from(vec![
        Span::styled(theme_display, Style::default().fg(theme.secondary())),
        Span::raw(" "),
    ]);
    let right_paragraph = Paragraph::new(right_text)
        .alignment(Alignment::Right);
    f.render_widget(right_paragraph, chunks[1]);
}

fn render_dashboard(f: &mut Frame, app: &App, area: Rect, layout: &ResponsiveLayout) {
    let theme = app.theme;
    let key_mode = layout.key_display_length();
    let chunks = layout.dashboard_layout(area);

    // Show loading state if still loading initial data
    if app.state.is_loading {
        let loading_text = vec![
            Line::from(vec![
                Span::styled("◌ ", Style::default().fg(theme.warning())),
                Span::styled("Connecting to node and loading data...", Style::default().fg(theme.text())),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  This may take a few seconds on first startup.", Style::default().fg(theme.muted())),
            ]),
        ];
        let loading_widget = Paragraph::new(loading_text)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border()))
                .title(Span::styled("Network Status", Style::default().fg(theme.primary()).add_modifier(Modifier::BOLD))));
        f.render_widget(loading_widget, chunks[0]);

        // Empty placeholders for other panels
        let placeholder = Paragraph::new("")
            .block(Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border()))
                .title(Span::styled("Our Validator", Style::default().fg(theme.ours()).add_modifier(Modifier::BOLD))));
        f.render_widget(placeholder, chunks[1]);

        let placeholder2 = Paragraph::new("")
            .block(Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border()))
                .title(Span::styled("Recent Blocks", Style::default().fg(theme.primary()).add_modifier(Modifier::BOLD))));
        f.render_widget(placeholder2, chunks[2]);
        return;
    }

    // Create epoch progress bar (fixed width, no truncation)
    let epoch_progress = &app.state.epoch_progress;
    let progress_width = 20;
    let filled = ((epoch_progress.progress_percent / 100.0) * progress_width as f64) as usize;
    let progress_bar: String = format!(
        "{}{}",
        "━".repeat(filled.min(progress_width)),
        "░".repeat(progress_width.saturating_sub(filled))
    );

    // Build sync progress bar
    let sync = &app.state.sync_progress;
    let sync_bar_width = match layout.size {
        ScreenSize::Medium => 20,
        ScreenSize::Large => 25,
    };
    let sync_filled = ((sync.sync_percent / 100.0) * sync_bar_width as f64) as usize;
    let sync_bar: String = format!(
        "{}{}",
        "━".repeat(sync_filled.min(sync_bar_width)),
        "░".repeat(sync_bar_width.saturating_sub(sync_filled))
    );
    let (sync_icon, sync_color) = if sync.is_synced {
        ("✓", theme.success())
    } else {
        ("⟳", theme.warning())
    };

    // Standard network status with enhanced info (same for Medium and Large)
    // Line 1: Block number first (most important), then peers
    let mut network_text = vec![
        Line::from(vec![
            Span::styled("Block: ", Style::default().fg(theme.muted())),
            Span::styled(format!("#{}", app.state.chain_tip), Style::default().fg(theme.block_number())),
            Span::styled(
                if app.state.chain_tip == app.state.finalized_block { " (finalized)" } else { "" },
                Style::default().fg(theme.success())
            ),
            Span::raw("      "),
            Span::styled("Peers: ", Style::default().fg(theme.muted())),
            Span::styled(format!("{}", app.state.peer_count), Style::default().fg(theme.text())),
            Span::styled(format!(" (↑{} ↓{})", app.state.peers_outbound, app.state.peers_inbound), Style::default().fg(theme.muted())),
        ]),
    ];

    // Calculate MVM sync percentage (highest synced block vs chain tip)
    // Use the most recent block number from our DB, not the count
    let mvm_last_block = app.state.recent_blocks.first()
        .map(|b| b.block_number)
        .unwrap_or(0);
    let mvm_sync_pct = if app.state.chain_tip > 0 && mvm_last_block > 0 {
        (mvm_last_block as f64 / app.state.chain_tip as f64) * 100.0
    } else if app.state.total_blocks > 0 {
        100.0 // Have blocks but can't calculate - assume synced
    } else {
        0.0
    };
    let mvm_synced = mvm_sync_pct >= 99.9;
    let mvm_status = if mvm_synced {
        format!("{} blocks (100%)", app.state.total_blocks)
    } else {
        format!("{} blocks ({:.1}%)", app.state.total_blocks, mvm_sync_pct)
    };
    let mvm_color = if mvm_synced { theme.text() } else { theme.warning() };

    // Node sync status line
    if sync.is_synced {
        network_text.push(Line::from(vec![
            Span::styled("Node: ", Style::default().fg(theme.muted())),
            Span::styled(format!("{} Synced", sync_icon), Style::default().fg(sync_color)),
            Span::raw("       "),
            Span::styled("MVM: ", Style::default().fg(theme.muted())),
            Span::styled(mvm_status, Style::default().fg(mvm_color)),
        ]));
    } else {
        network_text.push(Line::from(vec![
            Span::styled("Node:          ", Style::default().fg(theme.muted())),
            Span::styled(sync_bar.clone(), Style::default().fg(theme.warning())),
            Span::styled(format!(" {:.1}%", sync.sync_percent), Style::default().fg(theme.text())),
            Span::styled(format!("  ({} behind)", sync.blocks_remaining), Style::default().fg(theme.muted())),
            Span::raw("     "),
            Span::styled("MVM: ", Style::default().fg(theme.muted())),
            Span::styled(mvm_status, Style::default().fg(mvm_color)),
        ]));
    }

    // Build mainchain progress bar
    let mainchain_filled = ((epoch_progress.mainchain_progress_percent / 100.0) * progress_width as f64) as usize;
    let mainchain_progress_bar: String = format!(
        "{}{}",
        "━".repeat(mainchain_filled.min(progress_width)),
        "░".repeat(progress_width.saturating_sub(mainchain_filled))
    );

    network_text.push(Line::from(vec![
        Span::styled("Sidechain:  ", Style::default().fg(theme.muted())),
        Span::styled(format!("epoch {:>5}", app.state.sidechain_epoch), Style::default().fg(theme.epoch())),
        Span::raw("  "),
        Span::styled(progress_bar.clone(), Style::default().fg(theme.primary())),
        Span::styled(format!(" {:.1}%", epoch_progress.progress_percent), Style::default().fg(theme.text())),
    ]));
    network_text.push(Line::from(vec![
        Span::styled("Mainchain:   ", Style::default().fg(theme.muted())),
        Span::styled(format!("epoch {:>5}", app.state.mainchain_epoch), Style::default().fg(theme.epoch())),
        Span::raw("  "),
        Span::styled(mainchain_progress_bar, Style::default().fg(theme.primary())),
        Span::styled(format!(" {:.1}%", epoch_progress.mainchain_progress_percent), Style::default().fg(theme.text())),
    ]));

    // Node metrics line - bandwidth, uptime, grandpa status
    let bandwidth_in = format_bytes(app.state.bandwidth_in);
    let bandwidth_out = format_bytes(app.state.bandwidth_out);
    let uptime = format_uptime(app.state.uptime_secs);
    let grandpa_icon = if app.state.grandpa_voter { "✓" } else { "○" };
    let grandpa_color = if app.state.grandpa_voter { theme.success() } else { theme.muted() };

    network_text.push(Line::from(vec![
        Span::styled("Bandwidth: ", Style::default().fg(theme.muted())),
        Span::styled(format!("↓{} ↑{}", bandwidth_in, bandwidth_out), Style::default().fg(theme.text())),
        Span::raw("    "),
        Span::styled("Uptime: ", Style::default().fg(theme.muted())),
        Span::styled(uptime, Style::default().fg(theme.text())),
        Span::raw("    "),
        Span::styled("Grandpa: ", Style::default().fg(theme.muted())),
        Span::styled(grandpa_icon, Style::default().fg(grandpa_color)),
    ]));

    // Transaction pool line
    network_text.push(Line::from(vec![
        Span::styled("Tx Pool: ", Style::default().fg(theme.muted())),
        Span::styled(format!("{} ready", app.state.txpool_ready), Style::default().fg(theme.text())),
        Span::raw("    "),
        Span::styled("Validations: ", Style::default().fg(theme.muted())),
        Span::styled(format!("{}", app.state.txpool_validations), Style::default().fg(theme.text())),
    ]));

    // Network identity line (external IP and peer ID)
    let external_ip = if app.state.external_ips.is_empty() {
        "unknown".to_string()
    } else {
        app.state.external_ips.join(", ")
    };
    let peer_id_display = if app.state.local_peer_id.len() > 20 {
        format!("{}...{}", &app.state.local_peer_id[..8], &app.state.local_peer_id[app.state.local_peer_id.len()-6..])
    } else if app.state.local_peer_id.is_empty() {
        "unknown".to_string()
    } else {
        app.state.local_peer_id.clone()
    };
    network_text.push(Line::from(vec![
        Span::styled("External IP: ", Style::default().fg(theme.muted())),
        Span::styled(external_ip, Style::default().fg(theme.text())),
        Span::raw("    "),
        Span::styled("Peer ID: ", Style::default().fg(theme.muted())),
        Span::styled(peer_id_display, Style::default().fg(theme.secondary())),
    ]));

    let network_widget = Paragraph::new(network_text)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border()))
            .title(Span::styled("Network Status", Style::default().fg(theme.primary()).add_modifier(Modifier::BOLD))));
    f.render_widget(network_widget, chunks[0]);

    // Our validators - responsive with epoch block predictions
    let our_validators_text = if app.state.our_validators_count > 0 {
        let total_our_blocks: u64 = app.state.our_validators.iter().map(|v| v.total_blocks).sum();
        let share = if app.state.total_blocks > 0 {
            (total_our_blocks as f64 / app.state.total_blocks as f64) * 100.0
        } else {
            0.0
        };

        // Epoch block tracking
        let epoch_blocks = epoch_progress.our_blocks_this_epoch;
        let expected_blocks = epoch_progress.expected_blocks;
        let performance_indicator = if expected_blocks > 0.0 {
            let ratio = epoch_blocks as f64 / expected_blocks;
            if ratio >= 0.9 { "✓" } else if ratio >= 0.5 { "○" } else { "!" }
        } else {
            "?"
        };

        // Panel height (9) fits 1 validator with all 3 keys (4 header + 3 key lines + 2 border)
        let max_validators = 1;

        // Committee election status
        let (committee_icon, committee_color) = if app.state.committee_elected {
            ("✓", theme.success())
        } else {
            ("✗", theme.warning())
        };

        // Committee status text (same for Medium and Large)
        let committee_status = if app.state.committee_elected {
            if app.state.committee_seats > 1 {
                format!("{} Elected ({} seats in {} member committee)", committee_icon, app.state.committee_seats, app.state.committee_size)
            } else {
                format!("{} Elected ({} seat in {} member committee)", committee_icon, app.state.committee_seats, app.state.committee_size)
            }
        } else if app.state.committee_size > 0 {
            format!("{} Not elected this epoch", committee_icon)
        } else {
            "? Checking committee...".to_string()
        };

        // Format node version (trim git hash for display)
        let version_display = if app.state.node_version.contains('-') {
            app.state.node_version.split('-').next().unwrap_or(&app.state.node_version)
        } else if app.state.node_version.is_empty() {
            "?"
        } else {
            &app.state.node_version
        };

        let mut lines = vec![
            Line::from(vec![
                Span::styled("Node: ", Style::default().fg(theme.muted())),
                Span::styled(format!("v{}", version_display), Style::default().fg(theme.text())),
                Span::raw("    "),
                Span::styled("Committee: ", Style::default().fg(theme.muted())),
                Span::styled(committee_status, Style::default().fg(committee_color)),
            ]),
            Line::from(vec![
                Span::styled("All-Time Blocks: ", Style::default().fg(theme.muted())),
                Span::styled(format!("{}", total_our_blocks), Style::default().fg(theme.success())),
                Span::raw("      "),
                Span::styled("Share: ", Style::default().fg(theme.muted())),
                Span::styled(format!("{:.3}%", share), Style::default().fg(theme.success())),
            ]),
            Line::from(vec![
                Span::styled("This Epoch: ", Style::default().fg(theme.muted())),
                Span::styled(format!("{}", epoch_blocks), Style::default().fg(theme.primary())),
                Span::styled(format!(" blocks  (expected: ~{:.1})", expected_blocks), Style::default().fg(theme.muted())),
                Span::raw("  "),
                Span::styled(performance_indicator, Style::default().fg(
                    if performance_indicator == "✓" { theme.success() }
                    else if performance_indicator == "!" { theme.warning() }
                    else { theme.muted() }
                )),
            ]),
            // Sparkline showing block production over last 12 sidechain epochs (24h)
            Line::from(vec![
                Span::styled("Last 24h:   ", Style::default().fg(theme.muted())),
                Span::styled(sparkline_bars(&app.state.our_blocks_sparkline), Style::default().fg(theme.primary())),
                Span::styled(
                    format!("  (total: {})", app.state.our_blocks_sparkline.iter().sum::<u64>()),
                    Style::default().fg(theme.muted())
                ),
            ]),
        ];

        // Show validators with all three public keys (same for Medium and Large)
        for v in app.state.our_validators.iter().take(max_validators) {
            let sidechain_display = key_mode.format(&v.sidechain_key);
            let label = v.label.as_ref().map(|l| format!(" ({})", l)).unwrap_or_default();

            lines.push(Line::from(vec![
                Span::styled("  ★ Sidechain: ", Style::default().fg(theme.ours())),
                Span::styled(sidechain_display.clone(), Style::default().fg(theme.secondary())),
                Span::styled(label.clone(), Style::default().fg(theme.muted())),
            ]));

            // Show AURA key if available
            if let Some(ref aura_key) = v.aura_key {
                let aura_display = key_mode.format(aura_key);
                lines.push(Line::from(vec![
                    Span::styled("    AURA:      ", Style::default().fg(theme.muted())),
                    Span::styled(aura_display, Style::default().fg(theme.text())),
                ]));
            }

            // Show Grandpa key if available
            if let Some(ref grandpa_key) = v.grandpa_key {
                let grandpa_display = key_mode.format(grandpa_key);
                lines.push(Line::from(vec![
                    Span::styled("    Grandpa:   ", Style::default().fg(theme.muted())),
                    Span::styled(grandpa_display, Style::default().fg(theme.text())),
                ]));
            }
        }

        if app.state.our_validators_count > max_validators as u64 {
            lines.push(Line::from(vec![
                Span::styled(format!("  +{} more", app.state.our_validators_count - max_validators as u64), Style::default().fg(theme.muted())),
            ]));
        }

        lines
    } else {
        // No validators message (same for Medium and Large)
        vec![
            Line::from(vec![
                Span::styled("No validators marked as 'ours'", Style::default().fg(theme.warning())),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Run: ", Style::default().fg(theme.muted())),
                Span::styled("mvm keys --keystore <path> verify", Style::default().fg(theme.secondary())),
            ]),
        ]
    };

    let our_validators_widget = Paragraph::new(our_validators_text)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border()))
            .title(Span::styled("Our Validator", Style::default().fg(theme.ours()).add_modifier(Modifier::BOLD))));
    f.render_widget(our_validators_widget, chunks[1]);

    // Recent blocks - fill available space (panel height minus 2 for borders)
    let blocks_to_show = chunks[2].height.saturating_sub(2) as usize;

    let blocks_items: Vec<ListItem> = app.state.recent_blocks.iter().take(blocks_to_show).map(|block| {
        let author_display = if let Some(ref author) = block.author_key {
            key_mode.format(author)
        } else {
            "unknown".to_string()
        };

        let finalized = if block.is_finalized { "✓" } else { " " };

        // Standard block format (same for Medium and Large)
        let line = Line::from(vec![
            Span::styled(format!("#{:<8}", block.block_number), Style::default().fg(theme.block_number())),
            Span::raw("  "),
            Span::styled(format!("slot {:>12}", block.slot_number), Style::default().fg(theme.muted())),
            Span::raw("  "),
            Span::styled(format!("epoch {:>6}", block.sidechain_epoch), Style::default().fg(theme.epoch())),
            Span::raw("  "),
            Span::styled(format!("{} ", finalized), Style::default().fg(theme.success())),
            Span::styled("author: ", Style::default().fg(theme.muted())),
            Span::styled(author_display, Style::default().fg(theme.text())),
        ]);

        ListItem::new(line)
    }).collect();

    let blocks_list = List::new(blocks_items)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border()))
            .title(Span::styled("Recent Blocks", Style::default().fg(theme.primary()).add_modifier(Modifier::BOLD))));
    f.render_widget(blocks_list, chunks[2]);
}

fn render_blocks(f: &mut Frame, app: &App, area: Rect, layout: &ResponsiveLayout) {
    let theme = app.theme;
    let key_mode = layout.key_display_length();
    let block_cols = layout.block_list_columns();

    let blocks_items: Vec<ListItem> = app.state.recent_blocks.iter().map(|block| {
        let author_display = if let Some(ref author) = block.author_key {
            key_mode.format(author)
        } else {
            "unknown".to_string()
        };

        let finalized = if block.is_finalized { "✓" } else { " " };

        // Standard block format (same for Medium and Large)
        let mut spans = vec![
            Span::styled(format!("#{:<8}", block.block_number), Style::default().fg(theme.block_number())),
            Span::raw("  "),
        ];

        if block_cols.show_slot {
            spans.push(Span::styled(format!("slot {:>12}", block.slot_number), Style::default().fg(theme.muted())));
            spans.push(Span::raw("  "));
        }

        if block_cols.show_epoch {
            spans.push(Span::styled(format!("epoch {:>6}", block.sidechain_epoch), Style::default().fg(theme.epoch())));
            spans.push(Span::raw("  "));
        }

        if block_cols.show_extrinsics {
            spans.push(Span::styled(format!("extr:{:<3}", block.extrinsics_count), Style::default().fg(theme.text())));
            spans.push(Span::raw("  "));
        }

        spans.push(Span::styled(format!("{} ", finalized), Style::default().fg(theme.success())));
        spans.push(Span::styled(author_display, Style::default().fg(theme.text())));

        ListItem::new(Line::from(spans))
    }).collect();

    let title = format!("Blocks ({} total, showing last {}) - Use j/k or ↑/↓ to scroll", app.state.total_blocks, app.state.recent_blocks.len());

    let blocks_list = List::new(blocks_items)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border()))
            .title(Span::styled(title, Style::default().fg(theme.primary()).add_modifier(Modifier::BOLD))))
        .highlight_style(Style::default().bg(theme.highlight()).add_modifier(Modifier::BOLD).fg(theme.text()));

    let mut list_state = ListState::default();
    list_state.select(Some(app.selected_index));
    f.render_stateful_widget(blocks_list, area, &mut list_state);
}

fn render_validators(f: &mut Frame, app: &App, area: Rect, layout: &ResponsiveLayout) {
    let theme = app.theme;
    let key_mode = layout.key_display_length();
    let val_cols = layout.validator_list_columns();

    let validators = if app.show_ours_only {
        &app.state.our_validators
    } else {
        &app.state.validators
    };

    let validator_items: Vec<ListItem> = validators.iter().map(|v| {
        let status = v.registration_status.as_deref().unwrap_or("unknown");
        let ours = if v.is_ours { "★" } else { " " };
        let key_display = key_mode.format(&v.sidechain_key);

        // Get seats from epoch data if available
        let seats_display = app.state.validator_epoch_data
            .get(&v.sidechain_key)
            .map(|epoch| format!("{:>3}", epoch.committee_seats))
            .unwrap_or_else(|| "  -".to_string());

        // Standard validator format (same for Medium and Large)
        let mut spans = vec![
            Span::styled(ours, Style::default().fg(theme.ours())),
            Span::raw(" "),
            Span::styled(key_display, Style::default().fg(theme.secondary())),
            Span::raw("  "),
        ];

        if val_cols.show_status {
            spans.push(Span::styled(format!("{:<12}", status), Style::default().fg(if v.is_ours { theme.success() } else { theme.muted() })));
            spans.push(Span::raw("  "));
        }

        // Add seats column
        let seats_style = if app.state.validator_epoch_data.get(&v.sidechain_key).map(|e| e.committee_seats > 0).unwrap_or(false) {
            Style::default().fg(theme.success())
        } else {
            Style::default().fg(theme.muted())
        };
        spans.push(Span::styled(seats_display, seats_style));
        spans.push(Span::styled(" seats", Style::default().fg(theme.muted())));
        spans.push(Span::raw("  "));

        // Show epoch blocks (not total) - consistent with seats being per-epoch
        let epoch_blocks = app.state.validator_epoch_blocks
            .get(&v.sidechain_key)
            .copied()
            .unwrap_or(0);
        spans.push(Span::styled(format!("{:>4} blocks", epoch_blocks), Style::default().fg(theme.text())));

        ListItem::new(Line::from(spans))
    }).collect();

    let filter_text = if app.show_ours_only { " (ours)" } else { "" };
    let epoch_label = if app.state.sidechain_epoch > 0 {
        format!(" epoch {}", app.state.sidechain_epoch)
    } else {
        String::new()
    };
    let title = format!("Validators ({} total{}{}) - [F] filter, j/k or ↑/↓ scroll", validators.len(), filter_text, epoch_label);

    let validators_list = List::new(validator_items)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border()))
            .title(Span::styled(title, Style::default().fg(theme.primary()).add_modifier(Modifier::BOLD))))
        .highlight_style(Style::default().bg(theme.highlight()).add_modifier(Modifier::BOLD).fg(theme.text()));

    let mut list_state = ListState::default();
    list_state.select(Some(app.selected_index));
    f.render_stateful_widget(validators_list, area, &mut list_state);
}

fn render_performance(f: &mut Frame, app: &App, area: Rect, layout: &ResponsiveLayout) {
    let theme = app.theme;
    let key_mode = layout.key_display_length();

    let mut validators = if app.show_ours_only {
        app.state.our_validators.clone()
    } else {
        app.state.validators.clone()
    };

    // Sort by block count descending
    validators.sort_by(|a, b| b.total_blocks.cmp(&a.total_blocks));

    // Always use total blocks from all validators for percentage (not filtered total)
    let total_blocks = app.state.total_blocks;

    let validator_items: Vec<ListItem> = validators.iter().enumerate().map(|(i, v)| {
        let share = if total_blocks > 0 {
            (v.total_blocks as f64 / total_blocks as f64) * 100.0
        } else {
            0.0
        };

        let ours = if v.is_ours { "★" } else { " " };
        let key_display = key_mode.format(&v.sidechain_key);

        // Standard performance format (same for Medium and Large)
        let line = Line::from(vec![
            Span::styled(format!("{:>3}.", i + 1), Style::default().fg(theme.warning())),
            Span::raw(" "),
            Span::styled(ours, Style::default().fg(theme.ours())),
            Span::raw(" "),
            Span::styled(key_display, Style::default().fg(theme.secondary())),
            Span::raw("  "),
            Span::styled(format!("{:>6} blocks", v.total_blocks), Style::default().fg(theme.text())),
            Span::raw("  "),
            Span::styled(format!("{:>6.3}%", share), Style::default().fg(if v.is_ours { theme.success() } else { theme.muted() })),
        ]);

        ListItem::new(line)
    }).collect();

    let filter_text = if app.show_ours_only { " (ours)" } else { "" };
    let title = format!("Performance Rankings{} - [F] filter, j/k or ↑/↓ scroll", filter_text);

    let performance_list = List::new(validator_items)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border()))
            .title(Span::styled(title, Style::default().fg(theme.primary()).add_modifier(Modifier::BOLD))))
        .highlight_style(Style::default().bg(theme.highlight()).add_modifier(Modifier::BOLD).fg(theme.text()));

    let mut list_state = ListState::default();
    list_state.select(Some(app.selected_index));
    f.render_stateful_widget(performance_list, area, &mut list_state);
}

fn render_peers(f: &mut Frame, app: &App, area: Rect, layout: &ResponsiveLayout) {
    use crate::tui::app::PeerInfo;

    let theme = app.theme;
    let key_mode = layout.key_display_length();

    // Header with peer summary
    let peer_items: Vec<ListItem> = app.state.connected_peers.iter().map(|peer: &PeerInfo| {
        let peer_id_display = key_mode.format(&peer.peer_id);

        // Show sync status compared to our best block
        let sync_status = if peer.best_number >= app.state.chain_tip {
            ("✓", theme.success())  // Ahead or at our tip
        } else if app.state.chain_tip.saturating_sub(peer.best_number) < 10 {
            ("~", theme.warning())  // Within 10 blocks
        } else {
            ("○", theme.muted())    // Behind
        };

        // Connection direction: ↑ = outbound (we dialed), ↓ = inbound (they dialed)
        let direction = if peer.is_outbound { "↑" } else { "↓" };
        let direction_color = if peer.is_outbound { theme.muted() } else { theme.success() };

        // Format address if available
        let addr_display = peer.address.as_ref()
            .map(|a| format!("  {}", a))
            .unwrap_or_default();

        let line = Line::from(vec![
            Span::styled(sync_status.0, Style::default().fg(sync_status.1)),
            Span::styled(direction, Style::default().fg(direction_color)),
            Span::raw(" "),
            Span::styled(peer_id_display, Style::default().fg(theme.secondary())),
            Span::raw("  "),
            Span::styled(format!("#{}", peer.best_number), Style::default().fg(theme.block_number())),
            Span::styled(addr_display, Style::default().fg(theme.muted())),
        ]);

        ListItem::new(line)
    }).collect();

    // Show external IP and our peer ID at top of title
    let external_ip = if app.state.external_ips.is_empty() {
        "unknown".to_string()
    } else {
        app.state.external_ips.join(", ")
    };
    let title = format!(
        "Connected Peers ({}) - External IP: {} - j/k or ↑/↓ scroll",
        app.state.connected_peers.len(),
        external_ip
    );

    let peers_list = List::new(peer_items)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border()))
            .title(Span::styled(title, Style::default().fg(theme.primary()).add_modifier(Modifier::BOLD))))
        .highlight_style(Style::default().bg(theme.highlight()).add_modifier(Modifier::BOLD).fg(theme.text()));

    let mut list_state = ListState::default();
    list_state.select(Some(app.selected_index));
    f.render_stateful_widget(peers_list, area, &mut list_state);
}

fn render_help(f: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme;

    // Build help items as ListItems for scrolling support
    let help_items: Vec<ListItem> = vec![
        ListItem::new(Line::from(vec![
            Span::styled("Keyboard Shortcuts", Style::default().fg(theme.title()).add_modifier(Modifier::BOLD)),
        ])),
        ListItem::new(Line::from("")),
        ListItem::new(Line::from(vec![
            Span::styled("  Navigation:", Style::default().fg(theme.primary()).add_modifier(Modifier::BOLD)),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("    1-5       ", Style::default().fg(theme.text())),
            Span::raw("Switch to view (1=Dashboard, 2=Blocks, 3=Validators, 4=Performance, 5=Peers)"),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("    Tab       ", Style::default().fg(theme.text())),
            Span::raw("Next view"),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("    Shift+Tab ", Style::default().fg(theme.text())),
            Span::raw("Previous view"),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("    ? / h / F1", Style::default().fg(theme.text())),
            Span::raw("  Show this help"),
        ])),
        ListItem::new(Line::from("")),
        ListItem::new(Line::from(vec![
            Span::styled("  Scrolling:", Style::default().fg(theme.primary()).add_modifier(Modifier::BOLD)),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("    ↑ / k     ", Style::default().fg(theme.text())),
            Span::raw("Scroll up"),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("    ↓ / j     ", Style::default().fg(theme.text())),
            Span::raw("Scroll down"),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("    PgUp / K  ", Style::default().fg(theme.text())),
            Span::raw("Page up (10 items)"),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("    PgDn / J  ", Style::default().fg(theme.text())),
            Span::raw("Page down (10 items)"),
        ])),
        ListItem::new(Line::from("")),
        ListItem::new(Line::from(vec![
            Span::styled("  Options:", Style::default().fg(theme.primary()).add_modifier(Modifier::BOLD)),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("    f / F     ", Style::default().fg(theme.text())),
            Span::raw("Toggle 'ours only' filter (Validators/Performance views)"),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("    t / T     ", Style::default().fg(theme.text())),
            Span::raw("Toggle theme (Midnight ⟷ Midday)"),
        ])),
        ListItem::new(Line::from("")),
        ListItem::new(Line::from(vec![
            Span::styled("  Quit:", Style::default().fg(theme.primary()).add_modifier(Modifier::BOLD)),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("    q / Esc   ", Style::default().fg(theme.text())),
            Span::raw("Quit application"),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("    Ctrl+C    ", Style::default().fg(theme.text())),
            Span::raw("Quit application"),
        ])),
        ListItem::new(Line::from("")),
        ListItem::new(Line::from(vec![
            Span::styled("  Views:", Style::default().fg(theme.primary()).add_modifier(Modifier::BOLD)),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("    [1] Dashboard   ", Style::default().fg(theme.text())),
            Span::raw("Network status, validator info, recent blocks"),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("    [2] Blocks      ", Style::default().fg(theme.text())),
            Span::raw("Detailed block list with authors and slots"),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("    [3] Validators  ", Style::default().fg(theme.text())),
            Span::raw("All validators with registration status"),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("    [4] Performance ", Style::default().fg(theme.text())),
            Span::raw("Rankings by block production share"),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("    [5] Peers       ", Style::default().fg(theme.text())),
            Span::raw("Connected peers with sync status and IPs"),
        ])),
        ListItem::new(Line::from("")),
        ListItem::new(Line::from(vec![
            Span::styled("  Symbols:", Style::default().fg(theme.primary()).add_modifier(Modifier::BOLD)),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("    ★  ", Style::default().fg(theme.ours())),
            Span::raw("Our validator (from keystore)"),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("    ✓  ", Style::default().fg(theme.success())),
            Span::raw("Finalized/synced/elected status"),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("    ✗  ", Style::default().fg(theme.warning())),
            Span::raw("Not elected to committee this epoch"),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("    ●  ", Style::default().fg(theme.success())),
            Span::raw("Connected to node"),
        ])),
        ListItem::new(Line::from("")),
        ListItem::new(Line::from(vec![
            Span::styled("  Dashboard Fields:", Style::default().fg(theme.primary()).add_modifier(Modifier::BOLD)),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("    Sidechain epoch  ", Style::default().fg(theme.text())),
            Span::raw("Committee election cycle (2h preview, TBD mainnet)"),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("    Mainchain epoch  ", Style::default().fg(theme.text())),
            Span::raw("Cardano epoch (24h preview, 5d preprod/mainnet)"),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("    Grandpa ✓        ", Style::default().fg(theme.text())),
            Span::raw("Node is participating in block finalization"),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("    This Epoch       ", Style::default().fg(theme.text())),
            Span::raw("Blocks produced in current sidechain epoch"),
        ])),
    ];

    let item_count = help_items.len();

    let help_list = List::new(help_items)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border()))
            .title(Span::styled("Help - Use j/k/J/K or ↑/↓ to scroll", Style::default().fg(theme.primary()).add_modifier(Modifier::BOLD))))
        .highlight_style(Style::default().bg(theme.highlight()).add_modifier(Modifier::BOLD).fg(theme.text()));

    let mut list_state = ListState::default();
    list_state.select(Some(app.selected_index));
    f.render_stateful_widget(help_list, area, &mut list_state);

    // Render scrollbar
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));

    let mut scrollbar_state = ScrollbarState::new(item_count)
        .position(app.selected_index);

    // Render scrollbar in the same area (it will appear on the right edge)
    f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
}

/// Format bytes into human-readable string (KB, MB, GB)
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1}GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1}MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.0}KB", bytes as f64 / KB as f64)
    } else {
        format!("{}B", bytes)
    }
}

/// Format uptime seconds into human-readable string
fn format_uptime(secs: u64) -> String {
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;

    if days > 0 {
        format!("{}d {}h", days, hours)
    } else if hours > 0 {
        format!("{}h {}m", hours, mins)
    } else if mins > 0 {
        format!("{}m", mins)
    } else {
        format!("{}s", secs)
    }
}
