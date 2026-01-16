//! UI rendering for TUI

use crate::tui::{App, ViewMode};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Row, Table, Wrap},
    Frame,
};
use std::time::Duration;

/// Render the UI
pub fn render(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title bar
            Constraint::Min(0),    // Main content
            Constraint::Length(3), // Status bar
        ])
        .split(f.size());

    // Render title bar
    render_title_bar(f, app, chunks[0]);

    // Render main content based on view mode
    match app.view_mode {
        ViewMode::Dashboard => render_dashboard(f, app, chunks[1]),
        ViewMode::Blocks => render_blocks(f, app, chunks[1]),
        ViewMode::Validators => render_validators(f, app, chunks[1]),
        ViewMode::Performance => render_performance(f, app, chunks[1]),
        ViewMode::Help => render_help(f, app, chunks[1]),
    }

    // Render status bar
    render_status_bar(f, app, chunks[2]);
}

fn render_title_bar(f: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme;

    let title = vec![
        Span::styled("Midnight Validator Monitor", Style::default().fg(theme.title()).add_modifier(Modifier::BOLD)),
        Span::styled(" v0.3.0", Style::default().fg(theme.muted())),
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
    ];

    let title_paragraph = Paragraph::new(Line::from(title))
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(theme.border())))
        .alignment(Alignment::Left);

    f.render_widget(title_paragraph, area);
}

fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme;
    let since_update = app.last_update.elapsed().as_secs();
    let status_text = if let Some(ref err) = app.state.last_error {
        vec![
            Span::styled("ERROR: ", Style::default().fg(theme.error()).add_modifier(Modifier::BOLD)),
            Span::styled(err, Style::default().fg(theme.error())),
        ]
    } else {
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
    };

    let status_paragraph = Paragraph::new(Line::from(status_text))
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(theme.border())))
        .alignment(Alignment::Left);

    f.render_widget(status_paragraph, area);
}

fn render_dashboard(f: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // Network status
            Constraint::Length(7), // Our validators (increased for more info)
            Constraint::Min(0),    // Recent blocks
        ])
        .split(area);

    // Network status
    let network_text = vec![
        Line::from(vec![
            Span::styled("Chain Tip:    ", Style::default().fg(theme.muted())),
            Span::styled(format!("#{}", app.state.chain_tip), Style::default().fg(theme.block_number())),
            Span::raw("      "),
            Span::styled("Finalized: ", Style::default().fg(theme.muted())),
            Span::styled(format!("#{}", app.state.finalized_block), Style::default().fg(theme.block_number())),
        ]),
        Line::from(vec![
            Span::styled("Mainchain Epoch: ", Style::default().fg(theme.muted())),
            Span::styled(format!("{}", app.state.mainchain_epoch), Style::default().fg(theme.epoch())),
            Span::raw("    "),
            Span::styled("Sidechain Epoch: ", Style::default().fg(theme.muted())),
            Span::styled(format!("{}", app.state.sidechain_epoch), Style::default().fg(theme.epoch())),
            Span::raw("  Slot: "),
            Span::styled(format!("{}", app.state.sidechain_slot), Style::default().fg(theme.epoch())),
        ]),
        Line::from(vec![
            Span::styled("Database: ", Style::default().fg(theme.muted())),
            Span::styled(format!("{} blocks", app.state.total_blocks), Style::default().fg(theme.text())),
            Span::raw(", "),
            Span::styled(format!("{} validators", app.state.total_validators), Style::default().fg(theme.text())),
        ]),
    ];

    let network_widget = Paragraph::new(network_text)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border()))
            .title(Span::styled("Network Status", Style::default().fg(theme.primary()).add_modifier(Modifier::BOLD))));
    f.render_widget(network_widget, chunks[0]);

    // Our validators
    let our_validators_text = if app.state.our_validators_count > 0 {
        let total_our_blocks: u64 = app.state.our_validators.iter().map(|v| v.total_blocks).sum();
        let share = if app.state.total_blocks > 0 {
            (total_our_blocks as f64 / app.state.total_blocks as f64) * 100.0
        } else {
            0.0
        };

        let mut lines = vec![
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
            Line::from(""),
        ];

        // Show up to 3 of our validators
        for (i, v) in app.state.our_validators.iter().take(3).enumerate() {
            let key_display = format!("{}...{}", &v.sidechain_key[..12], &v.sidechain_key[v.sidechain_key.len()-8..]);
            let label = v.label.as_ref().map(|l| format!(" ({})", l)).unwrap_or_default();
            lines.push(Line::from(vec![
                Span::styled("  ★ ", Style::default().fg(theme.ours())),
                Span::styled(key_display, Style::default().fg(theme.secondary())),
                Span::styled(label, Style::default().fg(theme.muted())),
                Span::raw(" - "),
                Span::styled(format!("{} blocks", v.total_blocks), Style::default().fg(theme.success())),
            ]));
        }

        if app.state.our_validators_count > 3 {
            lines.push(Line::from(vec![
                Span::styled(format!("  ... and {} more", app.state.our_validators_count - 3), Style::default().fg(theme.muted())),
            ]));
        }

        lines
    } else {
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
            .title(Span::styled("Our Validators", Style::default().fg(theme.ours()).add_modifier(Modifier::BOLD))));
    f.render_widget(our_validators_widget, chunks[1]);

    // Recent blocks
    let blocks_items: Vec<ListItem> = app.state.recent_blocks.iter().take(10).map(|block| {
        let author_short = if let Some(ref author) = block.author_key {
            format!("{}...{}", &author[..8], &author[author.len()-6..])
        } else {
            "unknown".to_string()
        };

        let finalized = if block.is_finalized { "✓" } else { " " };

        let line = Line::from(vec![
            Span::styled(format!("#{:<8}", block.block_number), Style::default().fg(theme.block_number())),
            Span::raw("  "),
            Span::styled(format!("slot {:>12}", block.slot_number), Style::default().fg(theme.muted())),
            Span::raw("  "),
            Span::styled(format!("epoch {:>4}", block.epoch), Style::default().fg(theme.epoch())),
            Span::raw("  "),
            Span::styled(format!("{} ", finalized), Style::default().fg(theme.success())),
            Span::styled("author: ", Style::default().fg(theme.muted())),
            Span::styled(author_short, Style::default().fg(theme.text())),
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

fn render_blocks(f: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme;
    let blocks_items: Vec<ListItem> = app.state.recent_blocks.iter().map(|block| {
        let author_display = if let Some(ref author) = block.author_key {
            format!("{}...{}", &author[..10], &author[author.len()-8..])
        } else {
            "unknown".to_string()
        };

        let finalized = if block.is_finalized { "✓" } else { " " };

        let line = Line::from(vec![
            Span::styled(format!("#{:<8}", block.block_number), Style::default().fg(theme.block_number())),
            Span::raw("  "),
            Span::styled(format!("slot {:>12}", block.slot_number), Style::default().fg(theme.muted())),
            Span::raw("  "),
            Span::styled(format!("epoch {:>4}", block.epoch), Style::default().fg(theme.epoch())),
            Span::raw("  "),
            Span::styled(format!("extr:{:<3}", block.extrinsics_count), Style::default().fg(theme.text())),
            Span::raw("  "),
            Span::styled(format!("{} ", finalized), Style::default().fg(theme.success())),
            Span::styled(author_display, Style::default().fg(theme.text())),
        ]);

        ListItem::new(line)
    }).collect();

    let title = format!("Blocks ({} total, showing last 20) - Use j/k or ↑/↓ to scroll", app.state.total_blocks);
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

fn render_validators(f: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme;
    let validators = if app.show_ours_only {
        &app.state.our_validators
    } else {
        &app.state.validators
    };

    let validator_items: Vec<ListItem> = validators.iter().map(|v| {
        let status = v.registration_status.as_ref().map(|s| s.as_str()).unwrap_or("unknown");
        let ours = if v.is_ours { "★" } else { " " };

        let line = Line::from(vec![
            Span::styled(ours, Style::default().fg(theme.ours())),
            Span::raw(" "),
            Span::styled(v.sidechain_key.clone(), Style::default().fg(theme.secondary())),
            Span::raw("  "),
            Span::styled(format!("{:<15}", status), Style::default().fg(if v.is_ours { theme.success() } else { theme.muted() })),
            Span::raw("  "),
            Span::styled(format!("{:>5} blocks", v.total_blocks), Style::default().fg(theme.text())),
        ]);

        ListItem::new(line)
    }).collect();

    let filter_text = if app.show_ours_only { " (ours only)" } else { "" };
    let title = format!("Validators ({} total{}) - [F] filter, j/k or ↑/↓ scroll", validators.len(), filter_text);
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

fn render_performance(f: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme;
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

        let key_short = format!("{}...{}", &v.sidechain_key[..12], &v.sidechain_key[v.sidechain_key.len()-8..]);

        let line = Line::from(vec![
            Span::styled(format!("{:>3}.", i + 1), Style::default().fg(theme.warning())),
            Span::raw(" "),
            Span::styled(ours, Style::default().fg(theme.ours())),
            Span::raw(" "),
            Span::styled(key_short, Style::default().fg(theme.secondary())),
            Span::raw("  "),
            Span::styled(format!("{:>6} blocks", v.total_blocks), Style::default().fg(theme.text())),
            Span::raw("  "),
            Span::styled(format!("{:>6.3}%", share), Style::default().fg(if v.is_ours { theme.success() } else { theme.muted() })),
        ]);

        ListItem::new(line)
    }).collect();

    let filter_text = if app.show_ours_only { " (ours only)" } else { "" };
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
