mod state;

pub use state::PruebaSession;

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
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Terminal;
use reco_core::chat::ChatRole;
use reco_core::store::ChatStore;
use reco_core::Recommendation;

const MAUVE: Color = Color::Rgb(203, 166, 247);
const GREEN: Color = Color::Rgb(166, 227, 161);
const DIM: Color = Color::Rgb(108, 112, 134);
const TEXT: Color = Color::Rgb(205, 214, 244);
const SURFACE: Color = Color::Rgb(49, 50, 68);

pub fn run(store: &ChatStore, rec: &Recommendation, demo: bool) -> io::Result<()> {
    let mut session = PruebaSession::echo(store, rec).map_err(io::Error::other)?;
    let _ = demo;

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
            KeyCode::Esc => break,
            KeyCode::Enter => {
                if let Err(err) = session.submit(store) {
                    session.status = err;
                }
            }
            KeyCode::Backspace => session.backspace(),
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
            format!("{}  ·  {}", session.repo_id, session.filename),
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
    let history = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(history, chunks[1]);

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
