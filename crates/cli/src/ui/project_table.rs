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
use smbcloud_model::project::Project;
use std::io;

// ── Internal app state ────────────────────────────────────────────────────────────────────────────

struct App {
    state: TableState,
    scroll: ScrollbarState,
    projects: Vec<Project>,
}

impl App {
    fn new(projects: Vec<Project>) -> Self {
        let count = projects.len();
        let mut state = TableState::default();
        if count > 0 {
            state.select(Some(0));
        }
        Self {
            state,
            scroll: ScrollbarState::new(count.saturating_sub(1)),
            projects,
        }
    }

    fn next(&mut self) {
        if self.projects.is_empty() {
            return;
        }
        let next = match self.state.selected() {
            Some(i) => (i + 1).min(self.projects.len() - 1),
            None => 0,
        };
        self.state.select(Some(next));
        self.scroll = self.scroll.position(next);
    }

    fn previous(&mut self) {
        if self.projects.is_empty() {
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

// ── Public entry-point ──────────────────────────────────────────────────────────────────────────────

/// Render an interactive, scrollable project list table.
///
/// Switches to the alternate screen and blocks until the user presses
/// `q` or `Esc`, then restores the terminal exactly as it was.
/// Any IO error is propagated to the caller.
pub fn show_projects_tui(projects: Vec<Project>) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(projects);
    let result = run_loop(&mut terminal, &mut app);

    // Always restore terminal state, even on error.
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

// ── Event loop ────────────────────────────────────────────────────────────────────────────────────

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

// ── Rendering ─────────────────────────────────────────────────────────────────────────────────────────

fn render(frame: &mut ratatui::Frame, app: &mut App) {
    let area = frame.area();

    // Root canvas background.
    frame.render_widget(Block::default().style(Style::default().bg(BG)), area);

    let zones = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // title bar
            Constraint::Min(0),    // table body
            Constraint::Length(2), // key-hint footer
        ])
        .split(area);

    render_title(frame, zones[0]);
    render_table(frame, app, zones[1]);
    render_footer(frame, zones[2]);
}

// ── Title bar ─────────────────────────────────────────────────────────────────────────────────────────

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
        Span::styled("Projects", Style::default().fg(TEXT_MUTED)),
    ]);

    let title = Paragraph::new(line)
        .block(block)
        .alignment(Alignment::Center);

    frame.render_widget(title, area);
}

// ── Projects table ────────────────────────────────────────────────────────────────────────────────────

fn render_table(frame: &mut ratatui::Frame, app: &mut App, area: ratatui::layout::Rect) {
    // Column header row.
    let header = Row::new(
        ["#", "Name", "Runner", "Repository", "Description"]
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

    // Data rows.
    let rows = app.projects.iter().enumerate().map(|(idx, project)| {
        // Subtle zebra-stripe: slightly lighter surface on even rows.
        let base_bg = if idx % 2 == 0 {
            BG
        } else {
            Color::Rgb(13, 13, 18)
        };

        Row::new([
            Cell::from(project.id.to_string()),
            Cell::from(project.name.clone()),
            Cell::from(project.runner.to_string()),
            Cell::from(project.repository.clone().unwrap_or_else(|| "—".into())),
            Cell::from(project.description.clone().unwrap_or_else(|| "—".into())),
        ])
        .style(Style::default().fg(TEXT_PRIMARY).bg(base_bg))
        .height(1)
    });

    // Column widths.
    let widths = [
        Constraint::Length(6),      // #
        Constraint::Percentage(22), // Name
        Constraint::Percentage(14), // Runner
        Constraint::Percentage(28), // Repository
        Constraint::Fill(1),        // Description (greedy)
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(BORDER_ACTIVE))
                .style(Style::default().bg(BG)),
        )
        // Selected row: brand-800 bg + brand-300 fg, bold.
        .row_highlight_style(
            Style::default()
                .fg(BRAND_LIGHT)
                .bg(BRAND_DARK)
                .add_modifier(Modifier::BOLD),
        )
        // Right-pointing triangle glyph marks the active row.
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(table, area, &mut app.state);

    // Scrollbar on the right edge of the table body.
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

// ── Key-hint footer ───────────────────────────────────────────────────────────────────────────────────────

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
