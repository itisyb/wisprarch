use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    symbols::border,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use std::io;

use super::theme::Theme;
use crate::models::{ModelRegistry, ModelStatus};

struct App {
    theme: Theme,
    registry: ModelRegistry,
    selected_index: usize,
    list_state: ListState,
    should_quit: bool,
}

impl App {
    fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            theme: Theme::catppuccin_mocha(),
            registry: ModelRegistry::new(),
            selected_index: 0,
            list_state,
            should_quit: false,
        }
    }

    fn next(&mut self) {
        let len = self.registry.models.len() + 2;
        self.selected_index = (self.selected_index + 1) % len;
        self.list_state.select(Some(self.selected_index));
    }

    fn previous(&mut self) {
        let len = self.registry.models.len() + 2;
        self.selected_index = if self.selected_index == 0 {
            len - 1
        } else {
            self.selected_index - 1
        };
        self.list_state.select(Some(self.selected_index));
    }
}

pub fn run_tui() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let res = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
                    KeyCode::Down | KeyCode::Char('j') => app.next(),
                    KeyCode::Up | KeyCode::Char('k') => app.previous(),
                    _ => {}
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

fn ui(frame: &mut Frame, app: &mut App) {
    let theme = app.theme.clone();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(frame.area());

    let title = Paragraph::new(" 󰓃 WisprArch ")
        .style(Style::default().fg(theme.accent))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_set(border::ROUNDED)
                .border_style(Style::default().fg(theme.border))
                .style(Style::default().bg(theme.bg)),
        );
    frame.render_widget(title, chunks[0]);

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(chunks[1]);

    render_model_list(frame, app, main_chunks[0], &theme);
    render_cloud_providers(frame, main_chunks[1], &theme);

    let actions = Paragraph::new(" [d] Download  [x] Delete  [Enter] Select  [q] Quit ")
        .style(Style::default().fg(theme.muted))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_set(border::ROUNDED)
                .border_style(Style::default().fg(theme.border))
                .title(" Actions ")
                .title_style(Style::default().fg(theme.accent)),
        );
    frame.render_widget(actions, chunks[2]);

    let status = Paragraph::new(" Ready │ Provider: Groq │ Lang: en ")
        .style(Style::default().fg(theme.muted).bg(theme.selection));
    frame.render_widget(status, chunks[3]);
}

fn render_model_list(frame: &mut Frame, app: &mut App, area: Rect, theme: &Theme) {
    let items: Vec<ListItem> = app
        .registry
        .models
        .iter()
        .map(|model| {
            let status = app.registry.get_model_status(&model.id);
            let status_icon = match status {
                ModelStatus::Downloaded => "󰄬",
                ModelStatus::NotDownloaded => "󰇚",
                ModelStatus::Downloading { .. } => "󰇘",
                ModelStatus::Corrupted => "󰅙",
            };

            let speed = "⚡".repeat(model.speed_rating as usize);
            let accuracy = "🎯".repeat(model.accuracy_rating as usize);

            let line = Line::from(vec![
                Span::styled(
                    format!(" {} ", status_icon),
                    Style::default().fg(theme.accent),
                ),
                Span::styled(format!("{:<30}", model.name), Style::default().fg(theme.fg)),
                Span::styled(
                    format!("{:>10}", format_size(model.size_bytes)),
                    Style::default().fg(theme.muted),
                ),
                Span::raw("  "),
                Span::styled(speed, Style::default()),
                Span::raw("  "),
                Span::styled(accuracy, Style::default()),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_set(border::ROUNDED)
                .border_style(Style::default().fg(theme.border))
                .title(" Local Models ")
                .title_style(Style::default().fg(theme.accent)),
        )
        .highlight_style(
            Style::default()
                .bg(theme.selection)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    frame.render_stateful_widget(list, area, &mut app.list_state);
}

fn render_cloud_providers(frame: &mut Frame, area: Rect, theme: &Theme) {
    let items = vec![
        ListItem::new(Line::from(vec![
            Span::styled(" 󰄬 ", Style::default().fg(theme.success)),
            Span::styled("Groq Cloud", Style::default().fg(theme.fg)),
            Span::styled(
                "  (whisper-large-v3-turbo)",
                Style::default().fg(theme.muted),
            ),
            Span::styled("  󰌆 API Key", Style::default().fg(theme.success)),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("   ", Style::default()),
            Span::styled("OpenAI Cloud", Style::default().fg(theme.fg)),
            Span::styled("  (whisper-1)", Style::default().fg(theme.muted)),
            Span::styled("  󰌆 API Key", Style::default().fg(theme.muted)),
        ])),
    ];

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_set(border::ROUNDED)
            .border_style(Style::default().fg(theme.border))
            .title(" Cloud Providers ")
            .title_style(Style::default().fg(theme.accent)),
    );

    frame.render_widget(list, area);
}

fn format_size(bytes: u64) -> String {
    const GB: u64 = 1_000_000_000;
    const MB: u64 = 1_000_000;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else {
        format!("{:.0} MB", bytes as f64 / MB as f64)
    }
}
