use crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::{
  layout::{Position, Rect},
  widgets::Widget,
};

#[derive(Default, Debug, PartialEq, Clone)]
pub struct Editor {
  mode: Mode,
  outer_box: [(usize, usize); 2],
  inner_box: [(usize, usize); 2],
  greather_col: usize,
  cursor_image: (usize, usize),
  cursor_real: (usize, usize),
  pub lines: Vec<Vec<char>>,
}

impl Editor {
  pub fn set_boxes(&mut self, area: &Rect) {
    if let Some((x, y)) = self.outer_box.last_mut() {
      *x = area.width as usize;
      *y = area.height as usize;
    }

    if let Some((x, y)) = self.inner_box.last_mut() {
      *x = area.width.saturating_sub(8) as usize;
      *y = area.height.saturating_sub(8) as usize;
    }
  }

  pub fn input(&mut self, event: Event) {
    if let Some(key) = event.as_key_event() {
      match key {
        KeyEvent { code: KeyCode::Left, .. } => self.move_cursor(|(x, y)| (x.saturating_sub(1), y)),
        KeyEvent { code: KeyCode::Right, .. } => self.move_cursor(|(x, y)| (x.saturating_add(1), y)),
        KeyEvent { code: KeyCode::Up, .. } => self.move_cursor(|(x, y)| (x, y.saturating_sub(1))),
        KeyEvent { code: KeyCode::Down, .. } => self.move_cursor(|(x, y)| (x, y.saturating_add(1))),
        _ => (),
      };
    }
  }

  fn move_cursor<F>(&mut self, f: F)
  where
    F: FnOnce((usize, usize)) -> (usize, usize),
  {
    let (x, y) = f(self.cursor_real);

    if let Some(next_line) = self.lines.get(y)
      && let Some(current_line) = self.lines.get(self.cursor_real.1)
    {
      if current_line.len() > next_line.len() && next_line.get(x).is_none() {
        self.cursor_real = (next_line.len().saturating_sub(1), y);
        self.cursor_image = (x, y);
      } else if next_line.get(self.cursor_image.0).is_some() && self.cursor_image != self.cursor_real {
        let (x, _) = self.cursor_image;
        self.cursor_real = (x, y);
        self.cursor_image = self.cursor_real;
      } else if next_line.get(x).is_some() {
        self.cursor_real = (x, y);
        self.cursor_image = self.cursor_real;
      }
    }
  }
}

impl Widget for Editor {
  fn render(self, _area: Rect, buf: &mut ratatui::prelude::Buffer)
  where
    Self: Sized,
  {
    let x_start = self.outer_box[0].0;
    let x_end = self.outer_box[1].0;

    let y_start = self.outer_box[0].1;
    let y_end = self.outer_box[1].1;

    for (y, line) in self.lines[y_start..].iter().enumerate().take_while(|(p, _)| *p < y_end) {
      for (x, col) in line[x_start..].iter().enumerate().take_while(|(p, _)| *p < x_end) {
        if let Some(cell) = buf.cell_mut(Position::new(x as u16, y as u16)) {
          cell.set_char(*col);
        }
      }
    }

    let (x, y) = self.cursor_real;
    if let Some(cell) = buf.cell_mut(Position::new(x as u16, y as u16)) {
      cell.set_fg(ratatui::style::Color::Black);
      cell.set_bg(ratatui::style::Color::Gray);
    }
  }
}

#[derive(Default, Debug, PartialEq, Clone, Copy)]
enum Mode {
  #[default]
  Normal,
  _Insert,
}

#[cfg(test)]
mod test {
  use ratatui::layout::Rect;

  use crate::editor::Editor;

  #[test]
  fn set_boxes_works() {
    let mut editor = Editor::default();
    editor.set_boxes(&Rect::new(0, 0, 20, 30));

    let mut expected_editor = Editor::default();
    expected_editor.outer_box[1] = (20, 30);
    expected_editor.inner_box[1] = (12, 22);

    assert_eq!(expected_editor, editor)
  }

  #[test]
  fn real_cursor_position_when_next_line_hasnt_a_col_acess() {
    let mut editor = Editor::default();
    editor.lines.push(vec!['T', 'e', 'x', 't']);
    editor.lines.push(vec!['E', 'x', 'a', 'm', 'p', 'l', 'e']);
    editor.lines.push(vec!['T', 'e', 'x', 't']);

    editor.cursor_real = (6, 1);
    editor.move_cursor(|(x, y)| (x, y.saturating_sub(1)));
    assert_eq!(editor.cursor_real, (3, 0))
  }

  #[test]
  fn image_cursor_position_when_the_next_line_hasnt_a_col_acess() {
    let mut editor = Editor::default();
    editor.lines.push(vec!['T', 'e', 'x', 't']);
    editor.lines.push(vec!['E', 'x', 'a', 'm', 'p', 'l', 'e']);
    editor.lines.push(vec!['T', 'e', 'x', 't']);

    editor.cursor_real = (6, 1);
    editor.move_cursor(|(x, y)| (x, y.saturating_add(1)));
    assert_eq!(editor.cursor_image, (6, 2))
  }

  #[test]
  fn cursors_position_are_equal_when_the_current_line_has_a_col_acess_and_the_next_line_has_a_col_acess() {
    let mut editor = Editor::default();
    editor.lines.push(vec!['E', 'x', 'a', 'm', 'p', 'l', 'e']);
    editor.lines.push(vec!['T', 'e', 'x', 't']);
    editor.move_cursor(|(x, y)| (x, y.saturating_add(1)));
    assert_eq!(editor.cursor_real, editor.cursor_image)
  }

  #[test]
  fn cursors_position_are_equal_when_the_current_line_hasnt_a_acess_col_but_the_next_line_has_a_acess_col() {
    let mut editor = Editor::default();
    editor.lines.push(vec!['E', 'x', 'a', 'm', 'p', 'l', 'e']);
    editor.lines.push(vec!['T', 'e', 'x', 't']);
    editor.move_cursor(|(x, y)| (x.saturating_add(6), y.saturating_add(1)));
    editor.move_cursor(|(x, y)| (x, y.saturating_sub(1)));
    assert_eq!(editor.cursor_real, editor.cursor_image)
  }
}
