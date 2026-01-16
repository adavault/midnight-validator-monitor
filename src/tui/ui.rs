//! UI rendering for TUI

use crate::tui::{App, ViewMode};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Row, Table, Wrap},
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
        ViewMode::Help => render_help(f, chunks[1]),
    }

    // Render status bar
    render_status_bar(f, app, chunks[2]);
}

fn render_title_bar(f: &mut Frame, app: &App, area: Rect) {
    let title = vec![
        Span::styled("Midnight Validator Monitor", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(" v0.3.0"),
        Span::raw("  |  "),
        Span::styled(
            match app.view_mode {
                ViewMode::Dashboard => "[1] Dashboard",
                ViewMode::Blocks => "[2] Blocks",
                ViewMode::Validators => "[3] Validators",
                ViewMode::Performance => "[4] Performance",
                ViewMode::Help => "[?] Help",
            },
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ),
    ];

    let title_paragraph = Paragraph::new(Line::from(title))
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Left);

    f.render_widget(title_paragraph, area);
}

fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let since_update = app.last_update.elapsed().as_secs();
    let status_text = if let Some(ref err) = app.state.last_error {
        vec![
            Span::styled("ERROR: ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::raw(err),
        ]
    } else {
        vec![
            Span::styled("●", Style::default().fg(Color::Green)),
            Span::raw(format!(" Synced  |  Updated {}s ago  |  ", since_update)),
            Span::styled("[Q]", Style::default().fg(Color::Yellow)),
            Span::raw(" Quit  "),
            Span::styled("[1-4]", Style::default().fg(Color::Yellow)),
            Span::raw(" Views  "),
            Span::styled("[?]", Style::default().fg(Color::Yellow)),
            Span::raw(" Help"),
        ]
    };

    let status_paragraph = Paragraph::new(Line::from(status_text))
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Left);

    f.render_widget(status_paragraph, area);
}

fn render_dashboard(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // Network status
            Constraint::Length(5), // Our validators
            Constraint::Min(0),    // Recent blocks
        ])
        .split(area);

    // Network status
    let network_text = vec![
        Line::from(vec![
            Span::styled("Chain Tip:    ", Style::default().fg(Color::Gray)),
            Span::raw(format!("#{}", app.state.chain_tip)),
            Span::raw("      "),
            Span::styled("Finalized: ", Style::default().fg(Color::Gray)),
            Span::raw(format!("#{}", app.state.finalized_block)),
        ]),
        Line::from(vec![
            Span::styled("Mainchain Epoch: ", Style::default().fg(Color::Gray)),
            Span::raw(format!("{}", app.state.mainchain_epoch)),
            Span::raw("    "),
            Span::styled("Sidechain Slot: ", Style::default().fg(Color::Gray)),
            Span::raw(format!("{}", app.state.sidechain_slot)),
        ]),
        Line::from(vec![
            Span::styled("Database: ", Style::default().fg(Color::Gray)),
            Span::raw(format!("{} blocks, {} validators", app.state.total_blocks, app.state.total_validators)),
        ]),
    ];

    let network_widget = Paragraph::new(network_text)
        .block(Block::default().borders(Borders::ALL).title("Network Status"));
    f.render_widget(network_widget, chunks[0]);

    // Our validators
    let our_validators_text = if app.state.our_validators_count > 0 {
        let total_our_blocks: u64 = app.state.our_validators.iter().map(|v| v.total_blocks).sum();
        let share = if app.state.total_blocks > 0 {
            (total_our_blocks as f64 / app.state.total_blocks as f64) * 100.0
        } else {
            0.0
        };

        vec![
            Line::from(vec![
                Span::styled("Count: ", Style::default().fg(Color::Gray)),
                Span::styled(format!("{}", app.state.our_validators_count), Style::default().fg(Color::Green)),
                Span::raw("      "),
                Span::styled("Total Blocks: ", Style::default().fg(Color::Gray)),
                Span::styled(format!("{}", total_our_blocks), Style::default().fg(Color::Green)),
                Span::raw("      "),
                Span::styled("Share: ", Style::default().fg(Color::Gray)),
                Span::styled(format!("{:.2}%", share), Style::default().fg(Color::Green)),
            ]),
            Line::from(""),
            Line::from(if !app.state.our_validators.is_empty() {
                let v = &app.state.our_validators[0];
                format!("  {}...{} - {} blocks",
                    &v.sidechain_key[..12],
                    &v.sidechain_key[v.sidechain_key.len()-8..],
                    v.total_blocks)
            } else {
                "  No validators".to_string()
            }),
        ]
    } else {
        vec![
            Line::from(vec![
                Span::styled("No validators marked as 'ours'", Style::default().fg(Color::Yellow)),
            ]),
            Line::from(""),
            Line::from("  Run: mvm keys --keystore <path> verify"),
        ]
    };

    let our_validators_widget = Paragraph::new(our_validators_text)
        .block(Block::default().borders(Borders::ALL).title("Our Validators"));
    f.render_widget(our_validators_widget, chunks[1]);

    // Recent blocks
    let blocks_items: Vec<ListItem> = app.state.recent_blocks.iter().take(10).map(|block| {
        let author_short = if let Some(ref author) = block.author_key {
            format!("{}...{}", &author[..8], &author[author.len()-6..])
        } else {
            "unknown".to_string()
        };

        let line = Line::from(vec![
            Span::styled(format!("#{:<8}", block.block_number), Style::default().fg(Color::Cyan)),
            Span::raw("  "),
            Span::styled(format!("slot {:>12}", block.slot_number), Style::default().fg(Color::Gray)),
            Span::raw("  "),
            Span::styled(format!("epoch {:>4}", block.epoch), Style::default().fg(Color::Gray)),
            Span::raw("  "),
            Span::raw(format!("author: {}", author_short)),
        ]);

        ListItem::new(line)
    }).collect();

    let blocks_list = List::new(blocks_items)
        .block(Block::default().borders(Borders::ALL).title("Recent Blocks"));
    f.render_widget(blocks_list, chunks[2]);
}

fn render_blocks(f: &mut Frame, app: &App, area: Rect) {
    let blocks_items: Vec<ListItem> = app.state.recent_blocks.iter().enumerate().map(|(i, block)| {
        let author_display = if let Some(ref author) = block.author_key {
            format!("{}...{}", &author[..10], &author[author.len()-8..])
        } else {
            "unknown".to_string()
        };

        let finalized = if block.is_finalized { "✓" } else { " " };

        let style = if i == app.selected_index {
            Style::default().bg(Color::DarkGray)
        } else {
            Style::default()
        };

        let line = Line::from(vec![
            Span::styled(format!("#{:<8}", block.block_number), style.fg(Color::Cyan)),
            Span::raw("  "),
            Span::styled(format!("slot {:>12}", block.slot_number), style.fg(Color::Gray)),
            Span::raw("  "),
            Span::styled(format!("epoch {:>4}", block.epoch), style),
            Span::raw("  "),
            Span::styled(format!("extr:{:<3}", block.extrinsics_count), style),
            Span::raw("  "),
            Span::styled(format!("{} ", finalized), style.fg(Color::Green)),
            Span::styled(author_display, style),
        ]);

        ListItem::new(line).style(style)
    }).collect();

    let title = format!("Blocks ({} total) - Use ↑/↓ to scroll", app.state.total_blocks);
    let blocks_list = List::new(blocks_items)
        .block(Block::default().borders(Borders::ALL).title(title));
    f.render_widget(blocks_list, area);
}

fn render_validators(f: &mut Frame, app: &App, area: Rect) {
    let validators = if app.show_ours_only {
        &app.state.our_validators
    } else {
        &app.state.validators
    };

    let validator_items: Vec<ListItem> = validators.iter().enumerate().map(|(i, v)| {
        let style = if i == app.selected_index {
            Style::default().bg(Color::DarkGray)
        } else {
            Style::default()
        };

        let status = v.registration_status.as_ref().map(|s| s.as_str()).unwrap_or("unknown");
        let ours = if v.is_ours { "★" } else { " " };

        let line = Line::from(vec![
            Span::styled(ours, style.fg(Color::Yellow)),
            Span::raw(" "),
            Span::styled(v.sidechain_key.clone(), style.fg(Color::Cyan)),
            Span::raw("  "),
            Span::styled(format!("{:<15}", status), style.fg(if v.is_ours { Color::Green } else { Color::Gray })),
            Span::raw("  "),
            Span::styled(format!("{:>5} blocks", v.total_blocks), style),
        ]);

        ListItem::new(line).style(style)
    }).collect();

    let filter_text = if app.show_ours_only { " (ours only)" } else { "" };
    let title = format!("Validators ({} total{}) - [F] filter, ↑/↓ scroll", validators.len(), filter_text);
    let validators_list = List::new(validator_items)
        .block(Block::default().borders(Borders::ALL).title(title));
    f.render_widget(validators_list, area);
}

fn render_performance(f: &mut Frame, app: &App, area: Rect) {
    let mut validators = if app.show_ours_only {
        app.state.our_validators.clone()
    } else {
        app.state.validators.clone()
    };

    // Sort by block count descending
    validators.sort_by(|a, b| b.total_blocks.cmp(&a.total_blocks));

    let total_blocks: u64 = validators.iter().map(|v| v.total_blocks).sum();

    let validator_items: Vec<ListItem> = validators.iter().enumerate().take(50).map(|(i, v)| {
        let style = if i == app.selected_index {
            Style::default().bg(Color::DarkGray)
        } else {
            Style::default()
        };

        let share = if total_blocks > 0 {
            (v.total_blocks as f64 / total_blocks as f64) * 100.0
        } else {
            0.0
        };

        let ours = if v.is_ours { "★" } else { " " };

        let key_short = format!("{}...{}", &v.sidechain_key[..12], &v.sidechain_key[v.sidechain_key.len()-8..]);

        let line = Line::from(vec![
            Span::styled(format!("{:>3}.", i + 1), style.fg(Color::Yellow)),
            Span::raw(" "),
            Span::styled(ours, style.fg(Color::Yellow)),
            Span::raw(" "),
            Span::styled(key_short, style.fg(Color::Cyan)),
            Span::raw("  "),
            Span::styled(format!("{:>6} blocks", v.total_blocks), style),
            Span::raw("  "),
            Span::styled(format!("{:>6.2}%", share), style.fg(if v.is_ours { Color::Green } else { Color::Gray })),
        ]);

        ListItem::new(line).style(style)
    }).collect();

    let filter_text = if app.show_ours_only { " (ours only)" } else { "" };
    let title = format!("Performance Rankings{} - [F] filter, ↑/↓ scroll", filter_text);
    let performance_list = List::new(validator_items)
        .block(Block::default().borders(Borders::ALL).title(title));
    f.render_widget(performance_list, area);
}

fn render_help(f: &mut Frame, area: Rect) {
    let help_text = vec![
        Line::from(vec![
            Span::styled("Keyboard Shortcuts", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Navigation:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from("    1-4      Switch to view (1=Dashboard, 2=Blocks, 3=Validators, 4=Performance)"),
        Line::from("    Tab      Next view"),
        Line::from("    Shift+Tab  Previous view"),
        Line::from("    ?  /  h / F1  Show this help"),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Scrolling:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from("    ↑  /  k  Scroll up"),
        Line::from("    ↓  /  j  Scroll down"),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Filters:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from("    f  /  F  Toggle 'ours only' filter (in Validators and Performance views)"),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Other:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from("    q / Q / Esc  Quit"),
        Line::from("    Ctrl+C       Quit"),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Views:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from("    Dashboard    - Network status, our validators, recent blocks"),
        Line::from("    Blocks       - Detailed block list with authors and slots"),
        Line::from("    Validators   - All validators with registration status"),
        Line::from("    Performance  - Rankings by block production"),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Symbols:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from("    ★  Our validator"),
        Line::from("    ✓  Finalized block"),
        Line::from("    ●  Status indicator (Green=OK, Red=Error)"),
    ];

    let help_paragraph = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title("Help"))
        .wrap(Wrap { trim: true });
    f.render_widget(help_paragraph, area);
}
