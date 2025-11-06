use color_eyre::{eyre::Result, owo_colors::OwoColorize};
use crossterm::event::{KeyEvent, MouseEvent, MouseEventKind};
use ratatui::{
  prelude::*,
  widgets::{Block, Borders, Clear, Paragraph},
};
use tokio::sync::mpsc::UnboundedSender;
use tui_textarea::{Input, Key, TextArea};

use super::{Component, Frame};
use crate::{
  action::Action,
  app::AppState,
  config::Config,
  database::get_keywords,
  focus::Focus,
  tui::Event,
  vim::{Mode, Transition, Vim},
};

fn keyword_regex() -> String {
  format!("(?i)(^|[^a-zA-Z0-9\'\"`._]+)({})($|[^a-zA-Z0-9\'\"`._]+)", get_keywords().join("|"))
}

#[derive(Default)]
pub struct Editor<'a> {
  command_tx: Option<UnboundedSender<Action>>,
  config: Config,
  textarea: TextArea<'a>,
  auto_complete: AutoComplete,
  vim_state: Vim,
  cursor_style: Style,
  last_query_duration: Option<chrono::Duration>,
}

impl Editor<'_> {
  pub fn new() -> Self {
    let mut textarea = TextArea::default();
    textarea.set_search_pattern(keyword_regex()).unwrap();
    Editor {
      command_tx: None,
      config: Config::default(),
      textarea,
      auto_complete: AutoComplete::default(),
      vim_state: Vim::new(Mode::Normal),
      cursor_style: Mode::Normal.cursor_style(),
      last_query_duration: None,
    }
  }

  pub fn transition_vim_state(&mut self, input: Input, app_state: &AppState) -> Result<()> {
    match input {
      Input { key: Key::Enter, alt: true, .. } | Input { key: Key::Enter, ctrl: true, .. } => {
        if !app_state.query_task_running
          && let Some(sender) = &self.command_tx
        {
          sender.send(Action::Query(self.textarea.lines().to_vec(), false, false))?;
          self.vim_state = Vim::new(Mode::Normal);
          self.vim_state.register_action_handler(self.command_tx.clone())?;
          self.cursor_style = Mode::Normal.cursor_style();
        }
      },
      Input { key: Key::Tab, shift: false, .. } if self.vim_state.mode != Mode::Insert => {
        if let Some(sender) = &self.command_tx {
          sender.send(Action::CycleFocusForwards)?;
        }
      },
      Input { key: Key::Char('f'), ctrl: true, .. } if self.vim_state.mode != Mode::Insert => {
        if let Some(sender) = &self.command_tx {
          sender.send(Action::RequestSaveFavorite(self.textarea.lines().to_vec()))?;
        }
      },
      Input { key: Key::Char('f'), alt: true, .. } => {
        if let Some(sender) = &self.command_tx {
          sender.send(Action::RequestSaveFavorite(self.textarea.lines().to_vec()))?;
        }
      },
      Input { key: Key::Char('c'), ctrl: true, .. } if matches!(self.vim_state.mode, Mode::Normal) => {
        if let Some(sender) = &self.command_tx {
          sender.send(Action::Quit)?;
        }
      },
      Input { key: Key::Char('q'), .. } if matches!(self.vim_state.mode, Mode::Normal) => {
        if let Some(sender) = &self.command_tx {
          sender.send(Action::AbortQuery)?;
        }
      },
      _ => {
        let new_vim_state = self.vim_state.clone();

        if let Mode::Insert = self.vim_state.mode {
          self.auto_complete.active =
            input.key != Key::Backspace && input.key != Key::Char(' ') && input.key != Key::Esc;
        }
        self.vim_state = match new_vim_state.transition(input, &mut self.textarea) {
          Transition::Mode(mode) if new_vim_state.mode != mode => {
            self.cursor_style = mode.cursor_style();
            Vim::new(mode)
          },
          Transition::Nop | Transition::Mode(_) => new_vim_state,
          Transition::Pending(input) => new_vim_state.with_pending(input),
        };
        self.vim_state.register_action_handler(self.command_tx.clone())?;
      },
    };

    if let Mode::Insert = self.vim_state.mode {};
    Ok(())
  }
}

impl Component for Editor<'_> {
  fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
    self.vim_state.register_action_handler(self.command_tx.clone())?;
    self.command_tx = Some(tx);
    Ok(())
  }

  fn register_config_handler(&mut self, config: Config) -> Result<()> {
    self.config = config;
    Ok(())
  }

  fn handle_mouse_events(&mut self, mouse: MouseEvent, app_state: &AppState) -> Result<Option<Action>> {
    if app_state.focus != Focus::Editor {
      return Ok(None);
    }
    match mouse.kind {
      MouseEventKind::ScrollDown => {
        self.textarea.scroll((1, 0));
      },
      MouseEventKind::ScrollUp => {
        self.textarea.scroll((-1, 0));
      },
      MouseEventKind::ScrollLeft => {
        self.transition_vim_state(Input { key: Key::Char('h'), ctrl: false, alt: false, shift: false }, app_state)?;
      },
      MouseEventKind::ScrollRight => {
        self.transition_vim_state(Input { key: Key::Char('j'), ctrl: false, alt: false, shift: false }, app_state)?;
      },
      _ => {},
    };
    Ok(None)
  }

  fn handle_events(
    &mut self,
    event: Option<Event>,
    last_tick_key_events: Vec<KeyEvent>,
    app_state: &AppState,
  ) -> Result<Option<Action>> {
    if app_state.focus != Focus::Editor {
      return Ok(None);
    }
    if let Some(Event::Paste(text)) = event {
      self.textarea.insert_str(text);
    } else if let Some(Event::Mouse(event)) = event {
      self.handle_mouse_events(event, app_state).unwrap();
    } else if let Some(Event::Key(key)) = event {
      let input = Input::from(key);
      self.transition_vim_state(input, app_state)?;
    };
    Ok(None)
  }

  fn update(&mut self, action: Action, app_state: &AppState) -> Result<Option<Action>> {
    match action {
      Action::SubmitEditorQueryBypassParser => {
        if let Some(sender) = &self.command_tx {
          sender.send(Action::Query(self.textarea.lines().to_vec(), false, true))?;
        }
      },
      Action::SubmitEditorQuery => {
        if let Some(sender) = &self.command_tx {
          sender.send(Action::Query(self.textarea.lines().to_vec(), false, false))?;
        }
      },
      Action::QueryToEditor(lines) => {
        self.textarea = TextArea::from(lines.clone());
        self.textarea.set_search_pattern(keyword_regex()).unwrap();
      },
      Action::CopyData(data) => {
        self.textarea.set_yank_text(data);
      },
      _ => {},
    }
    Ok(None)
  }

  fn draw(&mut self, f: &mut Frame<'_>, area: Rect, app_state: &AppState) -> Result<()> {
    let focused = app_state.focus == Focus::Editor;

    if let Some(query_start) = app_state.last_query_start {
      self.last_query_duration = match app_state.last_query_end {
        Some(end) => Some(end.signed_duration_since(query_start)),
        None => Some(chrono::Utc::now().signed_duration_since(query_start)),
      };
    }

    let duration_string = self.last_query_duration.map_or("".to_string(), |d| {
      let seconds: f64 = (d.num_milliseconds()
        % std::cmp::max(1, d.num_minutes()).saturating_mul(60).saturating_mul(1000)) as f64
        / 1000_f64;
      format!(
        " {}{}:{}{:.3}s ",
        if d.num_minutes() < 10 { "0" } else { "" },
        d.num_minutes(),
        if seconds < 10.0 { "0" } else { "" },
        seconds
      )
    });
    let block = self
      .vim_state
      .mode
      .block()
      .border_style(if focused { Style::new().green() } else { Style::new().dim() })
      .title(Line::from(duration_string).right_aligned());

    self.textarea.set_cursor_style(self.cursor_style);
    self.textarea.set_block(block);
    self.textarea.set_line_number_style(if focused { Style::default().fg(Color::Yellow) } else { Style::new().dim() });
    self.textarea.set_cursor_line_style(Style::default().not_underlined());
    self.textarea.set_hard_tab_indent(false);
    self.textarea.set_tab_length(2);
    self.textarea.set_search_style(Style::default().fg(Color::Magenta).bold());

    let limit = area.width.saturating_sub(2) as f32 * 0.9;
    let rest = area.width.saturating_sub(2) as f32 * 0.1;

    let (row, col) = self.textarea.cursor();

    let mut frame_mode = FrameMode::Normal;

    if col >= limit as usize {
      frame_mode = FrameMode::Limit;
    }

    match frame_mode {
      FrameMode::Normal => (),
      FrameMode::Limit => {
        if col.saturating_sub(self.auto_complete.cursor) == 1 {
          self.textarea.insert_str(" ".repeat(rest as usize));
          self.auto_complete.limit = Limit::Delete;
        } else {
          match self.auto_complete.limit {
            Limit::Delete => {
              let mut num = 0;
              while num < rest as usize {
                self.textarea.delete_char();
                num += 1;
              }
              self.auto_complete.limit = Limit::Stop;
            },
            Limit::Stop => (),
          }
        }
      },
    };

    f.render_widget(&self.textarea, area);
    self.auto_complete.cursor = col;

    self.auto_complete.active = false;
    if self.auto_complete.active {
      let (x, y) = (area.x + 3, area.y + 2);
      let limit = area.width.saturating_sub(3) as usize;
      let (row, col) = self.textarea.cursor();
      let area = if col + 6 < limit {
        Rect::new(x + col as u16, y + row as u16, 6, 6)
      } else {
        Rect::new(area.width.saturating_add(area.x).saturating_sub(7), y + row as u16, 6, 6)
      };
      let p1 = Block::new().borders(Borders::all()).title("ola");

      f.render_widget(Clear, area);
      f.render_widget(p1, area);
    }

    Ok(())
  }
}

enum FrameMode {
  Normal,
  Limit,
}

#[derive(Default)]
enum Limit {
  #[default]
  Stop,
  Delete,
}

#[derive(Default)]
struct AutoComplete {
  limit: Limit,
  cursor: usize,
  active: bool,
  test: bool,
  test1: bool,
}
