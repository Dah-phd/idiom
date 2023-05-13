mod file;
mod linter;

use linter::linter;
use file::Editor;
use std::path::PathBuf;
use tui::layout::{Constraint, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::{Block, Borders, List, ListItem, ListState, Tabs};
use tui::{backend::Backend, Frame};

#[derive(Default)]
pub struct EditorState {
    pub editors: Vec<Editor>,
    pub state: ListState,
}

impl EditorState {
    pub fn render(&mut self, frame: &mut Frame<impl Backend>, area: Rect) {
        let layout = Layout::default()
            .constraints(vec![Constraint::Percentage(6), Constraint::Min(2)])
            .split(area);
        if let Some(editor_id) = self.state.selected() {
            if let Some(file) = self.editors.get(editor_id) {
                let editor_content = List::new(file.content.iter().enumerate().map(linter).collect::<Vec<ListItem>>()).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(file.path.as_os_str().to_str().unwrap_or("Loading ...")),
                );
                frame.set_cursor(layout[1].x + 5 + file.cursor.1 as u16, layout[1].y + 1 + file.cursor.0 as u16);
                frame.render_stateful_widget(editor_content, layout[1], &mut self.state);

                let titles = self
                    .editors
                    .iter()
                    .flat_map(try_file_to_tab)
                    .collect();

                let tabs = Tabs::new(titles)
                    .block(Block::default().title("open editors"))
                    .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                    .select(editor_id);

                frame.render_widget(tabs, layout[0]);
                
            }
        }
    }

    pub fn get_active(&mut self) -> Option<&mut Editor> {
        self.editors.get_mut( self.state.selected()?)
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
}

fn try_file_to_tab(file:& Editor) -> Option<Spans> {
    file.path.as_os_str().to_str().map(|t| Spans::from(Span::styled(t, Style::default().fg(Color::Green))))
}
