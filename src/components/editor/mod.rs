mod file;
mod linter;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use file::Editor;
use linter::{Linter, RustSyntax};
use std::path::PathBuf;
use tui::layout::{Constraint, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::{List, ListItem, ListState, Tabs};
use tui::{backend::Backend, Frame};

#[derive(Default)]
pub struct EditorState {
    pub editors: Vec<Editor>,
    pub state: ListState,
}

impl EditorState {
    pub fn render(&mut self, frame: &mut Frame<impl Backend>, area: Rect) {
        let layout = Layout::default()
            .constraints(vec![Constraint::Percentage(4), Constraint::Min(2)])
            .split(area);
        if let Some(editor_id) = self.state.selected() {
            if let Some(file) = self.editors.get_mut(editor_id) {
                file.max_rows = layout[1].bottom();
                let max_digits = (file.content.len().ilog10() + 1) as usize;
                let mut linter = RustSyntax::default();
                let editor_content = List::new(
                    file.content[file.at_line..]
                        .iter()
                        .enumerate()
                        .map(|(idx, content)| linter.linter(idx + file.at_line, content, max_digits))
                        .collect::<Vec<ListItem>>(),
                );
                frame.set_cursor(
                    layout[1].x + 1 + (file.cursor.1 + max_digits) as u16,
                    layout[1].y + (file.cursor.0 - file.at_line) as u16,
                );
                frame.render_widget(editor_content, layout[1]);

                let mut titles_unordered: Vec<_> = self.editors.iter().flat_map(try_file_to_tab).collect();
                let mut titles = titles_unordered.split_off(editor_id);
                titles.extend(titles_unordered);

                let tabs = Tabs::new(titles)
                    .style(Style::default().add_modifier(Modifier::UNDERLINED))
                    .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                    .select(0);

                frame.render_widget(tabs, layout[0]);
            }
        }
    }

    pub fn get_active(&mut self) -> Option<&mut Editor> {
        self.editors.get_mut(self.state.selected()?)
    }

    pub fn new_from(&mut self, file_path: PathBuf) {
        for (idx, file) in self.editors.iter().enumerate() {
            if file_path == file.path {
                self.state.select(Some(idx));
                return;
            }
        }
        if let Ok(opened_file) = Editor::from_path(file_path) {
            self.state.select(Some(self.editors.len()));
            self.editors.push(opened_file);
        }
    }

    pub fn map(&mut self, key: &KeyEvent) -> bool {
        if let Some(editor) = self.get_active() {
            match key.modifiers {
                KeyModifiers::CONTROL => match key.code {
                    KeyCode::Char(c) => match c {
                        '[' => editor.unindent(),
                        ']' => editor.indent(),
                        _ => return false,
                    },
                    _ => return false,
                },
                KeyModifiers::NONE => match key.code {
                    KeyCode::Up => {
                        if key.modifiers.contains(KeyModifiers::CONTROL) {
                            editor.scroll_up()
                        } else {
                            editor.navigate_up()
                        }
                    }
                    KeyCode::Down => {
                        if key.modifiers.contains(KeyModifiers::CONTROL) {
                            editor.scroll_down()
                        } else {
                            editor.navigate_down()
                        }
                    }
                    KeyCode::Left => editor.navigate_left(),
                    KeyCode::Right => editor.navigate_right(),
                    KeyCode::Char(c) => editor.push_str(c.to_string().as_str()),
                    KeyCode::Backspace => editor.backspace(),
                    KeyCode::Enter => editor.new_line(),
                    KeyCode::Tab => editor.indent(),
                    KeyCode::Delete => editor.del(),
                    _ => return false,
                },
                KeyModifiers::SHIFT => {}
                KeyModifiers::ALT => {}
                _ => return false,
            }
        }
        true
    }

    pub fn save(&mut self) {
        if let Some(editor) = self.get_active() {
            editor.save()
        }
    }

    pub fn save_all(&mut self) {
        for editor in self.editors.iter_mut() {
            editor.save()
        }
    }
}

fn try_file_to_tab(file: &Editor) -> Option<Spans> {
    file.path
        .as_os_str()
        .to_str()
        .map(|t| Spans::from(Span::styled(t, Style::default().fg(Color::Green))))
}
