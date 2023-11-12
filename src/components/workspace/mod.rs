mod file;
use crate::configs::{EditorAction, EditorConfigs, EditorKeyMap, FileType, Mode};
use crate::events::Events;
use crate::lsp::LSP;
use anyhow::Result;
use crossterm::event::KeyEvent;
use file::Editor;
pub use file::{CursorPosition, DocStats, Offset, Select};
use lsp_types::WorkspaceEdit;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::prelude::CrosstermBackend;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{ListState, Tabs};
use ratatui::Frame;
use std::cell::RefCell;
use std::collections::{hash_map::Entry, HashMap};
use std::io::Stdout;
use std::path::PathBuf;
use std::rc::Rc;
use tokio::sync::Mutex;

type LSPPool = HashMap<FileType, Rc<Mutex<LSP>>>;

pub struct Workspace {
    pub editors: Vec<Editor>,
    pub state: ListState,
    events: Rc<RefCell<Events>>,
    base_config: EditorConfigs,
    key_map: EditorKeyMap,
    lsp_servers: LSPPool,
}

impl Workspace {
    pub fn new(key_map: EditorKeyMap, events: &Rc<RefCell<Events>>) -> Self {
        Self {
            editors: Vec::default(),
            state: ListState::default(),
            base_config: EditorConfigs::new(),
            events: Rc::clone(events),
            key_map,
            lsp_servers: HashMap::new(),
        }
    }

    pub fn render(&mut self, frame: &mut Frame<CrosstermBackend<&Stdout>>, screen: Rect) {
        let layout = Layout::default().constraints([Constraint::Length(1), Constraint::default()]).split(screen);
        if let Some(editor_id) = self.state.selected() {
            if let Some(file) = self.editors.get_mut(editor_id) {
                file.set_max_rows(layout[1].bottom());
                let cursor_x_offset = 1 + file.cursor.char;
                let cursor_y_offset = file.cursor.line - file.at_line;
                let (digits_offset, editor_content) = file.get_list_widget();
                let x_cursor = layout[1].x + (cursor_x_offset + digits_offset) as u16;
                let y_cursor = layout[1].y + cursor_y_offset as u16;

                frame.set_cursor(x_cursor, y_cursor);
                frame.render_widget(editor_content, layout[1]);
                file.lexer.render_modal_if_exist(frame, x_cursor, y_cursor);

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

    pub async fn map(&mut self, key: &KeyEvent, mode: &mut Mode) -> bool {
        if !matches!(mode, Mode::Insert) {
            return false;
        }
        let action = self.key_map.map(key);
        if let Some(editor) = self.get_active() {
            if let Some(action) = action {
                if editor.lexer.map_modal_if_exists(&action) {
                    return true;
                };
                match action {
                    EditorAction::Char(ch) => editor.push(ch).await,
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
                    EditorAction::Help => editor.hover().await,
                    EditorAction::Cut => editor.cut(),
                    EditorAction::Copy => editor.copy(),
                    EditorAction::Paste => editor.paste(),
                    EditorAction::Undo => editor.undo(),
                    EditorAction::Redo => editor.redo(),
                    EditorAction::Save => editor.save().await,
                    EditorAction::Close => {
                        self.close_active().await;
                        if self.state.selected().is_none() {
                            *mode = Mode::Select;
                        }
                    }
                }
                return true;
            }
        }
        false
    }

    pub fn get_stats(&self) -> Option<DocStats> {
        self.editors.get(self.state.selected()?).map(|editor| editor.get_stats())
    }

    pub fn tabs(&self) -> Vec<String> {
        self.editors.iter().map(|editor| editor.path.display().to_string()).collect()
    }

    pub fn get_active(&mut self) -> Option<&mut Editor> {
        self.editors.get_mut(self.state.selected()?)
    }

    pub async fn renames(&mut self, new_name: String) {
        if let Some(editor) = self.get_active() {
            editor.renames(new_name).await;
        }
    }

    pub async fn lexer_updates(&mut self) {
        if let Some(file) = self.get_active() {
            file.update_lsp().await;
        }
    }

    pub fn apply_edits(&mut self, edits: WorkspaceEdit) {
        if let Some(edits) = edits.changes {
            for (file_url, file_edits) in edits {
                if let Some(editor) = self.get_editor(file_url.path()) {
                    editor.apply_file_edits(file_edits);
                } else if let Ok(mut editor) = self.build_basic_editor(PathBuf::from(file_url.path())) {
                    editor.apply_file_edits(file_edits);
                }
            }
        }
    }

    fn get_editor<T: Into<PathBuf>>(&mut self, path: T) -> Option<&mut Editor> {
        let path: PathBuf = path.into();
        self.editors.iter_mut().find(|editor| editor.path == path)
    }

    fn build_basic_editor(&mut self, file_path: PathBuf) -> Result<Editor> {
        Ok(Editor::from_path(file_path, self.base_config.clone(), &self.events)?)
    }

    async fn build_editor(&mut self, file_path: PathBuf) -> Result<Editor> {
        let mut new = Editor::from_path(file_path, self.base_config.clone(), &self.events)?;
        match self.lsp_servers.entry(new.file_type) {
            Entry::Vacant(entry) => {
                if let Ok(lsp) = LSP::from(&new.file_type).await {
                    let lsp_rc = Rc::new(Mutex::new(lsp));
                    new.lexer.set_lsp(Rc::clone(&lsp_rc), &new.path).await;
                    for editor in self.editors.iter_mut().filter(|e| e.file_type == new.file_type) {
                        editor.lexer.set_lsp(Rc::clone(&lsp_rc), &editor.path).await;
                    }
                    entry.insert(lsp_rc);
                }
            }
            Entry::Occupied(entry) => {
                let lsp_rc = Rc::clone(entry.get());
                new.lexer.set_lsp(lsp_rc, &new.path).await;
            }
        }
        Ok(new)
    }

    pub async fn new_from(&mut self, file_path: PathBuf) {
        for (idx, file) in self.editors.iter().enumerate() {
            if file_path == file.path {
                self.state.select(Some(idx));
                return;
            }
        }
        if let Ok(editor) = self.build_editor(file_path).await {
            self.state.select(Some(self.editors.len()));
            self.editors.push(editor);
        }
    }

    pub async fn new_at_line(&mut self, file_path: PathBuf, line: usize) {
        self.new_from(file_path).await;
        if let Some(editor) = self.get_active() {
            editor.go_to(line);
        }
    }

    pub async fn full_sync(&mut self) {
        todo!()
    }

    async fn close_active(&mut self) {
        if let Some(index) = self.state.selected() {
            let editor = self.editors.remove(index);
            if let Some(lsp) = editor.lexer.lsp {
                let _ = lsp.lock().await.file_did_close(&editor.path).await;
            }
            if self.editors.is_empty() {
                self.state.select(None);
            } else if index >= self.editors.len() {
                self.state.select(Some(index - 1))
            }
        }
    }

    pub fn are_updates_saved(&self) -> bool {
        for editor in self.editors.iter() {
            if !editor.is_saved() {
                return false;
            }
        }
        true
    }

    pub async fn save(&mut self) {
        if let Some(editor) = self.get_active() {
            editor.save().await;
        }
    }

    pub async fn save_all(&mut self) {
        for editor in self.editors.iter_mut() {
            editor.save().await;
        }
    }

    pub async fn refresh_cfg(&mut self, new_key_map: EditorKeyMap) {
        self.key_map = new_key_map;
        self.base_config.refresh();
        for editor in self.editors.iter_mut() {
            editor.refresh_cfg(&self.base_config);
            if let Some(lsp) = self.lsp_servers.get(&editor.file_type) {
                if editor.lexer.lsp.is_none() {
                    editor.lexer.set_lsp(Rc::clone(lsp), &editor.path).await;
                }
            }
        }
    }

    pub async fn graceful_exit(&mut self) {
        for (_, lsp) in self.lsp_servers.iter_mut() {
            let mut lsp = lsp.lock().await;
            let _ = lsp.graceful_exit().await;
        }
    }
}

fn try_file_to_tab(file: &Editor) -> Option<Line> {
    file.path.as_os_str().to_str().map(|t| Line::from(Span::styled(t, Style::default().fg(Color::Green))))
}

#[cfg(test)]
mod test;
