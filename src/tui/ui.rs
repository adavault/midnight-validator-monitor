//! UI rendering for TUI

use crate::db::CommitteeSelectionStats;
use crate::tui::layout::ResponsiveLayout;
use crate::tui::{App, ScreenSize, ViewMode};
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Table, TableState,
    },
    Frame,
};

/// Convert a slice of values to Unicode sparkline bars
/// Uses block characters: ▁▂▃▄▅▆▇█ (8 levels)
#[allow(dead_code)]
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

/// Create colored sparkline spans - each bar colored based on blocks vs seats
/// Two-tier coloring: normal (purple) for everything, red only for missed blocks
fn sparkline_colored_spans<'a>(
    blocks: &[u64],
    seats: &[u64],
    normal_color: Color,
    error_color: Color,
) -> Vec<Span<'a>> {
    const BARS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

    if blocks.is_empty() {
        return vec![];
    }

    let max = *blocks.iter().max().unwrap_or(&1);

    blocks
        .iter()
        .zip(seats.iter().chain(std::iter::repeat(&0u64)))
        .map(|(&block_count, &seat_count)| {
            let bar_char = if max == 0 || block_count == 0 {
                BARS[0]
            } else {
                let idx = ((block_count as f64 / max as f64) * 7.0).round() as usize;
                BARS[idx.min(7)]
            };

            // Red only for missed blocks, purple for everything else
            let color = if seat_count > 0 && block_count < seat_count {
                error_color // Missed blocks - highlight problem
            } else {
                normal_color // Normal - not selected or produced all
            };

            Span::styled(bar_char.to_string(), Style::default().fg(color))
        })
        .collect()
}

/// Render the UI with responsive layout
pub fn render(f: &mut Frame, app: &App) {
    let layout = ResponsiveLayout::new(f.area());
    let chunks = layout.main_layout(f.area());

    // Render title bar (compact for small screens)
    render_title_bar(f, app, chunks[0], &layout);

    // Render main content based on view mode
    match app.view_mode {
        ViewMode::Dashboard => render_dashboard(f, app, chunks[1], &layout),
        ViewMode::Blocks => render_blocks(f, app, chunks[1], &layout),
        ViewMode::Validators => render_validators(f, app, chunks[1], &layout),
        ViewMode::Performance | ViewMode::ValidatorEpochDetail => {
            render_performance(f, app, chunks[1], &layout)
        }
        ViewMode::Peers => render_peers(f, app, chunks[1], &layout),
        ViewMode::Help => render_help(f, app, chunks[1]),
    }

    // Render status bar (compact for small screens)
    render_status_bar(f, app, chunks[2], &layout);

    // Render popup overlay if present
    if let Some(ref popup) = app.popup {
        render_popup(f, app, popup);
    }
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
    let view_label = match app.view_mode {
        ViewMode::Dashboard => "[1] Dashboard",
        ViewMode::Blocks => "[2] Blocks",
        ViewMode::Validators => "[3] Validators",
        ViewMode::Performance | ViewMode::ValidatorEpochDetail => "[4] Performance",
        ViewMode::Peers => "[5] Peers",
        ViewMode::Help => "[?] Help",
    };

    let left_text = vec![
        Span::styled(
            "Midnight Validator Monitor",
            Style::default()
                .fg(theme.title())
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  |  "),
        Span::styled(
            view_label,
            Style::default()
                .fg(theme.primary())
                .add_modifier(Modifier::BOLD),
        ),
    ];

    let left_paragraph = Paragraph::new(Line::from(left_text)).alignment(Alignment::Left);
    f.render_widget(left_paragraph, chunks[0]);

    // Right side: chain name and hostname (bold)
    let right_text = if app.state.chain_name.is_empty() {
        Line::from(vec![
            Span::styled(
                &app.state.node_name,
                Style::default()
                    .fg(theme.secondary())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
        ])
    } else {
        Line::from(vec![
            Span::styled(&app.state.chain_name, Style::default().fg(theme.epoch())),
            Span::styled(" @ ", Style::default().fg(theme.muted())),
            Span::styled(
                &app.state.node_name,
                Style::default()
                    .fg(theme.secondary())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
        ])
    };
    let right_paragraph = Paragraph::new(right_text).alignment(Alignment::Right);
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

    // Split inner area: left for status, right for MVM/version/theme
    let right_width = match layout.size {
        ScreenSize::Medium => 24, // MVM: 12345 ☽ Night
        ScreenSize::Large => 45,  // MVM: 12345  Node: v0.5.6  ☽ Midnight
    };
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(right_width)])
        .split(inner_area);

    // Left side: status info - compact for narrow screens
    // Show contextual hints based on current view/state
    let left_text = if app.has_popup() {
        // Popup is open - show dismiss hint
        vec![
            Span::styled("●", Style::default().fg(theme.success())),
            Span::styled(
                format!(" {}s ago  |  ", since_update),
                Style::default().fg(theme.text()),
            ),
            Span::styled("[Esc]", Style::default().fg(theme.primary())),
            Span::styled(" Close popup  ", Style::default().fg(theme.muted())),
            // Show scroll hint for validator detail popup
            if matches!(app.popup, Some(PopupContent::ValidatorDetail { .. })) {
                Span::styled("[j/k]", Style::default().fg(theme.primary()))
            } else {
                Span::raw("")
            },
            if matches!(app.popup, Some(PopupContent::ValidatorDetail { .. })) {
                Span::styled(" Scroll", Style::default().fg(theme.muted()))
            } else {
                Span::raw("")
            },
        ]
    } else if app.state.is_loading {
        vec![
            Span::styled("◌ ", Style::default().fg(theme.warning())),
            Span::styled("Loading...", Style::default().fg(theme.text())),
        ]
    } else if let Some(ref err) = app.state.last_error {
        vec![
            Span::styled(
                "ERR: ",
                Style::default()
                    .fg(theme.error())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(err.clone(), Style::default().fg(theme.error())),
        ]
    } else {
        // Show contextual hints based on view
        let enter_hint = match app.view_mode {
            ViewMode::Blocks => Some("Block details"),
            ViewMode::Validators => Some("Identity"),
            ViewMode::Performance => Some("Epoch history"),
            ViewMode::Peers => Some("Peer details"),
            _ => None,
        };

        match layout.size {
            ScreenSize::Medium => {
                // Compact status for narrow screens
                let mut spans = vec![
                    Span::styled("●", Style::default().fg(theme.success())),
                    Span::styled(
                        format!(" {}s ago | ", since_update),
                        Style::default().fg(theme.text()),
                    ),
                    Span::styled("[1-5]", Style::default().fg(theme.primary())),
                    Span::styled(" Views | ", Style::default().fg(theme.muted())),
                ];
                if let Some(hint) = enter_hint {
                    spans.push(Span::styled("Enter", Style::default().fg(theme.primary())));
                    spans.push(Span::styled(
                        format!(" {} | ", hint),
                        Style::default().fg(theme.muted()),
                    ));
                }
                spans.push(Span::styled("?", Style::default().fg(theme.primary())));
                spans.push(Span::styled(" Help", Style::default().fg(theme.muted())));
                spans
            }
            ScreenSize::Large => {
                // Full status for wide screens
                let mut spans = vec![
                    Span::styled("●", Style::default().fg(theme.success())),
                    Span::styled(
                        format!(" Connected  |  Updated {}s ago  |  ", since_update),
                        Style::default().fg(theme.text()),
                    ),
                    Span::styled("[1-5]", Style::default().fg(theme.primary())),
                    Span::styled(" Views  ", Style::default().fg(theme.muted())),
                ];
                if let Some(hint) = enter_hint {
                    spans.push(Span::styled(
                        "[Enter]",
                        Style::default().fg(theme.primary()),
                    ));
                    spans.push(Span::styled(
                        format!(" {}  ", hint),
                        Style::default().fg(theme.muted()),
                    ));
                }
                spans.push(Span::styled("[Q]", Style::default().fg(theme.primary())));
                spans.push(Span::styled(" Quit  ", Style::default().fg(theme.muted())));
                spans.push(Span::styled("[?]", Style::default().fg(theme.primary())));
                spans.push(Span::styled(" Help", Style::default().fg(theme.muted())));
                spans
            }
        }
    };

    let left_paragraph = Paragraph::new(Line::from(left_text)).alignment(Alignment::Left);
    f.render_widget(left_paragraph, chunks[0]);

    // Right side: MVM status, node version, theme name
    // Calculate MVM sync status
    let mvm_last_block = app
        .state
        .recent_blocks
        .first()
        .map(|b| b.block_number)
        .unwrap_or(0);
    let mvm_sync_pct = if app.state.chain_tip > 0 && mvm_last_block > 0 {
        (mvm_last_block as f64 / app.state.chain_tip as f64) * 100.0
    } else if app.state.total_blocks > 0 {
        100.0
    } else {
        0.0
    };
    let mvm_synced = mvm_sync_pct >= 99.9;

    // Format node version (trim git hash)
    let version_display = if app.state.node_version.contains('-') {
        app.state
            .node_version
            .split('-')
            .next()
            .unwrap_or(&app.state.node_version)
    } else if app.state.node_version.is_empty() {
        "?"
    } else {
        &app.state.node_version
    };

    let (theme_icon, theme_display) = if theme.name().contains("Midnight") {
        (
            "☽ ",
            match layout.size {
                ScreenSize::Medium => "Night",
                ScreenSize::Large => "Midnight",
            },
        )
    } else {
        (
            "☀ ",
            match layout.size {
                ScreenSize::Medium => "Day",
                ScreenSize::Large => "Midday",
            },
        )
    };

    let mvm_color = if mvm_synced {
        theme.text()
    } else {
        theme.warning()
    };
    let mvm_display = if mvm_synced {
        format!("{}", app.state.total_blocks)
    } else {
        format!("{} ({:.0}%)", app.state.total_blocks, mvm_sync_pct)
    };

    let right_text = match layout.size {
        ScreenSize::Medium => Line::from(vec![
            Span::styled("MVM:", Style::default().fg(theme.muted())),
            Span::styled(format!("{} ", mvm_display), Style::default().fg(mvm_color)),
            Span::styled(theme_icon, Style::default().fg(theme.primary())),
            Span::styled(theme_display, Style::default().fg(theme.secondary())),
            Span::raw(" "),
        ]),
        ScreenSize::Large => Line::from(vec![
            Span::styled("MVM: ", Style::default().fg(theme.muted())),
            Span::styled(format!("{}  ", mvm_display), Style::default().fg(mvm_color)),
            Span::styled("Node: ", Style::default().fg(theme.muted())),
            Span::styled(
                format!("v{}  ", version_display),
                Style::default().fg(theme.text()),
            ),
            Span::styled(theme_icon, Style::default().fg(theme.primary())),
            Span::styled(theme_display, Style::default().fg(theme.secondary())),
            Span::raw(" "),
        ]),
    };
    let right_paragraph = Paragraph::new(right_text).alignment(Alignment::Right);
    f.render_widget(right_paragraph, chunks[1]);
}

fn render_dashboard(f: &mut Frame, app: &App, area: Rect, layout: &ResponsiveLayout) {
    let theme = app.theme;
    let key_mode = layout.key_display_length();

    // Calculate dynamic row count for Network Status panel
    // Base: 7 rows (Node, Block, Sidechain, Mainchain, Identity, Bandwidth/Peers, Tx Pool)
    let mut network_rows: u16 = 7;
    if !app.state.sync_progress.is_synced {
        network_rows += 1; // Sync detail row
    }
    if app.state.system_memory_total_bytes > 0 {
        network_rows += 1; // System row
                           // Check if memory warning will be shown
        let mem_percent = (app.state.system_memory_used_bytes as f64
            / app.state.system_memory_total_bytes as f64)
            * 100.0;
        if mem_percent > 85.0 {
            network_rows += 1; // Memory warning row
        }
    }
    let chunks = layout.dashboard_layout(area, network_rows);

    // Show loading state if still loading initial data
    if app.state.is_loading {
        let loading_text = vec![
            Line::from(vec![
                Span::styled("◌ ", Style::default().fg(theme.warning())),
                Span::styled(
                    "Connecting to node and loading data...",
                    Style::default().fg(theme.text()),
                ),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "  This may take a few seconds on first startup.",
                Style::default().fg(theme.muted()),
            )]),
        ];
        let loading_widget = Paragraph::new(loading_text).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border()))
                .title(Span::styled(
                    "Network Status",
                    Style::default()
                        .fg(theme.primary())
                        .add_modifier(Modifier::BOLD),
                )),
        );
        f.render_widget(loading_widget, chunks[0]);

        // Empty placeholders for other panels
        let placeholder = Paragraph::new("").block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border()))
                .title(Span::styled(
                    "Our Validator",
                    Style::default()
                        .fg(theme.ours())
                        .add_modifier(Modifier::BOLD),
                )),
        );
        f.render_widget(placeholder, chunks[1]);

        let placeholder2 = Paragraph::new("").block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border()))
                .title(Span::styled(
                    "Recent Blocks",
                    Style::default()
                        .fg(theme.primary())
                        .add_modifier(Modifier::BOLD),
                )),
        );
        f.render_widget(placeholder2, chunks[2]);
        return;
    }

    // Create epoch progress bars (full width for epochs)
    let epoch_progress = &app.state.epoch_progress;
    let epoch_bar_width = 30; // Wider bars for full-width epoch rows
    let sidechain_filled =
        ((epoch_progress.progress_percent / 100.0) * epoch_bar_width as f64) as usize;
    let sidechain_bar: String = format!(
        "{}{}",
        "━".repeat(sidechain_filled.min(epoch_bar_width)),
        "░".repeat(epoch_bar_width.saturating_sub(sidechain_filled))
    );
    let mainchain_filled =
        ((epoch_progress.mainchain_progress_percent / 100.0) * epoch_bar_width as f64) as usize;
    let mainchain_bar: String = format!(
        "{}{}",
        "━".repeat(mainchain_filled.min(epoch_bar_width)),
        "░".repeat(epoch_bar_width.saturating_sub(mainchain_filled))
    );

    // Build sync progress bar (12 chars + space + percentage fits in 22 char column)
    let sync = &app.state.sync_progress;
    let sync_bar_width = 12;
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

    // Prepare common values
    let uptime = format_uptime(app.state.uptime_secs);
    let bandwidth_in = format_bytes(app.state.bandwidth_in);
    let bandwidth_out = format_bytes(app.state.bandwidth_out);

    // Two-column layout with fixed positions
    // Column 1: 14-char label + value padded to 22 chars = 36 chars total
    // Column 2: 14-char label + value

    // Format block and finalized display
    let block_str = format!("#{}", app.state.chain_tip);
    let finalized_lag = app
        .state
        .chain_tip
        .saturating_sub(app.state.finalized_block);
    let finalized_str = if finalized_lag == 0 {
        format!("#{} (tip)", app.state.finalized_block)
    } else {
        format!("#{} (-{})", app.state.finalized_block, finalized_lag)
    };

    // Row 1: Node sync + Uptime
    let mut network_text = vec![];
    if sync.is_synced {
        network_text.push(Line::from(vec![
            Span::styled("Node:         ", Style::default().fg(theme.muted())),
            Span::styled(
                format!("{:<22}", format!("{} Synced", sync_icon)),
                Style::default().fg(sync_color),
            ),
            Span::styled("Uptime:       ", Style::default().fg(theme.muted())),
            Span::styled(uptime.clone(), Style::default().fg(theme.text())),
        ]));
    } else {
        let sync_display = format!("{} {:.1}%", sync_bar, sync.sync_percent);
        network_text.push(Line::from(vec![
            Span::styled("Node:         ", Style::default().fg(theme.muted())),
            Span::styled(
                format!("{:<22}", sync_display),
                Style::default().fg(theme.warning()),
            ),
            Span::styled("Uptime:       ", Style::default().fg(theme.muted())),
            Span::styled(uptime.clone(), Style::default().fg(theme.text())),
        ]));
        // Row 1b: Sync details with ETA (only when syncing)
        let eta_str = if let Some(eta_secs) = sync.eta_seconds {
            format!("ETA {}", format_uptime(eta_secs))
        } else {
            "ETA --".to_string()
        };
        let rate_str = if sync.sync_rate_bps > 0.1 {
            format!("{:.0} blk/s", sync.sync_rate_bps)
        } else {
            "-- blk/s".to_string()
        };
        network_text.push(Line::from(vec![
            Span::styled("Sync:         ", Style::default().fg(theme.muted())),
            Span::styled(
                format!("{} remaining", sync.blocks_remaining),
                Style::default().fg(theme.warning()),
            ),
            Span::styled("  ", Style::default()),
            Span::styled(rate_str, Style::default().fg(theme.warning())),
            Span::styled("  ", Style::default()),
            Span::styled(eta_str, Style::default().fg(theme.warning())),
        ]));
    }

    // Row 2: Block + Finalized
    network_text.push(Line::from(vec![
        Span::styled("Block:        ", Style::default().fg(theme.muted())),
        Span::styled(
            format!("{:<22}", block_str),
            Style::default().fg(theme.block_number()),
        ),
        Span::styled("Finalized:    ", Style::default().fg(theme.muted())),
        Span::styled(finalized_str, Style::default().fg(theme.text())),
    ]));

    // Row 3: Sidechain epoch (full width with longer bar + countdown)
    // Highlight countdown in warning color only when >90% through epoch (last 10%)
    let sidechain_countdown = format_countdown(epoch_progress.sidechain_time_remaining_secs);
    let sidechain_countdown_color = if epoch_progress.progress_percent > 90.0 {
        theme.warning()
    } else {
        theme.muted()
    };
    network_text.push(Line::from(vec![
        Span::styled("Sidechain:    ", Style::default().fg(theme.muted())),
        Span::styled(
            format!("epoch {:<6}", app.state.sidechain_epoch),
            Style::default().fg(theme.epoch()),
        ),
        Span::styled(sidechain_bar, Style::default().fg(theme.primary())),
        Span::styled(
            format!(" {:>5.1}%", epoch_progress.progress_percent),
            Style::default().fg(theme.text()),
        ),
        Span::styled("  next ", Style::default().fg(theme.muted())),
        Span::styled(
            sidechain_countdown,
            Style::default().fg(sidechain_countdown_color),
        ),
    ]));

    // Row 4: Mainchain epoch (full width with longer bar + countdown)
    let mainchain_countdown = format_countdown(epoch_progress.mainchain_time_remaining_secs);
    let mainchain_countdown_color = if epoch_progress.mainchain_progress_percent > 90.0 {
        theme.warning()
    } else {
        theme.muted()
    };
    network_text.push(Line::from(vec![
        Span::styled("Mainchain:    ", Style::default().fg(theme.muted())),
        Span::styled(
            format!("epoch {:<6}", app.state.mainchain_epoch),
            Style::default().fg(theme.epoch()),
        ),
        Span::styled(mainchain_bar, Style::default().fg(theme.primary())),
        Span::styled(
            format!(" {:>5.1}%", epoch_progress.mainchain_progress_percent),
            Style::default().fg(theme.text()),
        ),
        Span::styled("  next ", Style::default().fg(theme.muted())),
        Span::styled(
            mainchain_countdown,
            Style::default().fg(mainchain_countdown_color),
        ),
    ]));

    // Row 5: Network identity (external IP + peer ID)
    let external_ip = if app.state.external_ips.is_empty() {
        "unknown".to_string()
    } else {
        app.state.external_ips.join(", ")
    };
    let peer_id_display = if app.state.local_peer_id.len() > 16 {
        format!(
            "{}...{}",
            &app.state.local_peer_id[..8],
            &app.state.local_peer_id[app.state.local_peer_id.len() - 4..]
        )
    } else if app.state.local_peer_id.is_empty() {
        "unknown".to_string()
    } else {
        app.state.local_peer_id.clone()
    };
    network_text.push(Line::from(vec![
        Span::styled("Identity:     ", Style::default().fg(theme.muted())),
        Span::styled(
            format!("{:<22}", external_ip),
            Style::default().fg(theme.text()),
        ),
        Span::styled(peer_id_display, Style::default().fg(theme.secondary())),
    ]));

    // Row 6: Bandwidth + Peers (network I/O grouped, color-coded like Peers view)
    // ↓ inbound = green (success), ↑ outbound = muted
    network_text.push(Line::from(vec![
        Span::styled("Bandwidth:    ", Style::default().fg(theme.muted())),
        Span::styled("↓", Style::default().fg(theme.success())),
        Span::styled("↑ ", Style::default().fg(theme.muted())),
        Span::styled(
            bandwidth_in.to_string(),
            Style::default().fg(theme.success()),
        ),
        Span::styled(" / ", Style::default().fg(theme.text())),
        Span::styled(
            format!("{:<10}", bandwidth_out),
            Style::default().fg(theme.muted()),
        ),
        Span::styled("Peers:        ", Style::default().fg(theme.muted())),
        Span::styled("↓", Style::default().fg(theme.success())),
        Span::styled("↑ ", Style::default().fg(theme.muted())),
        Span::styled(
            format!("{}", app.state.peers_inbound),
            Style::default().fg(theme.success()),
        ),
        Span::styled(" / ", Style::default().fg(theme.text())),
        Span::styled(
            format!("{}", app.state.peers_outbound),
            Style::default().fg(theme.muted()),
        ),
    ]));

    // Row 7: Tx Pool
    let txpool_str = format!("{} ready", app.state.txpool_ready);
    network_text.push(Line::from(vec![
        Span::styled("Tx Pool:      ", Style::default().fg(theme.muted())),
        Span::styled(txpool_str, Style::default().fg(theme.text())),
    ]));

    // Row 8: System resources (from node_exporter if configured) - infrastructure last
    if app.state.system_memory_total_bytes > 0 {
        let mem_used = format_bytes(app.state.system_memory_used_bytes);
        let mem_total = format_bytes(app.state.system_memory_total_bytes);
        let disk_used = format_bytes(app.state.system_disk_used_bytes);
        let disk_total = format_bytes(app.state.system_disk_total_bytes);

        // Calculate memory percentage and determine color/warning
        let mem_percent = (app.state.system_memory_used_bytes as f64
            / app.state.system_memory_total_bytes as f64)
            * 100.0;
        let mem_color = if mem_percent > 90.0 {
            theme.error() // Critical
        } else if mem_percent > 80.0 {
            theme.warning() // Warning
        } else {
            theme.text() // Normal
        };

        // Memory trend indicator
        use crate::tui::app::MemoryTrend;
        let trend_indicator = match app.state.memory_trend {
            MemoryTrend::Rising => ("↑", theme.warning()),
            MemoryTrend::Falling => ("↓", theme.success()),
            MemoryTrend::Stable => ("─", theme.muted()),
        };

        network_text.push(Line::from(vec![
            Span::styled("System:       ", Style::default().fg(theme.muted())),
            Span::styled("Mem ", Style::default().fg(theme.muted())),
            Span::styled(
                format!("{}/{}", mem_used, mem_total),
                Style::default().fg(mem_color),
            ),
            Span::styled(trend_indicator.0, Style::default().fg(trend_indicator.1)),
            Span::styled("  ", Style::default()),
            Span::styled("Disk ", Style::default().fg(theme.muted())),
            Span::styled(
                format!("{}/{}  ", disk_used, disk_total),
                Style::default().fg(theme.text()),
            ),
            Span::styled("Load ", Style::default().fg(theme.muted())),
            Span::styled(
                format!("{:.2}", app.state.system_load1),
                Style::default().fg(theme.text()),
            ),
        ]));

        // Add memory warning if high usage
        if mem_percent > 85.0 {
            network_text.push(Line::from(vec![
                Span::styled("              ", Style::default()),
                Span::styled("⚠ ", Style::default().fg(theme.warning())),
                Span::styled(
                    format!(
                        "Memory usage high ({:.0}%) - Node may crash if this continues",
                        mem_percent
                    ),
                    Style::default().fg(theme.warning()),
                ),
            ]));
        }
    }

    let network_widget = Paragraph::new(network_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border()))
            .title(Span::styled(
                "Network Status",
                Style::default()
                    .fg(theme.primary())
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(network_widget, chunks[0]);

    // Our validators - responsive with epoch block predictions
    let our_validators_text = if app.state.our_validators_count > 0 {
        let total_our_blocks: u64 = app
            .state
            .our_validators
            .iter()
            .map(|v| v.total_blocks)
            .sum();
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
            if ratio >= 0.9 {
                "✓"
            } else if ratio >= 0.5 {
                "○"
            } else {
                "!"
            }
        } else {
            "?"
        };
        let perf_color = if performance_indicator == "✓" {
            theme.success()
        } else if performance_indicator == "!" {
            theme.warning()
        } else {
            theme.muted()
        };

        // Panel height (9) fits 1 validator with all 3 keys (4 header + 3 key lines + 2 border)
        let max_validators = 1;

        // Committee election status
        let (committee_icon, committee_color) = if app.state.committee_elected {
            ("✓", theme.success())
        } else {
            ("✗", theme.warning())
        };

        // Committee status - compact format
        let committee_status = if app.state.committee_elected {
            format!(
                "{} Elected ({} / {})",
                committee_icon, app.state.committee_seats, app.state.committee_size
            )
        } else if app.state.committee_size > 0 {
            format!("{} Not elected", committee_icon)
        } else {
            "? Checking...".to_string()
        };

        let blocks_str = format!("{} blocks", total_our_blocks);
        let epoch_str = format!("{} blocks", epoch_blocks);

        // Sparkline performance coloring: green >= 90%, warning 70-90%, error < 70%
        let sparkline_blocks: u64 = app.state.our_blocks_sparkline.iter().sum();
        let sparkline_seats = app.state.sparkline_total_seats;
        let sparkline_ratio = if sparkline_seats > 0 {
            sparkline_blocks as f64 / sparkline_seats as f64
        } else {
            1.0 // No seats = no missed blocks
        };
        let _sparkline_color = if sparkline_ratio >= 0.9 {
            theme.primary() // Good performance (>= 90%)
        } else if sparkline_ratio >= 0.7 {
            theme.warning() // Moderate issues (70-90%)
        } else {
            theme.error() // Significant issues (< 70%)
        };

        // GRANDPA voter status
        let (grandpa_icon, grandpa_color) = if app.state.grandpa_voter {
            ("✓ Voting", theme.success())
        } else {
            ("○ Not voting", theme.muted())
        };

        let mut lines = vec![
            // Row 1: Committee + GRANDPA (validator participation statuses)
            Line::from(vec![
                Span::styled("Committee:    ", Style::default().fg(theme.muted())),
                Span::styled(
                    format!("{:<22}", committee_status),
                    Style::default().fg(committee_color),
                ),
                Span::styled("GRANDPA:      ", Style::default().fg(theme.muted())),
                Span::styled(grandpa_icon, Style::default().fg(grandpa_color)),
            ]),
            // Row 2: All-time blocks + Share
            Line::from(vec![
                Span::styled("All-Time:     ", Style::default().fg(theme.muted())),
                Span::styled(
                    format!("{:<22}", blocks_str),
                    Style::default().fg(theme.success()),
                ),
                Span::styled("Share:        ", Style::default().fg(theme.muted())),
                Span::styled(
                    format!("{:.3}%", share),
                    Style::default().fg(theme.success()),
                ),
            ]),
            // Row 3: This epoch + Expected
            Line::from(vec![
                Span::styled("This Epoch:   ", Style::default().fg(theme.muted())),
                Span::styled(
                    format!("{:<22}", epoch_str),
                    Style::default().fg(theme.primary()),
                ),
                Span::styled("Expected:     ", Style::default().fg(theme.muted())),
                Span::styled(
                    format!("~{:.1} ", expected_blocks),
                    Style::default().fg(theme.text()),
                ),
                Span::styled(performance_indicator, Style::default().fg(perf_color)),
            ]),
            // Row 4: Sparkline (24 epoch trend, spans both columns)
            // Each bar colored individually: normal for met expectations, red for missed blocks
            {
                let mut sparkline_spans = vec![Span::styled(
                    "24 Epochs:    ",
                    Style::default().fg(theme.muted()),
                )];
                sparkline_spans.extend(sparkline_colored_spans(
                    &app.state.our_blocks_sparkline,
                    &app.state.our_seats_sparkline,
                    theme.primary(), // Purple - normal
                    theme.error(),   // Red - missed blocks only
                ));
                sparkline_spans.push(Span::styled(
                    format!(
                        "  ({} blocks / {} seats)",
                        sparkline_blocks, sparkline_seats
                    ),
                    Style::default().fg(theme.text()),
                ));
                Line::from(sparkline_spans)
            },
        ];

        // Show validators with all three public keys (14-char labels)
        for v in app.state.our_validators.iter().take(max_validators) {
            let sidechain_display = key_mode.format(&v.sidechain_key);
            let label = v
                .label
                .as_ref()
                .map(|l| format!(" ({})", l))
                .unwrap_or_default();

            // Row 5: Sidechain key
            lines.push(Line::from(vec![
                Span::styled("* Sidechain:  ", Style::default().fg(theme.ours())),
                Span::styled(
                    sidechain_display.clone(),
                    Style::default().fg(theme.secondary()),
                ),
                Span::styled(label.clone(), Style::default().fg(theme.muted())),
            ]));

            // Row 6: AURA key
            if let Some(ref aura_key) = v.aura_key {
                let aura_display = key_mode.format(aura_key);
                lines.push(Line::from(vec![
                    Span::styled("  AURA:       ", Style::default().fg(theme.muted())),
                    Span::styled(aura_display, Style::default().fg(theme.text())),
                ]));
            }

            // Row 7: Grandpa key
            if let Some(ref grandpa_key) = v.grandpa_key {
                let grandpa_display = key_mode.format(grandpa_key);
                lines.push(Line::from(vec![
                    Span::styled("  Grandpa:    ", Style::default().fg(theme.muted())),
                    Span::styled(grandpa_display, Style::default().fg(theme.text())),
                ]));
            }
        }

        if app.state.our_validators_count > max_validators as u64 {
            lines.push(Line::from(vec![Span::styled(
                format!(
                    "  +{} more validators",
                    app.state.our_validators_count - max_validators as u64
                ),
                Style::default().fg(theme.muted()),
            )]));
        }

        lines
    } else {
        // No validators message (same for Medium and Large)
        vec![
            Line::from(vec![Span::styled(
                "No validators marked as 'ours'",
                Style::default().fg(theme.warning()),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Run: ", Style::default().fg(theme.muted())),
                Span::styled(
                    "mvm keys --keystore <path> verify",
                    Style::default().fg(theme.secondary()),
                ),
            ]),
        ]
    };

    let our_validators_widget = Paragraph::new(our_validators_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border()))
            .title(Span::styled(
                "Our Validator",
                Style::default()
                    .fg(theme.ours())
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(our_validators_widget, chunks[1]);

    // Recent blocks - fill available space (panel height minus 2 for borders)
    let blocks_to_show = chunks[2].height.saturating_sub(2) as usize;

    let blocks_items: Vec<ListItem> = app
        .state
        .recent_blocks
        .iter()
        .take(blocks_to_show)
        .map(|block| {
            let author_display = if let Some(ref author) = block.author_key {
                key_mode.format(author)
            } else {
                "unknown".to_string()
            };

            let finalized = if block.is_finalized { "✓" } else { " " };

            // Standard block format (same for Medium and Large)
            let line = Line::from(vec![
                Span::styled(
                    format!("#{:<8}", block.block_number),
                    Style::default().fg(theme.block_number()),
                ),
                Span::raw("  "),
                Span::styled(
                    format!("slot {:>12}", block.slot_number),
                    Style::default().fg(theme.muted()),
                ),
                Span::raw("  "),
                Span::styled(
                    format!("epoch {:>6}", block.sidechain_epoch),
                    Style::default().fg(theme.epoch()),
                ),
                Span::raw("  "),
                Span::styled(
                    format!("{} ", finalized),
                    Style::default().fg(theme.success()),
                ),
                Span::styled("author: ", Style::default().fg(theme.muted())),
                Span::styled(author_display, Style::default().fg(theme.text())),
            ]);

            ListItem::new(line)
        })
        .collect();

    let blocks_list = List::new(blocks_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border()))
            .title(Span::styled(
                "Recent Blocks",
                Style::default()
                    .fg(theme.primary())
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(blocks_list, chunks[2]);
}

fn render_blocks(f: &mut Frame, app: &App, area: Rect, layout: &ResponsiveLayout) {
    let theme = app.theme;
    let key_mode = layout.key_display_length();
    let block_cols = layout.block_list_columns();

    let blocks_items: Vec<ListItem> = app
        .state
        .recent_blocks
        .iter()
        .map(|block| {
            let author_display = if let Some(ref author) = block.author_key {
                key_mode.format(author)
            } else {
                "unknown".to_string()
            };

            let finalized = if block.is_finalized { "✓" } else { " " };

            // Standard block format (same for Medium and Large)
            let mut spans = vec![
                Span::styled(
                    format!("#{:<8}", block.block_number),
                    Style::default().fg(theme.block_number()),
                ),
                Span::raw("  "),
            ];

            if block_cols.show_slot {
                spans.push(Span::styled(
                    format!("slot {:>12}", block.slot_number),
                    Style::default().fg(theme.muted()),
                ));
                spans.push(Span::raw("  "));
            }

            if block_cols.show_epoch {
                spans.push(Span::styled(
                    format!("epoch {:>6}", block.sidechain_epoch),
                    Style::default().fg(theme.epoch()),
                ));
                spans.push(Span::raw("  "));
            }

            if block_cols.show_extrinsics {
                spans.push(Span::styled(
                    format!("extr:{:<3}", block.extrinsics_count),
                    Style::default().fg(theme.text()),
                ));
                spans.push(Span::raw("  "));
            }

            spans.push(Span::styled(
                format!("{} ", finalized),
                Style::default().fg(theme.success()),
            ));
            spans.push(Span::styled(
                author_display,
                Style::default().fg(theme.text()),
            ));

            ListItem::new(Line::from(spans))
        })
        .collect();

    let title = format!(
        "Blocks ({} total, showing last {}) - Use j/k or ↑/↓ to scroll",
        app.state.total_blocks,
        app.state.recent_blocks.len()
    );

    let blocks_list = List::new(blocks_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border()))
                .title(Span::styled(
                    title,
                    Style::default()
                        .fg(theme.primary())
                        .add_modifier(Modifier::BOLD),
                )),
        )
        .highlight_style(
            Style::default()
                .bg(theme.highlight())
                .add_modifier(Modifier::BOLD)
                .fg(theme.text()),
        );

    let mut list_state = ListState::default();
    list_state.select(Some(app.selected_index()));
    f.render_stateful_widget(blocks_list, area, &mut list_state);
}

fn render_validators(f: &mut Frame, app: &App, area: Rect, layout: &ResponsiveLayout) {
    let theme = app.theme;
    let key_mode = layout.key_display_length();
    let val_cols = layout.validator_list_columns();

    // Use the shared sorted validator list
    let validators = app.get_sorted_validators();

    let validator_items: Vec<ListItem> = validators
        .iter()
        .map(|v| {
            let status = v.registration_status.as_deref().unwrap_or("unknown");
            let ours = if v.is_ours { "★" } else { " " };
            let key_display = key_mode.format(&v.sidechain_key);

            // Get seats from epoch data if available
            let seats_display = app
                .state
                .validator_epoch_data
                .get(&v.sidechain_key)
                .map(|epoch| format!("{:>3}", epoch.committee_seats))
                .unwrap_or_else(|| "  0".to_string());

            // Standard validator format (same for Medium and Large)
            // Fixed-width label column (5 chars for pool ticker)
            let label_display = v
                .label
                .as_ref()
                .map(|l| {
                    let truncated = if l.len() > 5 { &l[..5] } else { l.as_str() };
                    format!("{:<5}", truncated)
                })
                .unwrap_or_else(|| "     ".to_string()); // 5 spaces for unlabeled

            let mut spans = vec![
                Span::styled(ours, Style::default().fg(theme.ours())),
                Span::raw(" "),
                Span::styled(key_display, Style::default().fg(theme.secondary())),
                Span::raw(" "),
                Span::styled(label_display, Style::default().fg(theme.warning())),
                Span::raw(" "),
            ];

            if val_cols.show_status {
                spans.push(Span::styled(
                    format!("{:<12}", status),
                    Style::default().fg(if v.is_ours {
                        theme.success()
                    } else {
                        theme.muted()
                    }),
                ));
                spans.push(Span::raw("  "));
            }

            // Add seats column
            let seats_style = if app
                .state
                .validator_epoch_data
                .get(&v.sidechain_key)
                .map(|e| e.committee_seats > 0)
                .unwrap_or(false)
            {
                Style::default().fg(theme.success())
            } else {
                Style::default().fg(theme.muted())
            };
            spans.push(Span::styled(seats_display, seats_style));
            spans.push(Span::styled(" seats", Style::default().fg(theme.muted())));
            spans.push(Span::raw("  "));

            // Show epoch blocks (not total) - consistent with seats being per-epoch
            let epoch_blocks = app
                .state
                .validator_epoch_blocks
                .get(&v.sidechain_key)
                .copied()
                .unwrap_or(0);
            spans.push(Span::styled(
                format!("{:>4} blocks", epoch_blocks),
                Style::default().fg(theme.text()),
            ));

            ListItem::new(Line::from(spans))
        })
        .collect();

    let filter_text = if app.show_ours_only { " (ours)" } else { "" };
    let epoch_label = if app.state.sidechain_epoch > 0 {
        format!(", epoch {}", app.state.sidechain_epoch)
    } else {
        String::new()
    };
    let title = format!(
        "Validators ({} total{}{}) - [F] filter, j/k or ↑/↓ scroll",
        validators.len(),
        filter_text,
        epoch_label
    );

    let validators_list = List::new(validator_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border()))
                .title(Span::styled(
                    title,
                    Style::default()
                        .fg(theme.primary())
                        .add_modifier(Modifier::BOLD),
                )),
        )
        .highlight_style(
            Style::default()
                .bg(theme.highlight())
                .add_modifier(Modifier::BOLD)
                .fg(theme.text()),
        );

    let mut list_state = ListState::default();
    list_state.select(Some(app.selected_index()));
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

    let validator_items: Vec<ListItem> = validators
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let share = if total_blocks > 0 {
                (v.total_blocks as f64 / total_blocks as f64) * 100.0
            } else {
                0.0
            };

            let ours = if v.is_ours { "★" } else { " " };
            let key_display = key_mode.format(&v.sidechain_key);

            // Standard performance format (same for Medium and Large)
            let line = Line::from(vec![
                Span::styled(
                    format!("{:>3}.", i + 1),
                    Style::default().fg(theme.warning()),
                ),
                Span::raw(" "),
                Span::styled(ours, Style::default().fg(theme.ours())),
                Span::raw(" "),
                Span::styled(key_display, Style::default().fg(theme.secondary())),
                Span::raw("  "),
                Span::styled(
                    format!("{:>6} blocks", v.total_blocks),
                    Style::default().fg(theme.text()),
                ),
                Span::raw("  "),
                Span::styled(
                    format!("{:>6.3}%", share),
                    Style::default().fg(if v.is_ours {
                        theme.success()
                    } else {
                        theme.muted()
                    }),
                ),
            ]);

            ListItem::new(line)
        })
        .collect();

    let filter_text = if app.show_ours_only { " (ours)" } else { "" };
    let title = format!(
        "Performance Rankings{} - [F] filter, j/k or ↑/↓ scroll",
        filter_text
    );

    let performance_list = List::new(validator_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border()))
                .title(Span::styled(
                    title,
                    Style::default()
                        .fg(theme.primary())
                        .add_modifier(Modifier::BOLD),
                )),
        )
        .highlight_style(
            Style::default()
                .bg(theme.highlight())
                .add_modifier(Modifier::BOLD)
                .fg(theme.text()),
        );

    let mut list_state = ListState::default();
    list_state.select(Some(app.selected_index()));
    f.render_stateful_widget(performance_list, area, &mut list_state);
}

fn render_peers(f: &mut Frame, app: &App, area: Rect, layout: &ResponsiveLayout) {
    use crate::tui::app::PeerInfo;
    use ratatui::layout::{Constraint, Direction, Layout};

    let theme = app.theme;
    let key_mode = layout.key_display_length();

    // Peer health analysis
    let peer_count = app.state.connected_peers.len();
    let (health_status, health_color) = if peer_count == 0 {
        ("CRITICAL: No peers connected!", theme.error())
    } else if peer_count < 3 {
        (
            "WARNING: Very few peers - network isolation risk",
            theme.error(),
        )
    } else if peer_count < 8 {
        ("CAUTION: Low peer count", theme.warning())
    } else {
        ("Healthy", theme.success())
    };

    // Check peer diversity (inbound vs outbound balance)
    let diversity_warning = if app.state.peers_inbound == 0 && peer_count > 0 {
        Some("No inbound peers - check firewall/port forwarding")
    } else if app.state.peers_outbound == 0 && peer_count > 0 {
        Some("No outbound peers - check internet connectivity")
    } else {
        None
    };

    // Calculate synced vs unsynced peers
    let synced_peers = app
        .state
        .connected_peers
        .iter()
        .filter(|p| app.state.chain_tip.saturating_sub(p.best_number) < 10)
        .count();

    // Split area: header info + peer list
    let has_warnings = peer_count < 8 || diversity_warning.is_some();
    let header_height = if has_warnings { 4 } else { 2 };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(header_height), Constraint::Min(1)])
        .split(area);

    // Render header with health info
    let mut header_lines = vec![Line::from(vec![
        Span::styled("Status: ", Style::default().fg(theme.muted())),
        Span::styled(health_status, Style::default().fg(health_color)),
        Span::styled("    ", Style::default()),
        Span::styled("Synced: ", Style::default().fg(theme.muted())),
        Span::styled(
            format!("{}/{}", synced_peers, peer_count),
            Style::default().fg(if synced_peers == peer_count {
                theme.success()
            } else {
                theme.warning()
            }),
        ),
        Span::styled("    ", Style::default()),
        Span::styled("Balance: ", Style::default().fg(theme.muted())),
        Span::styled("↓", Style::default().fg(theme.success())),
        Span::styled(
            format!("{}", app.state.peers_inbound),
            Style::default().fg(theme.success()),
        ),
        Span::styled(" / ", Style::default().fg(theme.text())),
        Span::styled("↑", Style::default().fg(theme.muted())),
        Span::styled(
            format!("{}", app.state.peers_outbound),
            Style::default().fg(theme.muted()),
        ),
    ])];

    if let Some(warning) = diversity_warning {
        header_lines.push(Line::from(vec![
            Span::styled("⚠ ", Style::default().fg(theme.warning())),
            Span::styled(warning, Style::default().fg(theme.warning())),
        ]));
    } else if peer_count < 8 {
        header_lines.push(Line::from(vec![
            Span::styled("⚠ ", Style::default().fg(theme.warning())),
            Span::styled(
                "Consider opening port 30333 for better network connectivity",
                Style::default().fg(theme.warning()),
            ),
        ]));
    }

    let header_block = Block::default()
        .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
        .border_style(Style::default().fg(theme.border()))
        .title(Span::styled(
            "Peer Network Health",
            Style::default()
                .fg(theme.primary())
                .add_modifier(Modifier::BOLD),
        ));

    let header = Paragraph::new(header_lines).block(header_block);
    f.render_widget(header, chunks[0]);

    // Peer list
    let peer_items: Vec<ListItem> = app
        .state
        .connected_peers
        .iter()
        .map(|peer: &PeerInfo| {
            let peer_id_display = key_mode.format(&peer.peer_id);

            // Show sync status compared to our best block
            let sync_status = if peer.best_number >= app.state.chain_tip {
                ("✓", theme.success()) // Ahead or at our tip
            } else if app.state.chain_tip.saturating_sub(peer.best_number) < 10 {
                ("~", theme.warning()) // Within 10 blocks
            } else {
                ("○", theme.muted()) // Behind
            };

            // Connection direction: ↑ = outbound (we dialed), ↓ = inbound (they dialed)
            let direction = if peer.is_outbound { "↑" } else { "↓" };
            let direction_color = if peer.is_outbound {
                theme.muted()
            } else {
                theme.success()
            };

            // Format address if available
            let addr_display = peer
                .address
                .as_ref()
                .map(|a| format!("  {}", a))
                .unwrap_or_default();

            let line = Line::from(vec![
                Span::styled(sync_status.0, Style::default().fg(sync_status.1)),
                Span::styled(direction, Style::default().fg(direction_color)),
                Span::raw(" "),
                Span::styled(peer_id_display, Style::default().fg(theme.secondary())),
                Span::raw("  "),
                Span::styled(
                    format!("#{}", peer.best_number),
                    Style::default().fg(theme.block_number()),
                ),
                Span::styled(addr_display, Style::default().fg(theme.muted())),
            ]);

            ListItem::new(line)
        })
        .collect();

    // Build title with count
    let title = format!(
        "{} connected peers - j/k or ↑/↓ scroll, Enter for details",
        peer_count
    );

    let peers_list = List::new(peer_items)
        .block(
            Block::default()
                .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
                .border_style(Style::default().fg(theme.border()))
                .title(Span::styled(title, Style::default().fg(theme.muted()))),
        )
        .highlight_style(
            Style::default()
                .bg(theme.highlight())
                .add_modifier(Modifier::BOLD)
                .fg(theme.text()),
        );

    let mut list_state = ListState::default();
    list_state.select(Some(app.selected_index()));
    f.render_stateful_widget(peers_list, chunks[1], &mut list_state);
}

fn render_help(f: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme;

    // Build help items as ListItems for scrolling support
    let help_items: Vec<ListItem> = vec![
        ListItem::new(Line::from(vec![Span::styled(
            "About",
            Style::default()
                .fg(theme.title())
                .add_modifier(Modifier::BOLD),
        )])),
        ListItem::new(Line::from("")),
        ListItem::new(Line::from(vec![Span::styled(
            "  Version:",
            Style::default()
                .fg(theme.primary())
                .add_modifier(Modifier::BOLD),
        )])),
        ListItem::new(Line::from(vec![Span::styled(
            format!("    MVM v{}", env!("CARGO_PKG_VERSION")),
            Style::default().fg(theme.text()),
        )])),
        ListItem::new(Line::from("")),
        ListItem::new(Line::from(vec![Span::styled(
            "  Credits:",
            Style::default()
                .fg(theme.primary())
                .add_modifier(Modifier::BOLD),
        )])),
        ListItem::new(Line::from(vec![
            Span::styled("    Source           ", Style::default().fg(theme.text())),
            Span::raw("https://github.com/adavault/midnight-validator-monitor"),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("    License          ", Style::default().fg(theme.text())),
            Span::raw("MIT"),
        ])),
        ListItem::new(Line::from("")),
        ListItem::new(Line::from(vec![Span::styled(
            "Keyboard Shortcuts",
            Style::default()
                .fg(theme.title())
                .add_modifier(Modifier::BOLD),
        )])),
        ListItem::new(Line::from("")),
        ListItem::new(Line::from(vec![Span::styled(
            "  Navigation:",
            Style::default()
                .fg(theme.primary())
                .add_modifier(Modifier::BOLD),
        )])),
        ListItem::new(Line::from(vec![
            Span::styled("    1-5       ", Style::default().fg(theme.text())),
            Span::raw(
                "Switch to view (1=Dashboard, 2=Blocks, 3=Validators, 4=Performance, 5=Peers)",
            ),
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
        ListItem::new(Line::from(vec![Span::styled(
            "  Scrolling:",
            Style::default()
                .fg(theme.primary())
                .add_modifier(Modifier::BOLD),
        )])),
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
        ListItem::new(Line::from(vec![Span::styled(
            "  Drill-Down / Details:",
            Style::default()
                .fg(theme.primary())
                .add_modifier(Modifier::BOLD),
        )])),
        ListItem::new(Line::from(vec![
            Span::styled("    Enter     ", Style::default().fg(theme.text())),
            Span::raw("Open details (Blocks: popup, Performance: drill-down, Peers: popup)"),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("    Esc       ", Style::default().fg(theme.text())),
            Span::raw("Close popup or return from drill-down"),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("    Backspace ", Style::default().fg(theme.text())),
            Span::raw("Return from drill-down view"),
        ])),
        ListItem::new(Line::from("")),
        ListItem::new(Line::from(vec![Span::styled(
            "  Options:",
            Style::default()
                .fg(theme.primary())
                .add_modifier(Modifier::BOLD),
        )])),
        ListItem::new(Line::from(vec![
            Span::styled("    f / F     ", Style::default().fg(theme.text())),
            Span::raw("Toggle 'ours only' filter (Validators/Performance views)"),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("    t / T     ", Style::default().fg(theme.text())),
            Span::raw("Toggle theme (Midnight ⟷ Midday)"),
        ])),
        ListItem::new(Line::from("")),
        ListItem::new(Line::from(vec![Span::styled(
            "  Quit:",
            Style::default()
                .fg(theme.primary())
                .add_modifier(Modifier::BOLD),
        )])),
        ListItem::new(Line::from(vec![
            Span::styled("    q / Esc   ", Style::default().fg(theme.text())),
            Span::raw("Quit application"),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("    Ctrl+C    ", Style::default().fg(theme.text())),
            Span::raw("Quit application"),
        ])),
        ListItem::new(Line::from("")),
        ListItem::new(Line::from(vec![Span::styled(
            "  Views:",
            Style::default()
                .fg(theme.primary())
                .add_modifier(Modifier::BOLD),
        )])),
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
        ListItem::new(Line::from(vec![Span::styled(
            "Reference",
            Style::default()
                .fg(theme.title())
                .add_modifier(Modifier::BOLD),
        )])),
        ListItem::new(Line::from("")),
        ListItem::new(Line::from(vec![Span::styled(
            "  Symbols:",
            Style::default()
                .fg(theme.primary())
                .add_modifier(Modifier::BOLD),
        )])),
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
        ListItem::new(Line::from(vec![Span::styled(
            "  Dashboard Fields:",
            Style::default()
                .fg(theme.primary())
                .add_modifier(Modifier::BOLD),
        )])),
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
        ListItem::new(Line::from("")),
        ListItem::new(Line::from(vec![Span::styled(
            "  Glossary:",
            Style::default()
                .fg(theme.primary())
                .add_modifier(Modifier::BOLD),
        )])),
        ListItem::new(Line::from(vec![
            Span::styled("    Extrinsics       ", Style::default().fg(theme.text())),
            Span::raw("Transactions or calls submitted to the blockchain"),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("    Sidechain Epoch  ", Style::default().fg(theme.text())),
            Span::raw("Committee rotation period (2h preview, TBD mainnet)"),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("    Mainchain Epoch  ", Style::default().fg(theme.text())),
            Span::raw("Cardano epoch alignment (24h preview, 5d mainnet)"),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("    Committee        ", Style::default().fg(theme.text())),
            Span::raw("Validators selected for block production each epoch"),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("    Seats            ", Style::default().fg(theme.text())),
            Span::raw("Weighted positions in the committee (stake-based)"),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("    AURA             ", Style::default().fg(theme.text())),
            Span::raw("Block authoring consensus mechanism"),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("    Grandpa          ", Style::default().fg(theme.text())),
            Span::raw("Block finalization protocol (GHOST-based)"),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("    Finalized        ", Style::default().fg(theme.text())),
            Span::raw("Irreversible blocks confirmed by 2/3+ validators"),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("    Slot             ", Style::default().fg(theme.text())),
            Span::raw("6-second time window for block production"),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("    State Pruning    ", Style::default().fg(theme.text())),
            Span::raw("Removal of old blockchain state to save disk space"),
        ])),
    ];

    let item_count = help_items.len();

    let help_list = List::new(help_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border()))
                .title(Span::styled(
                    "Help - Use j/k/J/K or ↑/↓ to scroll",
                    Style::default()
                        .fg(theme.primary())
                        .add_modifier(Modifier::BOLD),
                )),
        )
        .highlight_style(
            Style::default()
                .bg(theme.highlight())
                .add_modifier(Modifier::BOLD)
                .fg(theme.text()),
        );

    let mut list_state = ListState::default();
    list_state.select(Some(app.selected_index()));
    f.render_stateful_widget(help_list, area, &mut list_state);

    // Render scrollbar
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));

    let mut scrollbar_state = ScrollbarState::new(item_count).position(app.selected_index());

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

/// Format countdown seconds into HH:MM:SS format (always includes hours for alignment)
fn format_countdown(secs: u64) -> String {
    let hours = secs / 3600;
    let mins = (secs % 3600) / 60;
    let s = secs % 60;

    format!("{:02}:{:02}:{:02}", hours, mins, s)
}

/// Format timestamp as human-readable date/time
fn format_timestamp(timestamp: i64) -> String {
    use chrono::{TimeZone, Utc};
    if let Some(dt) = Utc.timestamp_opt(timestamp, 0).single() {
        dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
    } else {
        "Unknown".to_string()
    }
}

// ========================================
// Popup Rendering
// ========================================

use crate::tui::PopupContent;

/// Render popup overlay
fn render_popup(f: &mut Frame, app: &App, popup: &PopupContent) {
    match popup {
        PopupContent::BlockDetail { block } => render_block_detail_popup(f, app, block),
        PopupContent::PeerDetail { peer } => render_peer_detail_popup(f, app, peer),
        PopupContent::ValidatorDetail {
            validator,
            epoch_history,
            scroll_index,
        } => {
            render_validator_detail_popup(f, app, validator, epoch_history, *scroll_index);
        }
        PopupContent::ValidatorIdentity {
            validator,
            aura_key,
            current_epoch_seats,
            committee_size,
            blocks_this_epoch,
            stake_display,
            selection_stats,
        } => {
            render_validator_identity_popup(
                f,
                app,
                validator,
                aura_key.as_deref(),
                *current_epoch_seats,
                *committee_size,
                *blocks_this_epoch,
                stake_display.as_deref(),
                selection_stats.as_ref(),
            );
        }
    }
}

/// Calculate centered popup area with adaptive sizing
/// On narrow screens (<100 cols), uses nearly full width with small margins
/// On wider screens, uses percentage-based sizing
fn centered_popup(min_width: u16, percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    use ratatui::layout::{Constraint, Direction, Layout};

    // Vertical centering (always percentage-based)
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    // Horizontal: use min_width with margins on narrow screens, percentage on wide
    let percent_width = (area.width as u32 * percent_x as u32 / 100) as u16;
    let popup_width = percent_width
        .max(min_width)
        .min(area.width.saturating_sub(4));
    let margin = area.width.saturating_sub(popup_width) / 2;

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(margin),
            Constraint::Length(popup_width),
            Constraint::Length(margin),
        ])
        .split(popup_layout[1])[1]
}

/// Render block detail popup
fn render_block_detail_popup(f: &mut Frame, app: &App, block: &crate::db::BlockRecord) {
    use ratatui::widgets::Clear;

    let theme = app.theme;
    let area = centered_popup(90, 60, 65, f.area()); // min 90 cols for hash lines

    // Clear the area behind the popup
    f.render_widget(Clear, area);

    // Format author key with label if known
    let author_display = if let Some(ref key) = block.author_key {
        // Check if this validator has a label
        let label = app
            .state
            .validators
            .iter()
            .find(|v| v.sidechain_key == *key)
            .and_then(|v| v.label.as_ref())
            .map(|l| format!(" ({})", l))
            .unwrap_or_default();
        format!("{}{}", key, label)
    } else {
        "Unknown".to_string()
    };

    // Build popup content
    let content = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(" Block Number:     ", Style::default().fg(theme.muted())),
            Span::styled(
                format!("#{}", block.block_number),
                Style::default()
                    .fg(theme.block_number())
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(" Block Hash:       ", Style::default().fg(theme.muted())),
            Span::styled(&block.block_hash, Style::default().fg(theme.secondary())),
        ]),
        Line::from(vec![
            Span::styled(" Parent Hash:      ", Style::default().fg(theme.muted())),
            Span::styled(&block.parent_hash, Style::default().fg(theme.text())),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" State Root:       ", Style::default().fg(theme.muted())),
            Span::styled(&block.state_root, Style::default().fg(theme.text())),
        ]),
        Line::from(vec![
            Span::styled(" Extrinsics Root:  ", Style::default().fg(theme.muted())),
            Span::styled(&block.extrinsics_root, Style::default().fg(theme.text())),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Slot Number:      ", Style::default().fg(theme.muted())),
            Span::styled(
                format!("{}", block.slot_number),
                Style::default().fg(theme.text()),
            ),
        ]),
        Line::from(vec![
            Span::styled(" Sidechain Epoch:  ", Style::default().fg(theme.muted())),
            Span::styled(
                format!("{}", block.sidechain_epoch),
                Style::default().fg(theme.epoch()),
            ),
        ]),
        Line::from(vec![
            Span::styled(" Mainchain Epoch:  ", Style::default().fg(theme.muted())),
            Span::styled(
                format!("{}", block.epoch),
                Style::default().fg(theme.epoch()),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Timestamp:        ", Style::default().fg(theme.muted())),
            Span::styled(
                format_timestamp(block.timestamp),
                Style::default().fg(theme.text()),
            ),
        ]),
        Line::from(vec![
            Span::styled(" Author:           ", Style::default().fg(theme.muted())),
            Span::styled(author_display, Style::default().fg(theme.secondary())),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Extrinsics:       ", Style::default().fg(theme.muted())),
            Span::styled(
                format!("{}", block.extrinsics_count),
                Style::default().fg(theme.text()),
            ),
        ]),
        Line::from(vec![
            Span::styled(" Finalized:        ", Style::default().fg(theme.muted())),
            Span::styled(
                if block.is_finalized { "Yes ✓" } else { "No" },
                Style::default().fg(if block.is_finalized {
                    theme.success()
                } else {
                    theme.warning()
                }),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Press ", Style::default().fg(theme.muted())),
            Span::styled("Esc", Style::default().fg(theme.primary())),
            Span::styled(" to close", Style::default().fg(theme.muted())),
        ]),
    ];

    let popup = Paragraph::new(content).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.primary()))
            .title(Span::styled(
                format!(" Block #{} ", block.block_number),
                Style::default()
                    .fg(theme.title())
                    .add_modifier(Modifier::BOLD),
            )),
    );

    f.render_widget(popup, area);
}

/// Render peer detail popup
fn render_peer_detail_popup(f: &mut Frame, app: &App, peer: &crate::tui::app::PeerInfo) {
    use ratatui::widgets::Clear;

    let theme = app.theme;
    let area = centered_popup(76, 60, 50, f.area()); // min 76 cols for peer IDs

    // Clear the area behind the popup
    f.render_widget(Clear, area);

    // Determine sync status
    let (sync_status, sync_color) = if peer.best_number >= app.state.chain_tip {
        ("Synced", theme.success())
    } else if app.state.chain_tip.saturating_sub(peer.best_number) < 10 {
        ("Nearly synced", theme.warning())
    } else {
        ("Behind", theme.muted())
    };

    let blocks_behind = app.state.chain_tip.saturating_sub(peer.best_number);

    // Build popup content
    let address_display = peer.address.as_deref().unwrap_or("Unknown");

    let content = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(" Peer ID:          ", Style::default().fg(theme.muted())),
            Span::styled(&peer.peer_id, Style::default().fg(theme.secondary())),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Remote Address:   ", Style::default().fg(theme.muted())),
            Span::styled(address_display, Style::default().fg(theme.text())),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Best Block:       ", Style::default().fg(theme.muted())),
            Span::styled(
                format!("#{}", peer.best_number),
                Style::default().fg(theme.block_number()),
            ),
        ]),
        Line::from(vec![
            Span::styled(" Best Hash:        ", Style::default().fg(theme.muted())),
            Span::styled(&peer.best_hash, Style::default().fg(theme.text())),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Connection:       ", Style::default().fg(theme.muted())),
            Span::styled(
                if peer.is_outbound {
                    "Outbound (we dialed)"
                } else {
                    "Inbound (they dialed)"
                },
                Style::default().fg(if peer.is_outbound {
                    theme.muted()
                } else {
                    theme.success()
                }),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Sync Status:      ", Style::default().fg(theme.muted())),
            Span::styled(sync_status, Style::default().fg(sync_color)),
            if blocks_behind > 0 {
                Span::styled(
                    format!(" ({} blocks behind)", blocks_behind),
                    Style::default().fg(theme.muted()),
                )
            } else {
                Span::raw("")
            },
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Press ", Style::default().fg(theme.muted())),
            Span::styled("Esc", Style::default().fg(theme.primary())),
            Span::styled(" to close", Style::default().fg(theme.muted())),
        ]),
    ];

    let popup = Paragraph::new(content).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.primary()))
            .title(Span::styled(
                " Peer Details ",
                Style::default()
                    .fg(theme.title())
                    .add_modifier(Modifier::BOLD),
            )),
    );

    f.render_widget(popup, area);
}

/// Render validator identity card popup
#[allow(clippy::too_many_arguments)]
fn render_validator_identity_popup(
    f: &mut Frame,
    app: &App,
    validator: &crate::db::ValidatorRecord,
    aura_key: Option<&str>,
    current_epoch_seats: u32,
    committee_size: u32,
    blocks_this_epoch: u64,
    stake_display: Option<&str>,
    selection_stats: Option<&CommitteeSelectionStats>,
) {
    use ratatui::widgets::Clear;

    let theme = app.theme;
    let area = centered_popup(76, 60, 75, f.area());

    // Clear the area behind the popup
    f.render_widget(Clear, area);

    let ours_marker = if validator.is_ours { "★ " } else { "" };
    let label_display = validator
        .label
        .as_ref()
        .map(|l| format!(" ({})", l))
        .unwrap_or_default();

    // Registration status
    let reg_status = validator
        .registration_status
        .as_deref()
        .unwrap_or("unknown");
    let is_permissioned = reg_status.to_lowercase().contains("permissioned");
    let is_registered = reg_status.to_lowercase().contains("registered") || is_permissioned;

    // Committee percentage
    let committee_pct = if committee_size > 0 {
        (current_epoch_seats as f64 / committee_size as f64) * 100.0
    } else {
        0.0
    };

    // Expected blocks this epoch (rough estimate)
    // Slots per epoch varies by network (1200 preview, 6000 mainnet)
    let slots_per_epoch = app.chain_timing.blocks_per_sidechain_epoch() as f64;
    let expected_blocks = if committee_size > 0 {
        (current_epoch_seats as f64 / committee_size as f64) * slots_per_epoch
    } else {
        0.0
    };

    // Build content
    let mut content = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!(" {}", ours_marker),
                Style::default().fg(theme.ours()),
            ),
            Span::styled(
                &validator.sidechain_key,
                Style::default()
                    .fg(theme.secondary())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(&label_display, Style::default().fg(theme.muted())),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Sidechain Key:  ", Style::default().fg(theme.muted())),
            Span::styled(&validator.sidechain_key, Style::default().fg(theme.text())),
        ]),
    ];

    // Add AURA key if available
    if let Some(aura) = aura_key {
        content.push(Line::from(vec![
            Span::styled(" AURA Key:       ", Style::default().fg(theme.muted())),
            Span::styled(aura, Style::default().fg(theme.text())),
        ]));
    }

    content.push(Line::from(""));

    // Registration section
    content.push(Line::from(vec![
        Span::styled(" Registration:   ", Style::default().fg(theme.muted())),
        Span::styled(
            reg_status,
            Style::default().fg(if is_permissioned {
                theme.warning()
            } else if is_registered {
                theme.success()
            } else {
                theme.muted()
            }),
        ),
    ]));

    // Stake if available
    if let Some(stake) = stake_display {
        content.push(Line::from(vec![
            Span::styled(" Stake:          ", Style::default().fg(theme.muted())),
            Span::styled(stake, Style::default().fg(theme.success())),
        ]));
    }

    content.push(Line::from(""));

    // Current epoch info
    content.push(Line::from(vec![
        Span::styled(" Seats:          ", Style::default().fg(theme.muted())),
        Span::styled(
            format!("{}", current_epoch_seats),
            Style::default().fg(if current_epoch_seats > 0 {
                theme.success()
            } else {
                theme.muted()
            }),
        ),
        Span::styled(
            format!(" / {} ({:.2}%)", committee_size, committee_pct),
            Style::default().fg(theme.muted()),
        ),
    ]));

    content.push(Line::from(vec![
        Span::styled(" Blocks:         ", Style::default().fg(theme.muted())),
        Span::styled(
            format!("{} produced", blocks_this_epoch),
            Style::default().fg(theme.text()),
        ),
        Span::styled(
            format!(" / {:.0} expected", expected_blocks),
            Style::default().fg(theme.muted()),
        ),
    ]));

    content.push(Line::from(vec![
        Span::styled(" Total Blocks:   ", Style::default().fg(theme.muted())),
        Span::styled(
            format!("{}", validator.total_blocks),
            Style::default().fg(theme.success()),
        ),
        Span::styled(" (all time)", Style::default().fg(theme.muted())),
    ]));

    // Committee selection statistics (if available)
    if let Some(stats) = selection_stats {
        content.push(Line::from(""));
        content.push(Line::from(vec![Span::styled(
            " ── Selection History ──",
            Style::default()
                .fg(theme.secondary())
                .add_modifier(Modifier::BOLD),
        )]));
        content.push(Line::from(""));

        // Selection rate
        content.push(Line::from(vec![
            Span::styled(" Selected:       ", Style::default().fg(theme.muted())),
            Span::styled(
                stats.selection_rate_display(),
                Style::default().fg(if stats.times_selected > 0 {
                    theme.success()
                } else {
                    theme.warning()
                }),
            ),
        ]));

        // Total seats received
        content.push(Line::from(vec![
            Span::styled(" Total Seats:    ", Style::default().fg(theme.muted())),
            Span::styled(
                format!("{}", stats.total_seats),
                Style::default().fg(theme.text()),
            ),
            if let Some(avg) = stats.avg_seats_when_selected() {
                Span::styled(
                    format!(" ({:.1} avg when selected)", avg),
                    Style::default().fg(theme.muted()),
                )
            } else {
                Span::raw("")
            },
        ]));

        // Average epochs between selections
        if let Some(avg_gap) = stats.avg_epochs_between_selections() {
            content.push(Line::from(vec![
                Span::styled(" Avg Gap:        ", Style::default().fg(theme.muted())),
                Span::styled(
                    format!("{:.1} epochs between selections", avg_gap),
                    Style::default().fg(theme.text()),
                ),
            ]));
        }

        // Last selected
        if let Some(last_epoch) = stats.last_selected_epoch {
            let epochs_ago = stats.epochs_since_selection().unwrap_or(0);
            content.push(Line::from(vec![
                Span::styled(" Last Selected:  ", Style::default().fg(theme.muted())),
                Span::styled(
                    format!("Epoch {}", last_epoch),
                    Style::default().fg(theme.text()),
                ),
                Span::styled(
                    if epochs_ago == 0 {
                        " (current)".to_string()
                    } else {
                        format!(" ({} epochs ago)", epochs_ago)
                    },
                    Style::default().fg(theme.muted()),
                ),
            ]));
        }

        // Current status
        content.push(Line::from(vec![
            Span::styled(" Status:         ", Style::default().fg(theme.muted())),
            Span::styled(
                if stats.currently_in_committee {
                    "In Committee"
                } else {
                    "Not Selected"
                },
                Style::default().fg(if stats.currently_in_committee {
                    theme.success()
                } else {
                    theme.warning()
                }),
            ),
        ]));

        // Stake rank (for dynamic validators)
        if let Some(rank) = stats.stake_rank {
            content.push(Line::from(""));
            content.push(Line::from(vec![
                Span::styled(" Stake Rank:     ", Style::default().fg(theme.muted())),
                Span::styled(
                    format!(
                        "#{} of {} dynamic validators",
                        rank, stats.total_dynamic_validators
                    ),
                    Style::default().fg(theme.text()),
                ),
            ]));
            if let Some(share) = stats.stake_share_percent {
                content.push(Line::from(vec![
                    Span::styled(" Stake Share:    ", Style::default().fg(theme.muted())),
                    Span::styled(
                        format!("{:.2}% of dynamic pool", share),
                        Style::default().fg(theme.text()),
                    ),
                ]));
            }
        }

        // Committee structure note
        if stats.permissioned_seats_percent > 0.0 {
            content.push(Line::from(vec![
                Span::styled(" Committee:      ", Style::default().fg(theme.muted())),
                Span::styled(
                    format!(
                        "~{:.0}% permissioned, ~{:.0}% dynamic",
                        stats.permissioned_seats_percent,
                        100.0 - stats.permissioned_seats_percent
                    ),
                    Style::default().fg(theme.muted()),
                ),
            ]));
        }
    }

    content.push(Line::from(""));
    content.push(Line::from(vec![
        Span::styled(" Press ", Style::default().fg(theme.muted())),
        Span::styled("Esc", Style::default().fg(theme.primary())),
        Span::styled(" to close", Style::default().fg(theme.muted())),
    ]));

    let title = if validator.is_ours {
        " ★ Validator Identity "
    } else {
        " Validator Identity "
    };

    let popup = Paragraph::new(content).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.primary()))
            .title(Span::styled(
                title,
                Style::default()
                    .fg(theme.title())
                    .add_modifier(Modifier::BOLD),
            )),
    );

    f.render_widget(popup, area);
}

/// Render validator detail popup with epoch history table
fn render_validator_detail_popup(
    f: &mut Frame,
    app: &App,
    validator: &crate::db::ValidatorRecord,
    epoch_history: &[crate::db::ValidatorEpochHistoryRecord],
    scroll_index: usize,
) {
    use ratatui::layout::{Constraint, Direction, Layout};
    use ratatui::widgets::Clear;

    let theme = app.theme;
    let area = centered_popup(76, 70, 80, f.area()); // 80% height for table

    // Clear the area behind the popup
    f.render_widget(Clear, area);

    // Split into header and table areas
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Min(0)])
        .split(area);

    // Header with validator info
    let label = validator
        .label
        .as_ref()
        .map(|l| format!(" ({})", l))
        .unwrap_or_default();
    let status = validator
        .registration_status
        .as_deref()
        .unwrap_or("unknown");
    let ours = if validator.is_ours { "★ " } else { "" };

    // Show full key if it fits, otherwise truncate
    let available_width = chunks[0].width.saturating_sub(16) as usize;
    let key_display = if validator.sidechain_key.len() <= available_width {
        validator.sidechain_key.clone()
    } else {
        format!(
            "{}...{}",
            &validator.sidechain_key[..10],
            &validator.sidechain_key[validator.sidechain_key.len() - 8..]
        )
    };

    let header_content = vec![
        Line::from(vec![
            Span::styled(ours, Style::default().fg(theme.ours())),
            Span::styled(
                key_display,
                Style::default()
                    .fg(theme.secondary())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(label, Style::default().fg(theme.muted())),
        ]),
        Line::from(vec![
            Span::styled("Status: ", Style::default().fg(theme.muted())),
            Span::styled(
                status,
                Style::default().fg(if validator.is_ours {
                    theme.success()
                } else {
                    theme.text()
                }),
            ),
            Span::styled("  |  Total Blocks: ", Style::default().fg(theme.muted())),
            Span::styled(
                format!("{}", validator.total_blocks),
                Style::default().fg(theme.success()),
            ),
            Span::styled("  |  ", Style::default().fg(theme.muted())),
            Span::styled("j/k", Style::default().fg(theme.primary())),
            Span::styled(" scroll  ", Style::default().fg(theme.muted())),
            Span::styled("Esc", Style::default().fg(theme.primary())),
            Span::styled(" close", Style::default().fg(theme.muted())),
        ]),
    ];

    let header_widget = Paragraph::new(header_content).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.primary()))
            .title(Span::styled(
                " Validator Detail ",
                Style::default()
                    .fg(theme.title())
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(header_widget, chunks[0]);

    // Build table rows
    // Slots per epoch varies by network (1200 preview, 6000 mainnet)
    let slots_per_epoch = app.chain_timing.blocks_per_sidechain_epoch() as f64;
    let rows: Vec<Row> = epoch_history
        .iter()
        .map(|record| {
            let expected = if record.committee_size > 0 {
                (record.seats as f64 / record.committee_size as f64) * slots_per_epoch
            } else {
                0.0
            };

            let ratio = if expected > 0.0 {
                (record.blocks_produced as f64 / expected) * 100.0
            } else if record.seats == 0 {
                100.0
            } else {
                0.0
            };

            let ratio_color = if ratio >= 90.0 {
                theme.success()
            } else if ratio >= 70.0 {
                theme.warning()
            } else if record.seats == 0 {
                theme.muted()
            } else {
                theme.error()
            };

            let seats_color = if record.seats > 0 {
                theme.success()
            } else {
                theme.muted()
            };

            Row::new(vec![
                Cell::from(format!("{:>6}", record.epoch))
                    .style(Style::default().fg(theme.epoch())),
                Cell::from(format!("{:>5}", record.seats)).style(Style::default().fg(seats_color)),
                Cell::from(format!("{:>6}", record.blocks_produced))
                    .style(Style::default().fg(theme.text())),
                Cell::from(format!("{:>6.0}", expected)).style(Style::default().fg(theme.muted())),
                Cell::from(format!("{:>6.1}%", ratio)).style(Style::default().fg(ratio_color)),
            ])
        })
        .collect();

    // Summary stats
    let total_epochs = epoch_history.len();
    let total_blocks: u64 = epoch_history.iter().map(|r| r.blocks_produced).sum();
    let epochs_with_seats = epoch_history.iter().filter(|r| r.seats > 0).count();

    let title = format!(
        " Epoch History ({} epochs, {} with seats, {} blocks) ",
        total_epochs, epochs_with_seats, total_blocks
    );

    // Table header
    let header_style = Style::default()
        .fg(theme.primary())
        .add_modifier(Modifier::BOLD);
    let header = Row::new(vec![
        Cell::from(format!("{:>6}", "Epoch")).style(header_style),
        Cell::from(format!("{:>5}", "Seats")).style(header_style),
        Cell::from(format!("{:>6}", "Blocks")).style(header_style),
        Cell::from(format!("{:>8}", "Expected")).style(header_style),
        Cell::from(format!("{:>7}", "Ratio")).style(header_style),
    ])
    .height(1)
    .bottom_margin(0);

    let widths = [
        Constraint::Length(8),
        Constraint::Length(7),
        Constraint::Length(8),
        Constraint::Length(10),
        Constraint::Length(9),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border()))
                .title(Span::styled(
                    title,
                    Style::default()
                        .fg(theme.primary())
                        .add_modifier(Modifier::BOLD),
                )),
        )
        .row_highlight_style(
            Style::default()
                .bg(theme.highlight())
                .add_modifier(Modifier::BOLD)
                .fg(theme.text()),
        )
        .highlight_symbol(" › ");

    let mut table_state = TableState::default();
    table_state.select(Some(scroll_index));
    f.render_stateful_widget(table, chunks[1], &mut table_state);
}

// ========================================
// Validator Epoch Detail View (Legacy - kept for reference)
// ========================================

#[allow(dead_code)]
fn render_validator_epoch_detail(f: &mut Frame, app: &App, area: Rect, layout: &ResponsiveLayout) {
    use ratatui::layout::{Constraint, Direction, Layout};

    let theme = app.theme;
    let key_mode = layout.key_display_length();

    // Split into header and table areas
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(6), Constraint::Min(0)])
        .split(area);

    // Render header with validator info
    let header_content = if let Some(ref validator) = app.drill_down_validator {
        // Show full key if it fits (66 chars + label overhead), otherwise truncate
        let available_width = chunks[0].width.saturating_sub(16) as usize; // Account for "Validator: " and borders
        let key_display = if validator.sidechain_key.len() <= available_width {
            validator.sidechain_key.clone()
        } else {
            key_mode.format(&validator.sidechain_key)
        };
        let label = validator
            .label
            .as_ref()
            .map(|l| format!(" ({})", l))
            .unwrap_or_default();
        let status = validator
            .registration_status
            .as_deref()
            .unwrap_or("unknown");
        let ours = if validator.is_ours { "★ " } else { "" };

        vec![
            Line::from(vec![
                Span::styled(ours, Style::default().fg(theme.ours())),
                Span::styled("Validator: ", Style::default().fg(theme.muted())),
                Span::styled(
                    key_display,
                    Style::default()
                        .fg(theme.secondary())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(label, Style::default().fg(theme.muted())),
            ]),
            Line::from(vec![
                Span::styled("  Status: ", Style::default().fg(theme.muted())),
                Span::styled(
                    status,
                    Style::default().fg(if validator.is_ours {
                        theme.success()
                    } else {
                        theme.text()
                    }),
                ),
                Span::styled("  |  Total Blocks: ", Style::default().fg(theme.muted())),
                Span::styled(
                    format!("{}", validator.total_blocks),
                    Style::default().fg(theme.success()),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Press ", Style::default().fg(theme.muted())),
                Span::styled("Esc", Style::default().fg(theme.primary())),
                Span::styled(" or ", Style::default().fg(theme.muted())),
                Span::styled("Backspace", Style::default().fg(theme.primary())),
                Span::styled(" to return", Style::default().fg(theme.muted())),
            ]),
        ]
    } else {
        vec![Line::from(Span::styled(
            "Loading...",
            Style::default().fg(theme.muted()),
        ))]
    };

    let header_widget = Paragraph::new(header_content).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border()))
            .title(Span::styled(
                "Validator Detail",
                Style::default()
                    .fg(theme.primary())
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(header_widget, chunks[0]);

    // Build table rows
    // Slots per epoch varies by network (1200 preview, 6000 mainnet)
    let slots_per_epoch = app.chain_timing.blocks_per_sidechain_epoch() as f64;
    let rows: Vec<Row> = app
        .validator_epoch_history
        .iter()
        .map(|record| {
            // Calculate expected blocks: (seats / committee_size) * slots_per_epoch
            let expected = if record.committee_size > 0 {
                (record.seats as f64 / record.committee_size as f64) * slots_per_epoch
            } else {
                0.0
            };

            let ratio = if expected > 0.0 {
                (record.blocks_produced as f64 / expected) * 100.0
            } else if record.seats == 0 {
                100.0 // No seats = no expected blocks = 100%
            } else {
                0.0
            };

            let ratio_color = if ratio >= 90.0 {
                theme.success()
            } else if ratio >= 70.0 {
                theme.warning()
            } else if record.seats == 0 {
                theme.muted()
            } else {
                theme.error()
            };

            let seats_color = if record.seats > 0 {
                theme.success()
            } else {
                theme.muted()
            };

            Row::new(vec![
                Cell::from(format!("{:>6}", record.epoch))
                    .style(Style::default().fg(theme.epoch())),
                Cell::from(format!("{:>5}", record.seats)).style(Style::default().fg(seats_color)),
                Cell::from(format!("{:>6}", record.blocks_produced))
                    .style(Style::default().fg(theme.text())),
                Cell::from(format!("{:>6.0}", expected)).style(Style::default().fg(theme.muted())),
                Cell::from(format!("{:>6.1}%", ratio)).style(Style::default().fg(ratio_color)),
            ])
        })
        .collect();

    // Calculate summary stats
    let total_epochs = app.validator_epoch_history.len();
    let total_blocks: u64 = app
        .validator_epoch_history
        .iter()
        .map(|r| r.blocks_produced)
        .sum();
    let epochs_with_seats = app
        .validator_epoch_history
        .iter()
        .filter(|r| r.seats > 0)
        .count();

    let title = format!(
        "Epoch History ({} epochs, {} with seats, {} total blocks)",
        total_epochs, epochs_with_seats, total_blocks
    );

    // Table header - right-aligned to match data columns
    let header_style = Style::default()
        .fg(theme.primary())
        .add_modifier(Modifier::BOLD);
    let header = Row::new(vec![
        Cell::from(format!("{:>6}", "Epoch")).style(header_style),
        Cell::from(format!("{:>5}", "Seats")).style(header_style),
        Cell::from(format!("{:>6}", "Blocks")).style(header_style),
        Cell::from(format!("{:>8}", "Expected")).style(header_style),
        Cell::from(format!("{:>7}", "Ratio")).style(header_style),
    ])
    .height(1)
    .bottom_margin(0);

    // Column widths - match header format widths
    let widths = [
        Constraint::Length(8),  // Epoch (6 + padding)
        Constraint::Length(7),  // Seats (5 + padding)
        Constraint::Length(8),  // Blocks (6 + padding)
        Constraint::Length(10), // Expected (8 + padding)
        Constraint::Length(9),  // Ratio (7 + padding)
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border()))
                .title(Span::styled(
                    title,
                    Style::default()
                        .fg(theme.primary())
                        .add_modifier(Modifier::BOLD),
                )),
        )
        .row_highlight_style(
            Style::default()
                .bg(theme.highlight())
                .add_modifier(Modifier::BOLD)
                .fg(theme.text()),
        )
        .highlight_symbol(" › ");

    let mut table_state = TableState::default();
    table_state.select(Some(app.selected_index()));
    f.render_stateful_widget(table, chunks[1], &mut table_state);

    // Render scrollbar
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));

    let mut scrollbar_state =
        ScrollbarState::new(app.validator_epoch_history.len()).position(app.selected_index());

    f.render_stateful_widget(scrollbar, chunks[1], &mut scrollbar_state);
}
