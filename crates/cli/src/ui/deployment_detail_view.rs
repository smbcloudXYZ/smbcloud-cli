//! Deployment detail TUI card.
//!
//! Shows all fields for a single deployment in a centred card.
//! Blocks until `q` / `Esc` is pressed, then restores the terminal.
use super::theme::{
    BG, BG_SURFACE, BORDER_ACTIVE, BORDER_IDLE, BRAND, BRAND_DARK, BRAND_MID, TEXT_MUTED,
    TEXT_PRIMARY,
};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use smbcloud_model::project::{Deployment, DeploymentStatus};
use std::io;

// ── Public entry-point ────────────────────────────────────────────────────────

/// Render a deployment detail card.
///
/// Enters the alternate screen, blocks until `q` / `Esc`, then restores
/// the terminal.
pub fn show_deployment_detail_tui(deployment: &Deployment) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_loop(&mut terminal, deployment);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

// ── Event loop ────────────────────────────────────────────────────────────────

fn run_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    deployment: &Deployment,
) -> io::Result<()> {
    loop {
        terminal.draw(|frame| render(frame, deployment))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            if matches!(key.code, KeyCode::Char('q') | KeyCode::Esc) {
                return Ok(());
            }
        }
    }
}

// ── Rendering ─────────────────────────────────────────────────────────────────

fn render(frame: &mut ratatui::Frame, deployment: &Deployment) {
    let area = frame.area();

    frame.render_widget(Block::default().style(Style::default().bg(BG)), area);

    let zones = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // title bar
            Constraint::Min(0),    // card
            Constraint::Length(2), // footer
        ])
        .split(area);

    render_title(frame, zones[0]);
    render_card(frame, deployment, zones[1]);
    render_footer(frame, zones[2]);
}

// ── Title bar ─────────────────────────────────────────────────────────────────

fn render_title(frame: &mut ratatui::Frame, area: ratatui::layout::Rect) {
    let block = Block::default()
        .style(Style::default().bg(BG_SURFACE))
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(BORDER_IDLE));

    let line = Line::from(vec![
        Span::styled(
            "smb",
            Style::default().fg(BRAND).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "Cloud",
            Style::default()
                .fg(TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  ·  ", Style::default().fg(BORDER_ACTIVE)),
        Span::styled("Deployment", Style::default().fg(TEXT_MUTED)),
    ]);

    frame.render_widget(
        Paragraph::new(line)
            .block(block)
            .alignment(Alignment::Center),
        area,
    );
}

// ── Deployment detail card ────────────────────────────────────────────────────

fn render_card(frame: &mut ratatui::Frame, deployment: &Deployment, area: ratatui::layout::Rect) {
    // Horizontally center — at most 72 columns wide.
    let h_zones = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Max(72),
            Constraint::Fill(1),
        ])
        .split(area);

    let card_col = h_zones[1];

    // 13 content lines + 2 border rows = 15 total.
    let v_zones = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(15),
            Constraint::Fill(1),
        ])
        .split(card_col);

    let card_area = v_zones[1];

    let sep_len = (card_area.width.saturating_sub(5)) as usize;
    let separator = "─".repeat(sep_len);

    let created_at = deployment
        .created_at
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    let updated_at = deployment
        .updated_at
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();

    let mut content: Vec<Line<'static>> = vec![
        Line::from(""),
        // Primary identity: "◈  Deploy #42"
        Line::from(vec![
            Span::raw("   "),
            Span::styled(
                "◈  ",
                Style::default().fg(BRAND).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("Deploy #{}", deployment.id),
                Style::default()
                    .fg(TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        // Separator in brand-700.
        Line::from(vec![
            Span::raw("   "),
            Span::styled(separator, Style::default().fg(BRAND_MID)),
        ]),
        Line::from(""),
        field_line("ID", &deployment.id.to_string()),
        field_line("Project ID", &deployment.project_id.to_string()),
        field_line("Commit", &deployment.commit_hash),
        // Status row — coloured by outcome.
        status_line(&deployment.status),
        Line::from(""),
        field_line("Created at", &created_at),
        field_line("Updated at", &updated_at),
        Line::from(""),
    ];

    // Ensure content is exactly 13 lines (pad if short).
    while content.len() < 13 {
        content.push(Line::from(""));
    }

    let block = Block::default()
        .title(Span::styled(
            " Deployment ",
            Style::default().fg(BRAND).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER_ACTIVE))
        .style(Style::default().bg(BG_SURFACE));

    frame.render_widget(Paragraph::new(content).block(block), card_area);
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn field_line(label: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::raw("   "),
        Span::styled(format!("{:<16}", label), Style::default().fg(TEXT_MUTED)),
        Span::styled(value.to_string(), Style::default().fg(TEXT_PRIMARY)),
    ])
}

fn status_line(status: &DeploymentStatus) -> Line<'static> {
    let (label, color): (&'static str, Color) = match status {
        DeploymentStatus::Started => ("🚀  Starting", Color::Rgb(251, 191, 36)), // amber-400
        DeploymentStatus::Done => ("✅  Done", Color::Rgb(34, 197, 94)),         // green-500
        DeploymentStatus::Failed => ("❌  Failed", BRAND),                       // brand-600
    };
    Line::from(vec![
        Span::raw("   "),
        Span::styled(format!("{:<16}", "Status"), Style::default().fg(TEXT_MUTED)),
        Span::styled(
            label,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
    ])
}

// ── Footer ────────────────────────────────────────────────────────────────────

fn render_footer(frame: &mut ratatui::Frame, area: ratatui::layout::Rect) {
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(
            " q / Esc ",
            Style::default()
                .fg(BRAND)
                .bg(BRAND_DARK)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  Quit", Style::default().fg(TEXT_MUTED)),
    ]))
    .alignment(Alignment::Center)
    .style(Style::default().bg(BG_SURFACE));

    frame.render_widget(footer, area);
}
