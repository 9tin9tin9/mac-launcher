use crate::backend::LauncherResult;
use crossterm::{
    event::{read, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    error::Error,
    io::{self, Stdout},
};
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Terminal,
};

// TODO: use stateful list
pub struct App {
    running: bool,
    terminal: Terminal<CrosstermBackend<Stdout>>,
    query: String,
    prompt: String,
    cursor_index: usize,
    list_len: usize,
    list_state: ListState,
    completion: bool,
}

impl App {
    pub fn init(prompt: &str) -> Result<App, io::Error> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(App {
            running: true,
            terminal,
            query: String::new(),
            prompt: String::from(prompt),
            cursor_index: 0,
            list_len: 0,
            list_state: ListState::default(),
            completion: false,
        })
    }

    pub fn update<'a>(&'a mut self, list: &'a [LauncherResult]) -> Result<&'a mut App, io::Error> {
        self.list_len = list.len();
        self.select_first_item();
        self.terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
                .split(f.size());
            // input field
            let block = Block::default().borders(Borders::ALL);
            let text = self.prompt.clone() + &self.query;
            let input_field = Text::from(Span::from(if self.completion {
                list[self.list_state.selected().unwrap()].get_string()
            } else {
                text
            }));
            let paragraph = Paragraph::new(input_field).block(block);
            f.render_widget(paragraph, chunks[0]);
            if self.completion {
                f.set_cursor(1, 1);
            } else {
                f.set_cursor(1 + self.prompt.len() as u16 + self.cursor_index as u16, 1);
            }

            // search result
            let items = list
                .iter()
                .map(|r| ListItem::new(Span::from(r.get_string())))
                .collect::<Vec<ListItem>>();
            let items = List::new(items)
                .block(Block::default().borders(Borders::ALL))
                .highlight_style(
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol(">> ");
            f.render_stateful_widget(items, chunks[1], &mut self.list_state);

            // not completion
            self.completion = false;
        })?;
        Ok(self)
    }

    pub fn wait_input(&mut self, index: &mut Option<usize>) -> Result<bool, Box<dyn Error>> {
        loop {
            match read()? {
                Event::Key(KeyEvent {
                    code,
                    modifiers,
                    kind: KeyEventKind::Press | KeyEventKind::Repeat,
                    state: _,
                }) => {
                    if code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
                        return Ok(true);
                    }
                    macro_rules! move_selection {
                        ($list_len:expr, $state:expr, $i:expr, $dir:expr) => {
                            if $list_len > 0 {
                                $state.select(if let Some(i) = $state.selected() {
                                    let i = i as i64 + $dir;
                                    let i = if i < 0 {
                                        $list_len - 1
                                    } else {
                                        i as usize % $list_len
                                    };
                                    Some(i)
                                } else {
                                    None
                                })
                            }
                        };
                    }
                    match code {
                        KeyCode::Char(ch) => {
                            if self.cursor_index == self.query.len() {
                                self.query.push(ch);
                            } else {
                                self.query.insert(self.cursor_index, ch);
                            }
                            self.cursor_index += 1;
                            return Ok(false);
                        }
                        KeyCode::Backspace | KeyCode::Delete => {
                            if self.cursor_index > 0 {
                                self.query = self.query[0..self.cursor_index - 1].to_string()
                                    + &self.query[self.cursor_index..];
                                self.cursor_index -= 1;
                            }
                            return Ok(false);
                        }
                        KeyCode::Up => {
                            move_selection!(self.list_len, self.list_state, i, -1);
                            return Ok(false);
                        }
                        KeyCode::Down => {
                            move_selection!(self.list_len, self.list_state, i, 1);
                            return Ok(false);
                        }
                        KeyCode::Left => {
                            if self.cursor_index > 0 {
                                self.cursor_index -= 1;
                            }
                            return Ok(false);
                        }
                        KeyCode::Right => {
                            if self.cursor_index < self.query.len() {
                                self.cursor_index += 1;
                            }
                            return Ok(false);
                        }
                        KeyCode::Enter => {
                            *index = self.list_state.selected();
                            if let None = index {
                                return Ok(false);
                            } else {
                                return Ok(true);
                            }
                        }
                        KeyCode::Tab => {
                            self.completion = true && self.list_len > 0;
                            move_selection!(self.list_len, self.list_state, i, 1);
                            return Ok(false);
                        }
                        _ => continue,
                    }
                }
                _ => {}
            }
        }
    }

    pub fn exit(&mut self) {
        if self.running {
            disable_raw_mode().unwrap();
            execute!(self.terminal.backend_mut(), LeaveAlternateScreen,).unwrap();
            self.terminal.show_cursor().unwrap();
            self.running = false
        }
    }

    pub fn get_query(&self) -> &str {
        return &self.query;
    }

    pub fn set_prompt(&mut self, prompt: &str) -> &mut App {
        self.prompt = prompt.to_string();
        self
    }

    fn select_first_item(&mut self) {
        if self.list_len > 0 {
            if let None = self.list_state.selected() {
                self.list_state.select(Some(0));
            }
        } else {
            self.list_state.select(None);
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.exit()
    }
}
