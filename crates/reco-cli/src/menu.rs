//! Arrow-key command launcher. `reco` with no args opens this on a TTY.

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
use reco_catalog::DownloadedModel;
use reco_core::format_gib;

const MAUVE: Color = Color::Rgb(203, 166, 247);
const CYAN: Color = Color::Rgb(137, 220, 235);
const PEACH: Color = Color::Rgb(250, 179, 135);
const DIM: Color = Color::Rgb(108, 112, 134);
const TEXT: Color = Color::Rgb(205, 214, 244);
const SURFACE: Color = Color::Rgb(49, 50, 68);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Launch {
    Catalog,
    Models,
    Desktop,
    Chat,
    Run,
    Serve,
    Doctor,
    Setup,
    Hardware,
    Config,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Item {
    pub id: Launch,
    pub cmd: &'static str,
    pub hint: &'static str,
}

pub const ITEMS: &[Item] = &[
    Item {
        id: Launch::Catalog,
        cmd: "reco ai",
        hint: "catálogo de modelos que caben en esta máquina",
    },
    Item {
        id: Launch::Models,
        cmd: "reco models",
        hint: "GGUF ya descargados en este disco",
    },
    Item {
        id: Launch::Desktop,
        cmd: "reco desktop",
        hint: "ventana Prueba: catálogo + chat",
    },
    Item {
        id: Launch::Chat,
        cmd: "reco chat",
        hint: "chatear: elige un modelo con las flechas",
    },
    Item {
        id: Launch::Run,
        cmd: "reco run",
        hint: "descargar el que cabe y abrir el chat",
    },
    Item {
        id: Launch::Serve,
        cmd: "reco serve",
        hint: "esta máquina sirve las APIs",
    },
    Item {
        id: Launch::Doctor,
        cmd: "reco doctor",
        hint: "llama-cli, claves, caché, ventana",
    },
    Item {
        id: Launch::Setup,
        cmd: "reco setup",
        hint: "checklist de instalación",
    },
    Item {
        id: Launch::Hardware,
        cmd: "reco hw",
        hint: "CPU, RAM y GPU detectados",
    },
    Item {
        id: Launch::Config,
        cmd: "reco config",
        hint: "claves BYOK y ruta de llama-cli",
    },
];

#[derive(Debug, Clone)]
pub struct MenuState {
    selected: usize,
}

impl MenuState {
    pub fn new() -> Self {
        Self { selected: 0 }
    }

    pub fn current(&self) -> Item {
        ITEMS[self.selected]
    }

    pub fn up(&mut self) {
        if self.selected == 0 {
            self.selected = ITEMS.len() - 1;
        } else {
            self.selected -= 1;
        }
    }

    pub fn down(&mut self) {
        self.selected = (self.selected + 1) % ITEMS.len();
    }
}

impl Default for MenuState {
    fn default() -> Self {
        Self::new()
    }
}

/// Full-screen command menu. `None` = quit.
pub fn run(status: &str) -> io::Result<Option<Launch>> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut state = MenuState::new();
    let result = loop {
        terminal.draw(|frame| draw(frame, &state, status))?;
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => state.up(),
            KeyCode::Down | KeyCode::Char('j') => state.down(),
            KeyCode::Enter | KeyCode::Char('l') => break Some(state.current().id),
            KeyCode::Esc | KeyCode::Char('q') => break None,
            _ => {}
        }
    };

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(result)
}

fn draw(frame: &mut ratatui::Frame<'_>, state: &MenuState, status: &str) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .split(frame.area());

    let header = Paragraph::new(vec![
        Line::from(Span::styled(
            " Reco AI",
            Style::default().fg(MAUVE).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!(" {status}"),
            Style::default().fg(DIM),
        )),
    ])
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(DIM)),
    );
    frame.render_widget(header, chunks[0]);

    let items: Vec<ListItem> = ITEMS
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let selected = idx == state.selected;
            let marker = if selected { "›" } else { " " };
            let line = format!("{marker} {:<14}  {}", item.cmd, item.hint);
            let style = if selected {
                Style::default().fg(PEACH).bg(SURFACE).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(TEXT)
            };
            ListItem::new(Line::from(Span::styled(line, style)))
        })
        .collect();

    frame.render_widget(List::new(items), chunks[1]);

    let current = state.current();
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" ↑↓ j k  mover  ", Style::default().fg(DIM)),
        Span::styled("enter", Style::default().fg(CYAN)),
        Span::styled("  elegir  ", Style::default().fg(DIM)),
        Span::styled("q", Style::default().fg(CYAN)),
        Span::styled("  salir   ", Style::default().fg(DIM)),
        Span::styled(current.cmd, Style::default().fg(MAUVE)),
    ]))
    .block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(DIM)),
    );
    frame.render_widget(footer, chunks[2]);
}

/// Pick a downloaded GGUF with arrows. `None` = back.
pub fn pick_downloaded(models: &[DownloadedModel]) -> io::Result<Option<DownloadedModel>> {
    if models.is_empty() {
        return Ok(None);
    }
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut selected = 0usize;
    let result = loop {
        terminal.draw(|frame| draw_models(frame, models, selected))?;
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                selected = if selected == 0 {
                    models.len() - 1
                } else {
                    selected - 1
                };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                selected = (selected + 1) % models.len();
            }
            KeyCode::Enter | KeyCode::Char('l') => break Some(models[selected].clone()),
            KeyCode::Esc | KeyCode::Char('q') => break None,
            _ => {}
        }
    };

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(result)
}

fn draw_models(frame: &mut ratatui::Frame<'_>, models: &[DownloadedModel], selected: usize) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(4), Constraint::Length(2)])
        .split(frame.area());

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            " reco models  ·  elige uno",
            Style::default().fg(MAUVE).add_modifier(Modifier::BOLD),
        ))),
        chunks[0],
    );

    let items: Vec<ListItem> = models
        .iter()
        .enumerate()
        .map(|(idx, model)| {
            let on = idx == selected;
            let marker = if on { "›" } else { " " };
            let line = format!(
                "{marker} {}  {}  {}",
                model.repo_id,
                model.filename,
                format_gib(model.size_bytes)
            );
            let style = if on {
                Style::default().fg(PEACH).bg(SURFACE)
            } else {
                Style::default().fg(TEXT)
            };
            ListItem::new(Line::from(Span::styled(line, style)))
        })
        .collect();
    frame.render_widget(List::new(items), chunks[1]);
    frame.render_widget(
        Paragraph::new(Span::styled(
            " ↑↓  enter abrir chat  ·  q volver",
            Style::default().fg(DIM),
        )),
        chunks[2],
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrows_wrap_and_select_commands() {
        let mut state = MenuState::new();
        assert_eq!(state.current().cmd, "reco ai");
        state.up();
        assert_eq!(state.current().cmd, "reco config");
        state.down();
        assert_eq!(state.current().cmd, "reco ai");
        for _ in 0..4 {
            state.down();
        }
        assert_eq!(state.current().id, Launch::Run);
        assert_eq!(ITEMS.len(), 10);
    }

    #[test]
    fn menu_lists_the_product_commands() {
        let cmds: Vec<_> = ITEMS.iter().map(|i| i.cmd).collect();
        assert!(cmds.contains(&"reco models"));
        assert!(cmds.contains(&"reco desktop"));
        assert!(cmds.contains(&"reco doctor"));
        assert!(cmds.contains(&"reco setup"));
    }
}
