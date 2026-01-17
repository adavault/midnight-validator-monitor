//! UI rendering for TUI

use crate::tui::layout::ResponsiveLayout;
use crate::tui::{App, ScreenSize, ViewMode};
use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

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
        ViewMode::Help => render_help(f, app, chunks[1]),
    }

    // Render status bar (compact for small screens)
    render_status_bar(f, app, chunks[2], &layout);
}

fn render_title_bar(f: &mut Frame, app: &App, area: Rect, layout: &ResponsiveLayout) {
    let theme = app.theme;

    let title = match layout.size {
        ScreenSize::Small => {
            // Compact title for small screens - no borders, minimal info
            vec![
                Span::styled("MVM", Style::default().fg(theme.title()).add_modifier(Modifier::BOLD)),
                Span::raw(" "),
                Span::styled(
                    match app.view_mode {
                        ViewMode::Dashboard => "Dash",
                        ViewMode::Blocks => "Blks",
                        ViewMode::Validators => "Vals",
                        ViewMode::Performance => "Perf",
                        ViewMode::Help => "Help",
                    },
                    Style::default().fg(theme.primary()).add_modifier(Modifier::BOLD),
                ),
            ]
        }
        _ => {
            // Standard title bar
            vec![
                Span::styled("Midnight Validator Monitor", Style::default().fg(theme.title()).add_modifier(Modifier::BOLD)),
                Span::styled(" v0.4.0-beta", Style::default().fg(theme.muted())),
                Span::raw("  |  "),
                Span::styled(
                    match app.view_mode {
                        ViewMode::Dashboard => "[1] Dashboard",
                        ViewMode::Blocks => "[2] Blocks",
                        ViewMode::Validators => "[3] Validators",
                        ViewMode::Performance => "[4] Performance",
                        ViewMode::Help => "[?] Help",
                    },
                    Style::default().fg(theme.primary()).add_modifier(Modifier::BOLD),
                ),
                Span::raw("  |  "),
                Span::styled(theme.name(), Style::default().fg(theme.secondary())),
            ]
        }
    };

    let title_paragraph = if layout.size == ScreenSize::Small {
        // No borders for small screens
        Paragraph::new(Line::from(title))
            .style(Style::default().bg(theme.border()))
            .alignment(Alignment::Center)
    } else {
        Paragraph::new(Line::from(title))
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(theme.border())))
            .alignment(Alignment::Left)
    };

    f.render_widget(title_paragraph, area);
}

fn render_status_bar(f: &mut Frame, app: &App, area: Rect, layout: &ResponsiveLayout) {
    let theme = app.theme;
    let since_update = app.last_update.elapsed().as_secs();

    let status_text = if let Some(ref err) = app.state.last_error {
        vec![
            Span::styled("ERR: ", Style::default().fg(theme.error()).add_modifier(Modifier::BOLD)),
            Span::styled(
                if layout.size == ScreenSize::Small && err.len() > 30 {
                    format!("{}...", &err[..27])
                } else {
                    err.clone()
                },
                Style::default().fg(theme.error())
            ),
        ]
    } else {
        match layout.size {
            ScreenSize::Small => {
                // Minimal status for small screens
                vec![
                    Span::styled("●", Style::default().fg(theme.success())),
                    Span::styled(format!(" {}s ", since_update), Style::default().fg(theme.text())),
                    Span::styled("Q", Style::default().fg(theme.primary())),
                    Span::styled(":quit ", Style::default().fg(theme.muted())),
                    Span::styled("?", Style::default().fg(theme.primary())),
                    Span::styled(":help", Style::default().fg(theme.muted())),
                ]
            }
            _ => {
                // Standard status bar
                vec![
                    Span::styled("●", Style::default().fg(theme.success())),
                    Span::styled(format!(" Synced  |  Updated {}s ago  |  ", since_update), Style::default().fg(theme.text())),
                    Span::styled("[Q]", Style::default().fg(theme.primary())),
                    Span::styled(" Quit  ", Style::default().fg(theme.muted())),
                    Span::styled("[1-4]", Style::default().fg(theme.primary())),
                    Span::styled(" Views  ", Style::default().fg(theme.muted())),
                    Span::styled("[T]", Style::default().fg(theme.primary())),
                    Span::styled(" Theme  ", Style::default().fg(theme.muted())),
                    Span::styled("[?]", Style::default().fg(theme.primary())),
                    Span::styled(" Help", Style::default().fg(theme.muted())),
                ]
            }
        }
    };

    let status_paragraph = if layout.size == ScreenSize::Small {
        Paragraph::new(Line::from(status_text))
            .style(Style::default().bg(theme.border()))
            .alignment(Alignment::Center)
    } else {
        Paragraph::new(Line::from(status_text))
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(theme.border())))
            .alignment(Alignment::Left)
    };

    f.render_widget(status_paragraph, area);
}

fn render_dashboard(f: &mut Frame, app: &App, area: Rect, layout: &ResponsiveLayout) {
    let theme = app.theme;
    let chunks = layout.dashboard_layout(area);
    let key_mode = layout.key_display_length();

    // Network status - responsive based on screen size with epoch progress
    let health_indicator = if app.state.node_health { "●" } else { "○" };
    let health_color = if app.state.node_health { theme.success() } else { theme.error() };

    // Create epoch progress bar
    let epoch_progress = &app.state.epoch_progress;
    let progress_width = match layout.size {
        ScreenSize::Small => 10,
        ScreenSize::Medium => 20,
        ScreenSize::Large => 30,
    };
    let filled = ((epoch_progress.progress_percent / 100.0) * progress_width as f64) as usize;
    let progress_bar: String = format!(
        "{}{}",
        "━".repeat(filled.min(progress_width)),
        "░".repeat(progress_width.saturating_sub(filled))
    );

    let network_text = match layout.size {
        ScreenSize::Small => {
            // Compact network status for small screens
            vec![
                Line::from(vec![
                    Span::styled(health_indicator, Style::default().fg(health_color)),
                    Span::raw(" "),
                    Span::styled(format!("#{}", app.state.chain_tip), Style::default().fg(theme.block_number())),
                    Span::raw(" "),
                    Span::styled(format!("P:{}", app.state.peer_count), Style::default().fg(theme.muted())),
                ]),
                Line::from(vec![
                    Span::styled("E:", Style::default().fg(theme.muted())),
                    Span::styled(format!("{}", app.state.sidechain_epoch), Style::default().fg(theme.epoch())),
                    Span::raw(" "),
                    Span::styled(progress_bar.clone(), Style::default().fg(theme.primary())),
                    Span::styled(format!(" {:.0}%", epoch_progress.progress_percent), Style::default().fg(theme.text())),
                ]),
            ]
        }
        _ => {
            // Standard network status with enhanced info
            vec![
                Line::from(vec![
                    Span::styled(health_indicator, Style::default().fg(health_color)),
                    Span::styled(" Health: ", Style::default().fg(theme.muted())),
                    Span::styled(
                        if app.state.node_health { "OK" } else { "SYNCING" },
                        Style::default().fg(health_color)
                    ),
                    Span::raw("      "),
                    Span::styled("Peers: ", Style::default().fg(theme.muted())),
                    Span::styled(format!("{}", app.state.peer_count), Style::default().fg(theme.text())),
                    Span::raw("      "),
                    Span::styled("Block: ", Style::default().fg(theme.muted())),
                    Span::styled(format!("#{}", app.state.chain_tip), Style::default().fg(theme.block_number())),
                    Span::styled(
                        if app.state.chain_tip == app.state.finalized_block { " (finalized)" } else { "" },
                        Style::default().fg(theme.success())
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Mainchain: ", Style::default().fg(theme.muted())),
                    Span::styled(format!("epoch {}", app.state.mainchain_epoch), Style::default().fg(theme.epoch())),
                    Span::raw("    "),
                    Span::styled("Sidechain: ", Style::default().fg(theme.muted())),
                    Span::styled(format!("epoch {}", app.state.sidechain_epoch), Style::default().fg(theme.epoch())),
                    Span::raw("  "),
                    Span::styled(format!("slot {}", app.state.sidechain_slot), Style::default().fg(theme.muted())),
                ]),
                Line::from(vec![
                    Span::styled("Epoch Progress: ", Style::default().fg(theme.muted())),
                    Span::styled(progress_bar.clone(), Style::default().fg(theme.primary())),
                    Span::styled(format!(" {:.1}%", epoch_progress.progress_percent), Style::default().fg(theme.text())),
                    Span::raw("   "),
                    Span::styled("Database: ", Style::default().fg(theme.muted())),
                    Span::styled(format!("{} blocks", app.state.total_blocks), Style::default().fg(theme.text())),
                ]),
            ]
        }
    };

    let network_widget = if layout.size == ScreenSize::Small {
        Paragraph::new(network_text)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border()))
                .title(Span::styled("Net", Style::default().fg(theme.primary()).add_modifier(Modifier::BOLD))))
    } else {
        Paragraph::new(network_text)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border()))
                .title(Span::styled("Network Status", Style::default().fg(theme.primary()).add_modifier(Modifier::BOLD))))
    };
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

        let max_validators = match layout.size {
            ScreenSize::Small => 2,
            ScreenSize::Medium => 3,
            ScreenSize::Large => 5,
        };

        let mut lines = match layout.size {
            ScreenSize::Small => {
                vec![
                    Line::from(vec![
                        Span::styled("N:", Style::default().fg(theme.muted())),
                        Span::styled(format!("{}", app.state.our_validators_count), Style::default().fg(theme.success())),
                        Span::raw(" "),
                        Span::styled("Tot:", Style::default().fg(theme.muted())),
                        Span::styled(format!("{}", total_our_blocks), Style::default().fg(theme.success())),
                        Span::raw(" "),
                        Span::styled(format!("{:.1}%", share), Style::default().fg(theme.success())),
                    ]),
                    Line::from(vec![
                        Span::styled("Epoch:", Style::default().fg(theme.muted())),
                        Span::styled(format!("{}", epoch_blocks), Style::default().fg(theme.primary())),
                        Span::styled(format!("/{:.0}", expected_blocks), Style::default().fg(theme.muted())),
                        Span::styled(format!(" {}", performance_indicator), Style::default().fg(
                            if performance_indicator == "✓" { theme.success() }
                            else if performance_indicator == "!" { theme.warning() }
                            else { theme.muted() }
                        )),
                    ]),
                ]
            }
            _ => {
                vec![
                    Line::from(vec![
                        Span::styled("Count: ", Style::default().fg(theme.muted())),
                        Span::styled(format!("{}", app.state.our_validators_count), Style::default().fg(theme.success())),
                        Span::raw("      "),
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
                ]
            }
        };

        // Show validators based on screen size
        for v in app.state.our_validators.iter().take(max_validators) {
            let key_display = key_mode.format(&v.sidechain_key);
            let label = v.label.as_ref().map(|l| format!(" ({})", l)).unwrap_or_default();

            if layout.size == ScreenSize::Small {
                lines.push(Line::from(vec![
                    Span::styled("★", Style::default().fg(theme.ours())),
                    Span::styled(key_display, Style::default().fg(theme.secondary())),
                    Span::raw(" "),
                    Span::styled(format!("{}b", v.total_blocks), Style::default().fg(theme.success())),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::styled("  ★ ", Style::default().fg(theme.ours())),
                    Span::styled(key_display, Style::default().fg(theme.secondary())),
                    Span::styled(label, Style::default().fg(theme.muted())),
                    Span::raw(" - "),
                    Span::styled(format!("{} blocks", v.total_blocks), Style::default().fg(theme.success())),
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
        match layout.size {
            ScreenSize::Small => {
                vec![
                    Line::from(vec![
                        Span::styled("No validators", Style::default().fg(theme.warning())),
                    ]),
                    Line::from(vec![
                        Span::styled("mvm keys verify", Style::default().fg(theme.secondary())),
                    ]),
                ]
            }
            _ => {
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
            }
        }
    };

    let our_validators_widget = if layout.size == ScreenSize::Small {
        Paragraph::new(our_validators_text)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border()))
                .title(Span::styled("Ours", Style::default().fg(theme.ours()).add_modifier(Modifier::BOLD))))
    } else {
        Paragraph::new(our_validators_text)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border()))
                .title(Span::styled("Our Validators", Style::default().fg(theme.ours()).add_modifier(Modifier::BOLD))))
    };
    f.render_widget(our_validators_widget, chunks[1]);

    // Recent blocks - responsive
    let blocks_to_show = match layout.size {
        ScreenSize::Small => 5,
        ScreenSize::Medium => 10,
        ScreenSize::Large => 15,
    };

    let blocks_items: Vec<ListItem> = app.state.recent_blocks.iter().take(blocks_to_show).map(|block| {
        let author_display = if let Some(ref author) = block.author_key {
            key_mode.format(author)
        } else {
            "unknown".to_string()
        };

        let finalized = if block.is_finalized { "✓" } else { " " };

        let line = match layout.size {
            ScreenSize::Small => {
                Line::from(vec![
                    Span::styled(format!("#{}", block.block_number), Style::default().fg(theme.block_number())),
                    Span::raw(" "),
                    Span::styled(format!("e{}", block.epoch), Style::default().fg(theme.epoch())),
                    Span::raw(" "),
                    Span::styled(finalized, Style::default().fg(theme.success())),
                    Span::styled(author_display, Style::default().fg(theme.text())),
                ])
            }
            _ => {
                Line::from(vec![
                    Span::styled(format!("#{:<8}", block.block_number), Style::default().fg(theme.block_number())),
                    Span::raw("  "),
                    Span::styled(format!("slot {:>12}", block.slot_number), Style::default().fg(theme.muted())),
                    Span::raw("  "),
                    Span::styled(format!("epoch {:>4}", block.epoch), Style::default().fg(theme.epoch())),
                    Span::raw("  "),
                    Span::styled(format!("{} ", finalized), Style::default().fg(theme.success())),
                    Span::styled("author: ", Style::default().fg(theme.muted())),
                    Span::styled(author_display, Style::default().fg(theme.text())),
                ])
            }
        };

        ListItem::new(line)
    }).collect();

    let blocks_title = if layout.size == ScreenSize::Small {
        "Blocks"
    } else {
        "Recent Blocks"
    };

    let blocks_list = List::new(blocks_items)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border()))
            .title(Span::styled(blocks_title, Style::default().fg(theme.primary()).add_modifier(Modifier::BOLD))));
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

        let line = match layout.size {
            ScreenSize::Small => {
                Line::from(vec![
                    Span::styled(format!("#{}", block.block_number), Style::default().fg(theme.block_number())),
                    Span::raw(" "),
                    Span::styled(format!("e{}", block.epoch), Style::default().fg(theme.epoch())),
                    Span::raw(" "),
                    Span::styled(finalized, Style::default().fg(theme.success())),
                    Span::raw(" "),
                    Span::styled(author_display, Style::default().fg(theme.text())),
                ])
            }
            _ => {
                let mut spans = vec![
                    Span::styled(format!("#{:<8}", block.block_number), Style::default().fg(theme.block_number())),
                    Span::raw("  "),
                ];

                if block_cols.show_slot {
                    spans.push(Span::styled(format!("slot {:>12}", block.slot_number), Style::default().fg(theme.muted())));
                    spans.push(Span::raw("  "));
                }

                if block_cols.show_epoch {
                    spans.push(Span::styled(format!("epoch {:>4}", block.epoch), Style::default().fg(theme.epoch())));
                    spans.push(Span::raw("  "));
                }

                if block_cols.show_extrinsics {
                    spans.push(Span::styled(format!("extr:{:<3}", block.extrinsics_count), Style::default().fg(theme.text())));
                    spans.push(Span::raw("  "));
                }

                spans.push(Span::styled(format!("{} ", finalized), Style::default().fg(theme.success())));
                spans.push(Span::styled(author_display, Style::default().fg(theme.text())));

                Line::from(spans)
            }
        };

        ListItem::new(line)
    }).collect();

    let title = match layout.size {
        ScreenSize::Small => format!("Blocks ({})", app.state.total_blocks),
        _ => format!("Blocks ({} total, showing last 20) - Use j/k or ↑/↓ to scroll", app.state.total_blocks),
    };

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
        let status = v.registration_status.as_ref().map(|s| s.as_str()).unwrap_or("unknown");
        let ours = if v.is_ours { "★" } else { " " };
        let key_display = key_mode.format(&v.sidechain_key);

        let line = match layout.size {
            ScreenSize::Small => {
                Line::from(vec![
                    Span::styled(ours, Style::default().fg(theme.ours())),
                    Span::styled(key_display, Style::default().fg(theme.secondary())),
                    Span::raw(" "),
                    Span::styled(format!("{}b", v.total_blocks), Style::default().fg(theme.text())),
                ])
            }
            _ => {
                let mut spans = vec![
                    Span::styled(ours, Style::default().fg(theme.ours())),
                    Span::raw(" "),
                    Span::styled(key_display, Style::default().fg(theme.secondary())),
                    Span::raw("  "),
                ];

                if val_cols.show_status {
                    spans.push(Span::styled(format!("{:<15}", status), Style::default().fg(if v.is_ours { theme.success() } else { theme.muted() })));
                    spans.push(Span::raw("  "));
                }

                spans.push(Span::styled(format!("{:>5} blocks", v.total_blocks), Style::default().fg(theme.text())));

                Line::from(spans)
            }
        };

        ListItem::new(line)
    }).collect();

    let filter_text = if app.show_ours_only { " (ours)" } else { "" };
    let title = match layout.size {
        ScreenSize::Small => format!("Vals ({}{})", validators.len(), filter_text),
        _ => format!("Validators ({} total{}) - [F] filter, j/k or ↑/↓ scroll", validators.len(), filter_text),
    };

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

    let total_blocks: u64 = validators.iter().map(|v| v.total_blocks).sum();

    let validator_items: Vec<ListItem> = validators.iter().enumerate().map(|(i, v)| {
        let share = if total_blocks > 0 {
            (v.total_blocks as f64 / total_blocks as f64) * 100.0
        } else {
            0.0
        };

        let ours = if v.is_ours { "★" } else { " " };
        let key_display = key_mode.format(&v.sidechain_key);

        let line = match layout.size {
            ScreenSize::Small => {
                Line::from(vec![
                    Span::styled(format!("{}.", i + 1), Style::default().fg(theme.warning())),
                    Span::styled(ours, Style::default().fg(theme.ours())),
                    Span::styled(key_display, Style::default().fg(theme.secondary())),
                    Span::raw(" "),
                    Span::styled(format!("{}b", v.total_blocks), Style::default().fg(theme.text())),
                    Span::raw(" "),
                    Span::styled(format!("{:.1}%", share), Style::default().fg(if v.is_ours { theme.success() } else { theme.muted() })),
                ])
            }
            _ => {
                Line::from(vec![
                    Span::styled(format!("{:>3}.", i + 1), Style::default().fg(theme.warning())),
                    Span::raw(" "),
                    Span::styled(ours, Style::default().fg(theme.ours())),
                    Span::raw(" "),
                    Span::styled(key_display, Style::default().fg(theme.secondary())),
                    Span::raw("  "),
                    Span::styled(format!("{:>6} blocks", v.total_blocks), Style::default().fg(theme.text())),
                    Span::raw("  "),
                    Span::styled(format!("{:>6.3}%", share), Style::default().fg(if v.is_ours { theme.success() } else { theme.muted() })),
                ])
            }
        };

        ListItem::new(line)
    }).collect();

    let filter_text = if app.show_ours_only { " (ours)" } else { "" };
    let title = match layout.size {
        ScreenSize::Small => format!("Rank{}", filter_text),
        _ => format!("Performance Rankings{} - [F] filter, j/k or ↑/↓ scroll", filter_text),
    };

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

fn render_help(f: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme;
    let help_text = vec![
        Line::from(vec![
            Span::styled("Keyboard Shortcuts", Style::default().fg(theme.title()).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Navigation:", Style::default().fg(theme.primary()).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("    1-4      ", Style::default().fg(theme.text())),
            Span::raw("Switch to view (1=Dashboard, 2=Blocks, 3=Validators, 4=Performance)"),
        ]),
        Line::from(vec![
            Span::styled("    Tab      ", Style::default().fg(theme.text())),
            Span::raw("Next view"),
        ]),
        Line::from(vec![
            Span::styled("    Shift+Tab", Style::default().fg(theme.text())),
            Span::raw("  Previous view"),
        ]),
        Line::from(vec![
            Span::styled("    ?  /  h / F1", Style::default().fg(theme.text())),
            Span::raw("  Show this help"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Scrolling:", Style::default().fg(theme.primary()).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("    ↑  /  k  ", Style::default().fg(theme.text())),
            Span::raw("Scroll up"),
        ]),
        Line::from(vec![
            Span::styled("    ↓  /  j  ", Style::default().fg(theme.text())),
            Span::raw("Scroll down"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Options:", Style::default().fg(theme.primary()).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("    f  /  F  ", Style::default().fg(theme.text())),
            Span::raw("Toggle 'ours only' filter (in Validators and Performance views)"),
        ]),
        Line::from(vec![
            Span::styled("    t  /  T  ", Style::default().fg(theme.text())),
            Span::raw("Toggle theme (Midnight Mode ⟷ Daytime Mode)"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Quit:", Style::default().fg(theme.primary()).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("    q / Q / Esc", Style::default().fg(theme.text())),
            Span::raw("  Quit application"),
        ]),
        Line::from(vec![
            Span::styled("    Ctrl+C     ", Style::default().fg(theme.text())),
            Span::raw("  Quit application"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Views:", Style::default().fg(theme.primary()).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("    Dashboard   ", Style::default().fg(theme.text())),
            Span::raw(" - Network status, our validators, recent blocks"),
        ]),
        Line::from(vec![
            Span::styled("    Blocks      ", Style::default().fg(theme.text())),
            Span::raw(" - Detailed block list with authors and slots"),
        ]),
        Line::from(vec![
            Span::styled("    Validators  ", Style::default().fg(theme.text())),
            Span::raw(" - All validators with registration status"),
        ]),
        Line::from(vec![
            Span::styled("    Performance ", Style::default().fg(theme.text())),
            Span::raw(" - Rankings by block production"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Symbols:", Style::default().fg(theme.primary()).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("    ★ ", Style::default().fg(theme.ours())),
            Span::raw(" Our validator"),
        ]),
        Line::from(vec![
            Span::styled("    ✓ ", Style::default().fg(theme.success())),
            Span::raw(" Finalized block"),
        ]),
        Line::from(vec![
            Span::styled("    ● ", Style::default().fg(theme.success())),
            Span::raw(" Status indicator (Synced)"),
        ]),
    ];

    let help_paragraph = Paragraph::new(help_text)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border()))
            .title(Span::styled("Help", Style::default().fg(theme.primary()).add_modifier(Modifier::BOLD))))
        .wrap(Wrap { trim: true });
    f.render_widget(help_paragraph, area);
}
