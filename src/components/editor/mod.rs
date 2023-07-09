mod cursor;
mod file;

use crate::messages::{EditorAction, EditorKeyMap};
use crate::syntax::Lexer;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use file::Editor;
use std::path::PathBuf;
use tui::layout::{Constraint, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::{List, ListItem, ListState, Tabs};
use tui::{backend::Backend, Frame};

pub struct EditorState {
    pub editors: Vec<Editor>,
    pub state: ListState,
    key_map: EditorKeyMap,
}

impl EditorState {
    pub fn new(key_map: EditorKeyMap) -> Self {
        Self {
            editors: Vec::default(),
            state: ListState::default(),
            key_map,
        }
    }

    pub fn len(&self) -> usize {
        self.editors.len()
    }

    pub fn render(&mut self, frame: &mut Frame<impl Backend>, area: Rect) {
        let layout = Layout::default()
            .constraints(vec![Constraint::Percentage(4), Constraint::Min(2)])
            .split(area);
        if let Some(editor_id) = self.state.selected() {
            if let Some(file) = self.editors.get_mut(editor_id) {
                file.cursor.max_rows = layout[1].bottom();
                let mut linter = Lexer::from_type(&file.file_type);
                if let Some(range) = file.cursor.selected.get() {
                    linter.select(range);
                }
                let max_digits = linter.line_number_max_digits(&file.content);
                let editor_content = List::new(
                    file.content[file.cursor.at_line..]
                        .iter()
                        .enumerate()
                        .map(|(idx, code_line)| linter.syntax_spans(idx + file.cursor.at_line, code_line))
                        .collect::<Vec<ListItem>>(),
                );

                frame.set_cursor(
                    layout[1].x + 1 + (file.cursor.char + max_digits) as u16,
                    layout[1].y + (file.cursor.line - file.cursor.at_line) as u16,
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
        let action = self.key_map.map(key);
        if let Some(editor) = self.get_active() {
            if let Some(action) = action {
                match action {
                    EditorAction::Char(ch) => editor.push(ch),
                    EditorAction::NewLine => editor.new_line(),
                    EditorAction::Indent => editor.indent(),
                    EditorAction::Backspace => editor.backspace(),
                    EditorAction::Delete => editor.del(),
                    EditorAction::IndentStart => editor.indent_start(),
                    EditorAction::Unintent => editor.unindent(),
                    EditorAction::Up => editor.up(),
                    EditorAction::Down => editor.down(),
                    EditorAction::Left => editor.left(),
                    EditorAction::Right => editor.right(),
                    EditorAction::SelectUp => editor.select_up(),
                    EditorAction::SelectDown => editor.select_down(),
                    EditorAction::SelectLeft => editor.select_left(),
                    EditorAction::SelectRight => editor.select_right(),
                    EditorAction::ScrollUp => editor.scroll_up(),
                    EditorAction::ScrollDown => editor.scroll_down(),
                    EditorAction::SwapUp => editor.swap_up(),
                    EditorAction::SwapDown => editor.swap_down(),
                    EditorAction::JumpLeft => editor.jump_left(),
                    EditorAction::JumpRight => editor.jump_right(),
                    EditorAction::Cut => editor.cut(),
                    EditorAction::Copy => editor.copy(),
                    EditorAction::Paste => editor.paste(),
                    EditorAction::Refresh => self.refresh(),
                }
                return true;
            }
        }
        false
    }

    pub fn close(&mut self, path: &PathBuf) {
        self.editors
            .retain(|editor| !editor.path.starts_with(path) && &editor.path != path)
    }

    pub fn are_updates_saved(&self) -> bool {
        for editor in self.editors.iter() {
            if !editor.is_saved() {
                return false;
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

    fn refresh(&mut self) {
        for editor in self.editors.iter_mut() {
            editor.configs.refresh()
        }
    }
}

fn try_file_to_tab(file: &Editor) -> Option<Spans> {
    file.path
        .as_os_str()
        .to_str()
        .map(|t| Spans::from(Span::styled(t, Style::default().fg(Color::Green))))
}
