//! Profile / "me" TUI view.
//!
//! Displays the authenticated user's account details in a centred card
//! with the smbCloud brand palette.  Blocks until the user presses `q` or
//! `Esc`, then restores the terminal.
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
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use smbcloud_model::account::User;
use std::io;

// ── Public entry-point ────────────────────────────────────────────────────────

/// Render the interactive account profile card.
///
/// Switches to the alternate screen and blocks until `q` / `Esc` is pressed,
/// then restores the terminal exactly as it was.
pub fn show_user_tui(user: &User) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_loop(&mut terminal, user);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

// ── Event loop ────────────────────────────────────────────────────────────────

fn run_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    user: &User,
) -> io::Result<()> {
    loop {
        terminal.draw(|frame| render(frame, user))?;

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

fn render(frame: &mut ratatui::Frame, user: &User) {
    let area = frame.area();

    frame.render_widget(Block::default().style(Style::default().bg(BG)), area);

    let zones = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // title bar
            Constraint::Min(0),    // profile card
            Constraint::Length(2), // footer
        ])
        .split(area);

    render_title(frame, zones[0]);
    render_profile(frame, user, zones[1]);
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
        Span::styled("Profile", Style::default().fg(TEXT_MUTED)),
    ]);

    frame.render_widget(
        Paragraph::new(line)
            .block(block)
            .alignment(Alignment::Center),
        area,
    );
}

// ── Profile card ──────────────────────────────────────────────────────────────

fn render_profile(frame: &mut ratatui::Frame, user: &User, area: ratatui::layout::Rect) {
    // Center the card horizontally — at most 66 columns wide.
    let h_zones = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Max(66),
            Constraint::Fill(1),
        ])
        .split(area);

    let card_col = h_zones[1];

    // Center the card vertically — 13 lines tall (11 content + 2 border).
    let v_zones = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(13),
            Constraint::Fill(1),
        ])
        .split(card_col);

    let card_area = v_zones[1];

    // Separator line — scales with the actual card width (minus 2 borders, minus 3 padding).
    let sep_len = (card_area.width.saturating_sub(5)) as usize;
    let separator = "─".repeat(sep_len);

    let member_since = user.created_at.format("%Y-%m-%d").to_string();
    let last_updated = user.updated_at.format("%Y-%m-%d").to_string();

    let content: Vec<Line<'static>> = vec![
        Line::from(""),
        // ◉  email — the primary identity line.
        Line::from(vec![
            Span::raw("   "),
            Span::styled(
                "◉  ",
                Style::default().fg(BRAND).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                user.email.clone(),
                Style::default()
                    .fg(TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        // Thin separator in brand-700.
        Line::from(vec![
            Span::raw("   "),
            Span::styled(separator, Style::default().fg(BRAND_MID)),
        ]),
        Line::from(""),
        // Field rows.
        field_line("ID", &user.id.to_string()),
        Line::from(""),
        field_line("Member since", &member_since),
        field_line("Last updated", &last_updated),
        Line::from(""),
    ];

    let block = Block::default()
        .title(Span::styled(
            " Account ",
            Style::default().fg(BRAND).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER_ACTIVE))
        .style(Style::default().bg(BG_SURFACE));

    frame.render_widget(Paragraph::new(content).block(block), card_area);
}

/// Render a single label / value field row with consistent alignment.
fn field_line(label: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::raw("   "),
        Span::styled(format!("{:<16}", label), Style::default().fg(TEXT_MUTED)),
        Span::styled(value.to_string(), Style::default().fg(TEXT_PRIMARY)),
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
