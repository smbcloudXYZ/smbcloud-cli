//! Deployments list TUI table.
//!
//! Interactive scrollable table for a list of deployments.
//! Navigate with ↑ / k and ↓ / j.  Press `q` or `Esc` to quit.
use super::theme::{
    BG, BG_HEADER_ROW, BG_SURFACE, BORDER_ACTIVE, BORDER_IDLE, BRAND, BRAND_DARK, BRAND_LIGHT,
    BRAND_MID, TEXT_MUTED, TEXT_PRIMARY,
};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Margin},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState,
        Table, TableState,
    },
    Terminal,
};
use smbcloud_model::project::{Deployment, DeploymentStatus};
use std::io;

// ── Internal app state ────────────────────────────────────────────────────────

struct App {
    state: TableState,
    scroll: ScrollbarState,
    deployments: Vec<Deployment>,
}

impl App {
    fn new(deployments: Vec<Deployment>) -> Self {
        let count = deployments.len();
        let mut state = TableState::default();
        if count > 0 {
            state.select(Some(0));
        }
        Self {
            state,
            scroll: ScrollbarState::new(count.saturating_sub(1)),
            deployments,
        }
    }

    fn next(&mut self) {
        if self.deployments.is_empty() {
            return;
        }
        let next = match self.state.selected() {
            Some(i) => (i + 1).min(self.deployments.len() - 1),
            None => 0,
        };
        self.state.select(Some(next));
        self.scroll = self.scroll.position(next);
    }

    fn previous(&mut self) {
        if self.deployments.is_empty() {
            return;
        }
        let prev = match self.state.selected() {
            Some(i) => i.saturating_sub(1),
            None => 0,
        };
        self.state.select(Some(prev));
        self.scroll = self.scroll.position(prev);
    }
}

// ── Public entry-point ────────────────────────────────────────────────────────

/// Render an interactive, scrollable deployments table.
///
/// Enters the alternate screen, blocks until `q` / `Esc`, then restores
/// the terminal.
pub fn show_deployments_tui(deployments: Vec<Deployment>) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(deployments);
    let result = run_loop(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

// ── Event loop ────────────────────────────────────────────────────────────────

fn run_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|frame| render(frame, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                KeyCode::Down | KeyCode::Char('j') => app.next(),
                KeyCode::Up | KeyCode::Char('k') => app.previous(),
                _ => {}
            }
        }
    }
}

// ── Rendering ─────────────────────────────────────────────────────────────────

fn render(frame: &mut ratatui::Frame, app: &mut App) {
    let area = frame.area();

    frame.render_widget(Block::default().style(Style::default().bg(BG)), area);

    let zones = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // title bar
            Constraint::Min(0),    // table body
            Constraint::Length(2), // footer
        ])
        .split(area);

    render_title(frame, zones[0]);
    render_table(frame, app, zones[1]);
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
        Span::styled("Deployments", Style::default().fg(TEXT_MUTED)),
    ]);

    frame.render_widget(
        Paragraph::new(line)
            .block(block)
            .alignment(Alignment::Center),
        area,
    );
}

// ── Deployments table ─────────────────────────────────────────────────────────

fn status_cell(status: &DeploymentStatus) -> Cell<'static> {
    let (text, color): (&'static str, Color) = match status {
        DeploymentStatus::Started => ("🚀  Starting", Color::Rgb(251, 191, 36)), // amber-400
        DeploymentStatus::Done => ("✅  Done", Color::Rgb(34, 197, 94)),         // green-500
        DeploymentStatus::Failed => ("❌  Failed", BRAND),                       // brand-600
    };
    Cell::from(text).style(Style::default().fg(color).add_modifier(Modifier::BOLD))
}

fn render_table(frame: &mut ratatui::Frame, app: &mut App, area: ratatui::layout::Rect) {
    let header = Row::new(
        ["#", "Commit", "Status", "Created", "Updated"]
            .iter()
            .map(|label| {
                Cell::from(*label).style(
                    Style::default()
                        .fg(BRAND)
                        .bg(BG_HEADER_ROW)
                        .add_modifier(Modifier::BOLD),
                )
            }),
    )
    .height(1)
    .bottom_margin(0);

    let rows = app.deployments.iter().enumerate().map(|(idx, deployment)| {
        let base_bg = if idx % 2 == 0 {
            BG
        } else {
            Color::Rgb(13, 13, 18)
        };

        // Truncate commit hash to 12 chars for a short-hash display.
        let short_commit: String = deployment.commit_hash.chars().take(12).collect();
        let created = deployment.created_at.format("%Y-%m-%d %H:%M").to_string();
        let updated = deployment.updated_at.format("%Y-%m-%d %H:%M").to_string();

        Row::new([
            Cell::from(deployment.id.to_string()),
            Cell::from(short_commit),
            status_cell(&deployment.status),
            Cell::from(created),
            Cell::from(updated),
        ])
        .style(Style::default().fg(TEXT_PRIMARY).bg(base_bg))
        .height(1)
    });

    let widths = [
        Constraint::Length(6),      // #
        Constraint::Length(14),     // Commit (12 chars + 2 padding)
        Constraint::Percentage(20), // Status
        Constraint::Percentage(25), // Created
        Constraint::Fill(1),        // Updated
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(BORDER_ACTIVE))
                .style(Style::default().bg(BG)),
        )
        .row_highlight_style(
            Style::default()
                .fg(BRAND_LIGHT)
                .bg(BRAND_DARK)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(table, area, &mut app.state);

    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"))
            .thumb_style(Style::default().fg(BRAND_MID))
            .track_style(Style::default().fg(BORDER_IDLE)),
        area.inner(Margin {
            vertical: 1,
            horizontal: 0,
        }),
        &mut app.scroll,
    );
}

// ── Footer ────────────────────────────────────────────────────────────────────

fn render_footer(frame: &mut ratatui::Frame, area: ratatui::layout::Rect) {
    let hints: &[(&str, &str)] = &[("↑ / k", "Up"), ("↓ / j", "Down"), ("q / Esc", "Quit")];

    let mut spans = Vec::with_capacity(hints.len() * 4);
    for (i, (key, label)) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("   ", Style::default()));
        }
        spans.push(Span::styled(
            format!(" {key} "),
            Style::default()
                .fg(BRAND)
                .bg(BRAND_DARK)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            format!(" {label}"),
            Style::default().fg(TEXT_MUTED),
        ));
    }

    let footer = Paragraph::new(Line::from(spans))
        .alignment(Alignment::Center)
        .style(Style::default().bg(BG_SURFACE));

    frame.render_widget(footer, area);
}
