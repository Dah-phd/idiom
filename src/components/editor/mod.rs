mod file;
pub use file::{CursorPosition, Offset};

use crate::configs::{EditorAction, EditorConfigs, EditorKeyMap, FileType};
use crate::lsp::{LSPMessage, LSP};
use crossterm::event::KeyEvent;
use file::Editor;
use serde_json::Value;
use std::collections::{hash_map::Entry, HashMap};
use std::path::PathBuf;
use tui::layout::{Constraint, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::{ListState, Tabs};
use tui::{backend::Backend, Frame};

pub struct EditorState {
    pub editors: Vec<Editor>,
    pub state: ListState,
    base_config: EditorConfigs,
    key_map: EditorKeyMap,
    lsp_servers: HashMap<FileType, LSP>,
    noifications: HashMap<FileType, Vec<LSPMessage>>,
    diagnostics: HashMap<PathBuf, Value>,
}

impl EditorState {
    pub fn new(key_map: EditorKeyMap) -> Self {
        Self {
            editors: Vec::default(),
            state: ListState::default(),
            base_config: EditorConfigs::new(),
            key_map,
            lsp_servers: HashMap::new(),
            noifications: HashMap::new(),
            diagnostics: HashMap::new(),
        }
    }

    pub fn render(&mut self, frame: &mut Frame<impl Backend>, area: Rect) {
        let layout = Layout::default()
            .constraints(vec![Constraint::Percentage(4), Constraint::Min(2)])
            .split(area);
        if let Some(editor_id) = self.state.selected() {
            if let Some(file) = self.editors.get_mut(editor_id) {
                file.set_max_rows(layout[1].bottom());
                let cursor_x_offset = 1 + file.cursor.char;
                let cursor_y_offset = file.cursor.line - file.at_line;
                let (digits_offset, editor_content) = file.get_list_widget();
                frame.set_cursor(
                    layout[1].x + (cursor_x_offset + digits_offset) as u16,
                    layout[1].y + cursor_y_offset as u16,
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

    pub async fn new_from(&mut self, file_path: PathBuf) {
        for (idx, file) in self.editors.iter().enumerate() {
            if file_path == file.path {
                self.state.select(Some(idx));
                return;
            }
        }
        if let Ok(opened_file) = Editor::from_path(file_path, self.base_config.clone()) {
            if let Entry::Vacant(e) = self.lsp_servers.entry(opened_file.file_type) {
                if let Ok(lsp) = LSP::from(&opened_file.file_type).await {
                    e.insert(lsp);
                }
            }
            self.state.select(Some(self.editors.len()));
            self.editors.push(opened_file);
        }
    }

    pub fn drain_notifications(&mut self) {
        for (file_type, lsp) in self.lsp_servers.iter_mut() {
            if let Ok(mut notifications) = lsp.notifications.try_lock() {
                if notifications.is_empty() {
                    continue;
                }
                self.noifications.insert(*file_type, notifications.drain(..).collect());
            }
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
                    EditorAction::JumpLeftSelect => editor.jump_left_select(),
                    EditorAction::JumpRight => editor.jump_right(),
                    EditorAction::JumpRightSelect => editor.jump_right_select(),
                    EditorAction::EndOfLine => editor.end_of_line(),
                    EditorAction::EndOfFile => editor.end_of_file(),
                    EditorAction::StartOfLine => editor.start_of_line(),
                    EditorAction::StartOfFile => editor.start_of_file(),
                    EditorAction::Cut => editor.cut(),
                    EditorAction::Copy => editor.copy(),
                    EditorAction::Paste => editor.paste(),
                    EditorAction::Undo => editor.undo(),
                    EditorAction::Redo => editor.redo(),
                    EditorAction::Save => editor.save(),
                    EditorAction::Close => self.close_active(),
                }
                return true;
            }
        }
        false
    }

    fn close_active(&mut self) {
        let path = if let Some(editor) = self.get_active() {
            editor.path.clone()
        } else {
            return;
        };
        self.close(&path)
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

    pub fn refresh_cfg(&mut self, new_key_map: EditorKeyMap) {
        self.key_map = new_key_map;
        self.base_config.refresh();
        for editor in self.editors.iter_mut() {
            editor.refresh_cfg(&self.base_config)
        }
    }
}

fn try_file_to_tab(file: &Editor) -> Option<Spans> {
    file.path
        .as_os_str()
        .to_str()
        .map(|t| Spans::from(Span::styled(t, Style::default().fg(Color::Green))))
}
