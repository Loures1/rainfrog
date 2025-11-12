mod editor;

use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use ratatui::DefaultTerminal;

use crate::editor::Editor;

fn main() -> Result<()> {
  color_eyre::install()?;
  let terminal = ratatui::init();
  let result = run(terminal);
  ratatui::restore();
  result
}

fn run(mut terminal: DefaultTerminal) -> Result<()> {
  let mut editor = Editor::default();
  editor.set_boxes(&terminal.get_frame().area());
  editor.lines.push(vec!['o', 'l', 'a', ',']);
  editor.lines.push(vec!['l', 'u', 'c', 'a', 's', ' ', 'l', 'o', 'u', 'r', 'e', 's']);
  editor.lines.push(vec!['T', 'e', 'x', 't']);
  editor.lines.push(vec!['E', 'x', 'a', 'm', 'p', 'l', 'e']);
  editor.lines.push(vec!['T', 'e', 'x', 't']);

  loop {
    terminal.draw(|f| {
      let editor = editor.clone();
      f.render_widget(editor, f.area());
    })?;
    let event = event::read()?;
    if matches!(event, Event::Key(KeyEvent { code: KeyCode::Esc, .. })) {
      break Ok(());
    }
    editor.input(event);
  }
}
