mod file;
mod linter;

use file::File;
use std::path::PathBuf;
use tui::layout::{Constraint, Layout, Rect};
use tui::text::{Span, Spans};
use tui::widgets::{Block, Borders, List, ListItem, ListState};
use tui::{backend::Backend, Frame};

#[derive(Default)]
pub struct EditorState {
    pub editors: Vec<File>,
    pub state: ListState,
}

impl EditorState {
    pub fn render(&mut self, frame: &mut Frame<impl Backend>, area: Rect) {
        let layout = Layout::default()
            .constraints(vec![Constraint::Percentage(6), Constraint::Min(2)])
            .split(area);
        if let Some(editor_id) = self.state.selected() {
            if let Some(file) = self.editors.get(editor_id) {
                let text_lines: Vec<ListItem> = file.content.iter().map(linter).collect();
                let editor_content = List::new(text_lines).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(file.path.as_os_str().to_str().unwrap_or("Loading ...")),
                );
                frame.render_stateful_widget(editor_content, layout[1], &mut self.state)
            }
        }
    }

    pub fn new_from(&mut self, file_path: PathBuf) {
        for (idx, file) in self.editors.iter().enumerate() {
            if file_path == file.path {
                self.state.select(Some(idx));
                return;
            }
        }
        if let Ok(opened_file) = File::from_path(file_path) {
            self.state.select(Some(self.editors.len()));
            self.editors.push(opened_file);
        }
    }
}

fn linter(line: &String) -> ListItem {
    ListItem::new(vec![Spans::from(Span::raw(line))])
}
