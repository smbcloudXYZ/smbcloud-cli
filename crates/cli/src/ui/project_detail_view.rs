//! Project detail TUI card.
//!
//! Shows all key fields for a single project in a centred card.
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
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use smbcloud_model::project::Project;
use std::io;

// ── Public entry-point ────────────────────────────────────────────────────────

/// Render a project detail card.
///
/// Enters the alternate screen, blocks until `q` / `Esc`, then restores
/// the terminal.
pub fn show_project_detail_tui(project: &Project) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_loop(&mut terminal, project);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

// ── Event loop ────────────────────────────────────────────────────────────────

fn run_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    project: &Project,
) -> io::Result<()> {
    loop {
        terminal.draw(|frame| render(frame, project))?;

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

fn render(frame: &mut ratatui::Frame, project: &Project) {
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
    render_card(frame, project, zones[1]);
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
        Span::styled("Project", Style::default().fg(TEXT_MUTED)),
    ]);

    frame.render_widget(
        Paragraph::new(line)
            .block(block)
            .alignment(Alignment::Center),
        area,
    );
}

// ── Project detail card ───────────────────────────────────────────────────────

fn render_card(frame: &mut ratatui::Frame, project: &Project, area: ratatui::layout::Rect) {
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

    // 15 content lines + 2 border rows = 17 total.
    let v_zones = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(17),
            Constraint::Fill(1),
        ])
        .split(card_col);

    let card_area = v_zones[1];

    let sep_len = (card_area.width.saturating_sub(5)) as usize;
    let separator = "─".repeat(sep_len);

    let created_at = project.created_at.format("%Y-%m-%d").to_string();
    let updated_at = project.updated_at.format("%Y-%m-%d").to_string();

    let content: Vec<Line<'static>> = vec![
        Line::from(""),
        // ◈  project-name — primary identity.
        Line::from(vec![
            Span::raw("   "),
            Span::styled(
                "◈  ",
                Style::default().fg(BRAND).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                project.name.clone(),
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
        // Fields.
        field_line("ID", &project.id.to_string()),
        field_line("Runner", &project.runner.to_string()),
        field_line("Deploy method", &project.deployment_method.to_string()),
        Line::from(""),
        field_line(
            "Repository",
            &project.repository.clone().unwrap_or_else(|| "—".into()),
        ),
        field_line(
            "Description",
            &project.description.clone().unwrap_or_else(|| "—".into()),
        ),
        Line::from(""),
        field_line("Created at", &created_at),
        field_line("Updated at", &updated_at),
        Line::from(""),
    ];

    let block = Block::default()
        .title(Span::styled(
            " Project ",
            Style::default().fg(BRAND).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER_ACTIVE))
        .style(Style::default().bg(BG_SURFACE));

    frame.render_widget(Paragraph::new(content).block(block), card_area);
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Label / value row with 16-char label padding.
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
