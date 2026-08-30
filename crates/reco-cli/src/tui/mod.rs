mod state;

pub use state::{AiTui, TuiAction};

use std::io::{self, stdout};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Terminal;
use reco_core::{format_gib, CatalogSource, HardwareProfile, Recommendation};

const MAUVE: Color = Color::Rgb(203, 166, 247);
const CYAN: Color = Color::Rgb(137, 220, 235);
const PEACH: Color = Color::Rgb(250, 179, 135);
const DIM: Color = Color::Rgb(108, 112, 134);
const TEXT: Color = Color::Rgb(205, 214, 244);
const SURFACE: Color = Color::Rgb(49, 50, 68);

pub fn run(
    profile: &HardwareProfile,
    recs: Vec<Recommendation>,
    source: CatalogSource,
) -> io::Result<Option<Recommendation>> {
    if recs.is_empty() {
        return Ok(None);
    }

    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut state = AiTui::new(recs);
    let result = loop {
        terminal.draw(|frame| draw(frame, profile, &state, source))?;
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        let action = match key.code {
            KeyCode::Up => {
                state.up();
                TuiAction::None
            }
            KeyCode::Down => {
                state.down();
                TuiAction::None
            }
            KeyCode::Enter => {
                if state.searching {
                    state.handle_char('\n')
                } else {
                    TuiAction::Confirm
                }
            }
            KeyCode::Esc => {
                if state.searching {
                    state.cancel_search();
                    TuiAction::None
                } else {
                    TuiAction::Quit
                }
            }
            KeyCode::Backspace => {
                state.backspace();
                TuiAction::None
            }
            KeyCode::Char(ch) => state.handle_char(ch),
            _ => TuiAction::None,
        };
        match action {
            TuiAction::Confirm => break state.current().cloned(),
            TuiAction::Quit => break None,
            TuiAction::None => {}
        }
    };

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(result)
}

fn draw(
    frame: &mut ratatui::Frame<'_>,
    profile: &HardwareProfile,
    state: &AiTui,
    source: CatalogSource,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(6),
            Constraint::Length(3),
        ])
        .split(frame.area());

    let gpu = profile
        .gpus
        .first()
        .map(|g| {
            let vram = g.vram_bytes.map(format_gib).unwrap_or_else(|| "?".into());
            format!("{} · {vram}", g.name)
        })
        .unwrap_or_else(|| "CPU only".into());

    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                " Reco AI  ",
                Style::default().fg(MAUVE).add_modifier(Modifier::BOLD),
            ),
            Span::styled(gpu, Style::default().fg(CYAN)),
        ]),
        Line::from(Span::styled(
            " ↑↓ navegar   enter descargar   / buscar   q salir",
            Style::default().fg(DIM),
        )),
    ])
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(DIM)),
    );
    frame.render_widget(header, chunks[0]);

    let items: Vec<ListItem> = state
        .visible()
        .map(|(pos, rec)| {
            let selected = pos == state.selected_index();
            let marker = if selected { "›" } else { " " };
            let title = format!(
                "{marker} {:<42}  {:<8}  {:>7}  {:>5.1}",
                truncate(&rec.repo_id, 42),
                rec.quant.label(),
                format_gib(rec.size_bytes),
                rec.total
            );
            let meta = format!("    {}", rec.why);
            let style = if selected {
                Style::default().fg(PEACH).bg(SURFACE)
            } else {
                Style::default().fg(TEXT)
            };
            ListItem::new(vec![
                Line::from(Span::styled(title, style.add_modifier(Modifier::BOLD))),
                Line::from(Span::styled(
                    meta,
                    Style::default()
                        .fg(DIM)
                        .bg(if selected { SURFACE } else { Color::Reset }),
                )),
            ])
        })
        .collect();

    let list = List::new(items).block(Block::default().borders(Borders::NONE));
    frame.render_widget(list, chunks[1]);

    let source_label = match source {
        CatalogSource::HuggingFace => "Hugging Face",
        CatalogSource::Cache => "caché local",
        CatalogSource::Seed => "semilla",
    };
    let count = state.visible().count();
    let search = if state.searching || !state.query().is_empty() {
        format!(
            "  /{}{}",
            state.query(),
            if state.searching { "█" } else { "" }
        )
    } else {
        String::new()
    };
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(
            format!(" {count} modelos  ·  {source_label}  ·  40/20/20/20"),
            Style::default().fg(DIM),
        ),
        Span::styled(search, Style::default().fg(CYAN)),
    ]))
    .block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(DIM)),
    );
    frame.render_widget(footer, chunks[2]);
}

fn truncate(text: &str, max: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max {
        return text.to_string();
    }
    chars.iter().take(max.saturating_sub(1)).collect::<String>() + "…"
}
