//! Branded deletion-confirmation modal.
//!
//! Displays a centred danger-zone dialog and returns `true` if the user
//! confirms, `false` if they cancel or press `Esc`.
//!
//! Keys:
//!  `y`                    → confirm immediately
//!  `n` / `q` / `Esc`     → cancel immediately
//!  `←` / `→` / `Tab`     → toggle between Confirm and Cancel buttons
//!  `Enter`                → activate the currently selected button
use super::theme::{BG, BG_SURFACE, BRAND, BRAND_DARK, BRAND_LIGHT, TEXT_MUTED, TEXT_PRIMARY};
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
use std::io;

// ── State ─────────────────────────────────────────────────────────────────────

/// Which button currently has focus.
/// Defaults to `Cancel` (index 1) to prevent accidental destructive actions.
struct State {
    selected: usize, // 0 = Confirm, 1 = Cancel
}

impl State {
    fn new() -> Self {
        Self { selected: 1 }
    }

    fn toggle(&mut self) {
        self.selected = 1 - self.selected;
    }
}

// ── Public entry-point ────────────────────────────────────────────────────────

/// Display a full-screen danger confirmation dialog.
///
/// Returns `Ok(true)` if the user confirmed, `Ok(false)` if they cancelled.
pub fn confirm_delete_tui(message: &str) -> io::Result<bool> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = State::new();
    let result = run_loop(&mut terminal, &mut state, message);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

// ── Event loop ────────────────────────────────────────────────────────────────

fn run_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    state: &mut State,
    message: &str,
) -> io::Result<bool> {
    loop {
        terminal.draw(|frame| render(frame, state, message))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                // Quick shortcuts.
                KeyCode::Char('y') => return Ok(true),
                KeyCode::Char('n') | KeyCode::Char('q') | KeyCode::Esc => return Ok(false),
                // Button navigation.
                KeyCode::Left | KeyCode::Right | KeyCode::Tab => state.toggle(),
                // Activate selected button.
                KeyCode::Enter => return Ok(state.selected == 0),
                _ => {}
            }
        }
    }
}

// ── Rendering ─────────────────────────────────────────────────────────────────

fn render(frame: &mut ratatui::Frame, state: &State, message: &str) {
    let area = frame.area();

    // Full-screen near-black background.
    frame.render_widget(Block::default().style(Style::default().bg(BG)), area);

    // Split content area + footer.
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(2)])
        .split(area);

    render_modal(frame, state, message, outer[0]);
    render_footer(frame, outer[1]);
}

fn render_modal(
    frame: &mut ratatui::Frame,
    state: &State,
    message: &str,
    area: ratatui::layout::Rect,
) {
    // Horizontally center — at most 56 columns wide.
    let h_zones = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Max(56),
            Constraint::Fill(1),
        ])
        .split(area);

    // Vertically center — 11 lines tall (9 content + 2 border).
    let v_zones = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(11),
            Constraint::Fill(1),
        ])
        .split(h_zones[1]);

    let modal_area = v_zones[1];

    // Button styles — the focused button gets a filled background.
    let confirm_style = if state.selected == 0 {
        Style::default()
            .fg(TEXT_PRIMARY)
            .bg(BRAND)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(TEXT_MUTED).bg(Color::Rgb(39, 39, 42))
    };

    let cancel_style = if state.selected == 1 {
        Style::default()
            .fg(TEXT_PRIMARY)
            .bg(Color::Rgb(63, 63, 70))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(TEXT_MUTED).bg(Color::Rgb(39, 39, 42))
    };

    let content: Vec<Line<'static>> = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "⚠  This action cannot be undone.",
                Style::default()
                    .fg(BRAND_LIGHT)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(message.to_string(), Style::default().fg(TEXT_PRIMARY)),
        ]),
        Line::from(""),
        Line::from(""),
        Line::from(""),
        // Buttons row.
        Line::from(vec![
            Span::raw("  "),
            Span::styled("  Confirm  ", confirm_style),
            Span::raw("    "),
            Span::styled("  Cancel  ", cancel_style),
        ]),
        Line::from(""),
    ];

    let block = Block::default()
        .title(Span::styled(
            " ⚠  Danger Zone ",
            Style::default().fg(BRAND).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BRAND))
        .style(Style::default().bg(BG_SURFACE));

    frame.render_widget(Paragraph::new(content).block(block), modal_area);
}

// ── Footer ────────────────────────────────────────────────────────────────────

fn render_footer(frame: &mut ratatui::Frame, area: ratatui::layout::Rect) {
    let hints: &[(&str, &str)] = &[
        ("y", "Confirm"),
        ("n / Esc", "Cancel"),
        ("← / →", "Switch"),
        ("Enter", "Select"),
    ];

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

    frame.render_widget(
        Paragraph::new(Line::from(spans))
            .alignment(Alignment::Center)
            .style(Style::default().bg(BG_SURFACE)),
        area,
    );
}
