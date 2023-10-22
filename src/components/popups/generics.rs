use std::io::Stdout;

use super::PopupInterface;
use crate::components::{Tree, Workspace};
use crate::utils::{centered_rect_static, right_corner_rect_static};
use crate::{components::Footer, configs::PopupMessage};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

pub struct Popup {
    pub message: String,
    pub title: Option<String>,
    pub message_as_buffer_builder: Option<fn(char) -> Option<char>>,
    pub buttons: Vec<Button>,
    pub size: Option<(u16, u16)>,
    pub state: usize,
}

impl PopupInterface for Popup {
    fn render(&mut self, frame: &mut Frame<CrosstermBackend<&Stdout>>) {
        let block = Block::default().title(self.title()).borders(Borders::ALL);
        let (h, v) = self.size.unwrap_or((40, 6));
        let area = centered_rect_static(h, v, frame.size());
        frame.render_widget(Clear, area);
        frame.render_widget(block, area);

        let chunks = Layout::default()
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .margin(1)
            .split(area);

        frame.render_widget(self.p_from_message(), chunks[0]);

        frame.render_widget(self.spans_from_buttons(), chunks[1]);
    }

    fn key_map(&mut self, key: &KeyEvent) -> PopupMessage {
        if let Some(button) =
            self.buttons.iter().find(|button| matches!(&button.key, Some(key_code) if key_code.contains(&key.code)))
        {
            return (button.command)(self);
        }
        match key.code {
            KeyCode::Char(ch) if self.message_as_buffer_builder.is_some() => {
                if let Some(buffer_builder) = self.message_as_buffer_builder {
                    if let Some(ch) = buffer_builder(ch) {
                        self.message.push(ch);
                    }
                }
                PopupMessage::None
            }
            KeyCode::Backspace if self.message_as_buffer_builder.is_some() => {
                self.message.pop();
                PopupMessage::None
            }
            KeyCode::Enter => (self.buttons[self.state].command)(self),
            KeyCode::Left => {
                if self.state > 0 {
                    self.state -= 1;
                } else {
                    self.state = self.buttons.len() - 1;
                }
                PopupMessage::None
            }
            KeyCode::Right => {
                if self.state < self.buttons.len() - 1 {
                    self.state += 1;
                } else {
                    self.state = 0;
                }
                PopupMessage::None
            }
            KeyCode::Esc => PopupMessage::Done,
            _ => PopupMessage::None,
        }
    }
    fn update_workspace(&mut self, _editor_state: &mut Workspace) {}
    fn update_tree(&mut self, _file_tree: &mut Tree) {}
    fn update_footer(&mut self, _footer: &mut Footer) {}
}

impl Popup {
    fn p_from_message(&self) -> Paragraph {
        if self.message_as_buffer_builder.is_none() {
            return Paragraph::new(Span::from(self.message.to_owned())).alignment(Alignment::Center);
        }
        Paragraph::new(Line::from(vec![
            Span::raw(" >> "),
            Span::raw(self.message.to_owned()),
            Span::styled("|", Style::default().add_modifier(Modifier::SLOW_BLINK)),
        ]))
    }

    fn title(&self) -> String {
        if let Some(title) = &self.title {
            return format!("{title} ");
        }
        "Prompt".to_owned()
    }

    fn spans_from_buttons(&self) -> Paragraph<'_> {
        Paragraph::new(Line::from(
            self.buttons
                .iter()
                .enumerate()
                .map(|(idx, button)| {
                    let padded_name = format!("  {}  ", button.name);
                    if self.state == idx {
                        Span::styled(padded_name, Style::default().add_modifier(Modifier::REVERSED))
                    } else {
                        Span::raw(padded_name)
                    }
                })
                .collect::<Vec<_>>(),
        ))
        .alignment(Alignment::Center)
    }
}

pub struct PopupSelector<T> {
    pub options: Vec<T>,
    pub display: fn(&T) -> String,
    pub command: fn(&mut PopupSelector<T>) -> PopupMessage,
    pub state: usize,
    pub size: Option<(u16, u16)>,
}

impl<T> PopupInterface for PopupSelector<T> {
    fn render(&mut self, frame: &mut Frame<CrosstermBackend<&Stdout>>) {
        let (h, v) = self.size.unwrap_or((120, 20));
        let area = centered_rect_static(h, v, frame.size());
        frame.render_widget(Clear, area);

        let mut state = ListState::default();
        state.select(Some(self.state));
        let options = if self.options.is_empty() {
            vec![ListItem::new("No results found!")]
        } else {
            self.options.iter().map(|el| ListItem::new((self.display)(el))).collect::<Vec<_>>()
        };
        let list = List::new(options)
            .block(Block::default().borders(Borders::ALL).title("Select"))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
        frame.render_stateful_widget(list, area, &mut state);
    }

    fn key_map(&mut self, key: &KeyEvent) -> PopupMessage {
        if self.options.is_empty() {
            return PopupMessage::Done;
        }
        match key.code {
            KeyCode::Enter => (self.command)(self),
            KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') => {
                if self.state > 0 {
                    self.state -= 1;
                } else {
                    self.state = self.options.len() - 1;
                }
                PopupMessage::None
            }
            KeyCode::Down | KeyCode::Char('d') | KeyCode::Char('D') => {
                if self.state < self.options.len() - 1 {
                    self.state += 1;
                } else {
                    self.state = 0;
                }
                PopupMessage::None
            }
            KeyCode::Esc => PopupMessage::Done,
            _ => PopupMessage::None,
        }
    }
    fn update_workspace(&mut self, _editor_state: &mut Workspace) {}
    fn update_tree(&mut self, _file_tree: &mut Tree) {}
    fn update_footer(&mut self, _footer: &mut Footer) {}
}

pub struct PopupActiveSelector<T: Clone> {
    pub options: Vec<T>,
    pub pattern: String,
    pub state: usize,
    command: fn(&mut Self) -> PopupMessage,
    to_selector: Option<fn(&mut Self) -> PopupMessage>,
    on_update: PopupMessage,
    footer_callback: Option<fn(&mut Self, &mut Footer)>,
    editor_callback: Option<fn(&mut Self, &mut Workspace)>,
    tree_callback: Option<fn(&mut Self, &mut Tree)>,
}

impl<T: Clone> PopupActiveSelector<T> {
    pub fn default(command: fn(&mut Self) -> PopupMessage, to_selector: Option<fn(&mut Self) -> PopupMessage>) -> Self {
        Self {
            options: Vec::new(),
            pattern: String::new(),
            state: 0,
            command,
            to_selector,
            on_update: PopupMessage::None,
            footer_callback: None,
            editor_callback: None,
            tree_callback: None,
        }
    }

    #[allow(dead_code)]
    pub fn for_footer(
        command: fn(&mut Self) -> PopupMessage,
        callback: fn(&mut Self, &mut Footer),
        to_selector: Option<fn(&mut Self) -> PopupMessage>,
    ) -> Self {
        Self {
            options: Vec::new(),
            pattern: String::new(),
            state: 0,
            command,
            to_selector,
            on_update: PopupMessage::UpdateFooter,
            footer_callback: Some(callback),
            editor_callback: None,
            tree_callback: None,
        }
    }

    pub fn for_editor(
        command: fn(&mut Self) -> PopupMessage,
        callback: fn(&mut Self, &mut Workspace),
        to_selector: Option<fn(&mut Self) -> PopupMessage>,
    ) -> Self {
        Self {
            options: Vec::new(),
            pattern: String::new(),
            state: 0,
            command,
            to_selector,
            on_update: PopupMessage::UpdateWorkspace,
            footer_callback: None,
            editor_callback: Some(callback),
            tree_callback: None,
        }
    }

    #[allow(dead_code)]
    pub fn for_tree(
        command: fn(&mut Self) -> PopupMessage,
        callback: fn(&mut Self, &mut Tree),
        to_selector: Option<fn(&mut Self) -> PopupMessage>,
    ) -> Self {
        Self {
            options: Vec::new(),
            pattern: String::new(),
            state: 0,
            command,
            to_selector,
            on_update: PopupMessage::UpdateTree,
            footer_callback: None,
            editor_callback: None,
            tree_callback: Some(callback),
        }
    }

    pub fn next(&mut self) -> Option<T> {
        if self.options.len() - 1 > self.state {
            self.state += 1;
        } else {
            self.state = 0;
        }
        self.options.get(self.state).cloned()
    }

    pub fn drain_next(&mut self) -> Option<T> {
        self.next()?;
        Some(self.options.remove(self.state))
    }

    pub fn get_option_count_as_string(&self) -> String {
        let len = self.options.len();
        if len < 10 {
            format!("  {len}")
        } else if len < 100 {
            format!(" {len}")
        } else {
            String::from("99+")
        }
    }
}

impl<T: Clone> PopupInterface for PopupActiveSelector<T> {
    fn key_map(&mut self, key: &KeyEvent) -> PopupMessage {
        match key.code {
            KeyCode::Enter | KeyCode::Right => (self.command)(self),
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => PopupMessage::Done,
            KeyCode::Char(ch) => {
                self.pattern.push(ch);
                self.on_update.clone()
            }
            KeyCode::Backspace => {
                self.pattern.pop();
                self.on_update.clone()
            }
            KeyCode::Up => PopupMessage::None,
            KeyCode::Esc | KeyCode::Left => PopupMessage::Done,
            KeyCode::Tab => {
                if let Some(cb) = self.to_selector {
                    (cb)(self)
                } else {
                    PopupMessage::Done
                }
            }
            _ => PopupMessage::None,
        }
    }
    fn render(&mut self, frame: &mut Frame<CrosstermBackend<&Stdout>>) {
        let area = right_corner_rect_static(50, 3, frame.size());
        let block = Block::default().title("Find").borders(Borders::ALL);
        frame.render_widget(Clear, area);
        let paragrapth = Paragraph::new(Line::from(vec![
            Span::raw(self.get_option_count_as_string()),
            Span::raw(" >> "),
            Span::raw(self.pattern.to_owned()),
            Span::styled("|", Style::default().add_modifier(Modifier::SLOW_BLINK)),
        ]));
        frame.render_widget(paragrapth.block(block), area);
    }
    fn update_workspace(&mut self, workspace: &mut Workspace) {
        if let Some(cb) = self.editor_callback {
            (cb)(self, workspace);
            self.state = self.options.len().checked_sub(1).unwrap_or_default();
        }
    }
    fn update_tree(&mut self, file_tree: &mut Tree) {
        if let Some(cb) = self.tree_callback {
            (cb)(self, file_tree);
            self.state = self.options.len().checked_sub(1).unwrap_or_default();
        }
    }
    fn update_footer(&mut self, footer: &mut Footer) {
        if let Some(cb) = self.footer_callback {
            (cb)(self, footer);
            self.state = self.options.len().checked_sub(1).unwrap_or_default();
        }
    }
}

#[derive(Clone)]
pub struct Button {
    pub command: fn(&mut Popup) -> PopupMessage,
    pub name: &'static str,
    pub key: Option<Vec<KeyCode>>,
}

impl std::fmt::Debug for Button {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("").field(&self.name).finish()
    }
}

pub fn message(content: String) -> Popup {
    Popup {
        message: content,
        title: Some("Message".to_owned()),
        message_as_buffer_builder: None,
        buttons: vec![Button { command: |_| PopupMessage::Done, name: "Ok", key: None }],
        size: Some((20, 16)),
        state: 0,
    }
}
