mod state;

pub use state::PruebaSession;

use std::io::{self, stdout};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Terminal;
use reco_core::chat::ChatRole;
use reco_core::infer::PickedEngine;
use reco_core::store::ChatStore;
use reco_core::Recommendation;

const MAUVE: Color = Color::Rgb(203, 166, 247);
const GREEN: Color = Color::Rgb(166, 227, 161);
const DIM: Color = Color::Rgb(108, 112, 134);
const TEXT: Color = Color::Rgb(205, 214, 244);
const SURFACE: Color = Color::Rgb(49, 50, 68);

pub fn run(store: &ChatStore, rec: &Recommendation, picked: PickedEngine) -> io::Result<()> {
    let mut session = PruebaSession::open(store, rec, picked).map_err(io::Error::other)?;

    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    loop {
        terminal.draw(|frame| draw(frame, &session))?;
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        match key.code {
            KeyCode::Esc => {
                if session.show_help {
                    session.show_help = false;
                } else {
                    break;
                }
            }
            KeyCode::Enter => {
                if let Err(err) = session.submit(store) {
                    session.status = err;
                }
            }
            KeyCode::PageUp => session.page_up(),
            KeyCode::PageDown => session.page_down(),
            KeyCode::Backspace => session.backspace(),
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if let Err(err) = session.new_chat(store) {
                    session.status = err;
                }
            }
            KeyCode::Char('?') if session.input.is_empty() => session.toggle_help(),
            KeyCode::Char(ch) => session.type_char(ch),
            _ => {}
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn draw(frame: &mut ratatui::Frame<'_>, session: &PruebaSession) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(6),
            Constraint::Length(3),
        ])
        .split(frame.area());

    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            " Prueba  ",
            Style::default().fg(MAUVE).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(
                "{}  ·  {}  ·  {}",
                session.repo_id, session.filename, session.engine_label
            ),
            Style::default().fg(DIM),
        ),
    ]))
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(DIM)),
    );
    frame.render_widget(header, chunks[0]);

    let mut lines = Vec::new();
    for msg in &session.messages {
        let (who, color) = match msg.role {
            ChatRole::User => ("tú", TEXT),
            ChatRole::Assistant => ("reco", GREEN),
            ChatRole::System => ("sys", DIM),
        };
        lines.push(Line::from(Span::styled(
            who,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )));
        for part in wrap_text(&msg.content, 88) {
            lines.push(Line::from(Span::styled(part, Style::default().fg(color))));
        }
        lines.push(Line::from(""));
    }
    let height = chunks[1].height as usize;
    let end = lines.len().saturating_sub(session.offset);
    let start = end.saturating_sub(height.max(1));
    let history = Paragraph::new(lines[start..end].to_vec()).wrap(Wrap { trim: false });
    frame.render_widget(history, chunks[1]);
    if session.show_help {
        let help = Paragraph::new(vec![
            Line::from(Span::styled(
                " Prueba",
                Style::default().fg(MAUVE).add_modifier(Modifier::BOLD),
            )),
            Line::from(" enter        enviar"),
            Line::from(" RePág/AvPág  historial"),
            Line::from(" Ctrl+n       conversación nueva"),
            Line::from(" ?            esta ayuda"),
            Line::from(" esc          cerrar"),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(MAUVE)),
        );
        let w = 36u16;
        let h = 9u16;
        let area = ratatui::layout::Rect {
            x: frame.area().x + frame.area().width.saturating_sub(w) / 2,
            y: frame.area().y + frame.area().height.saturating_sub(h) / 2,
            width: w,
            height: h,
        };
        frame.render_widget(help, area);
    }

    let input = Paragraph::new(Line::from(vec![
        Span::styled(" › ", Style::default().fg(MAUVE)),
        Span::styled(session.input.clone(), Style::default().fg(TEXT)),
        Span::styled("█", Style::default().fg(MAUVE)),
    ]))
    .block(
        Block::default()
            .borders(Borders::TOP)
            .title(Span::styled(
                format!(" {} ", session.status),
                Style::default().fg(DIM),
            ))
            .border_style(Style::default().fg(SURFACE)),
    );
    frame.render_widget(input, chunks[2]);
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for raw in text.lines() {
        if raw.is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut current = String::new();
        for word in raw.split_whitespace() {
            if current.is_empty() {
                current = word.to_string();
            } else if current.len() + 1 + word.len() <= width {
                current.push(' ');
                current.push_str(word);
            } else {
                lines.push(current);
                current = word.to_string();
            }
        }
        if !current.is_empty() {
            lines.push(current);
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}
