use super::{popup_file_open::OpenFileSelector, PopupInterface};
use crate::{
    configs::{CONFIG_FOLDER, EDITOR_CFG_FILE, KEY_MAP, THEME_FILE, THEME_UI},
    global_state::{Clipboard, GlobalState, IdiomEvent, PopupMessage},
    render::{layout::Rect, state::State, TextField},
    tree::Tree,
    workspace::Workspace,
};
use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use dirs::config_dir;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};

pub struct Pallet {
    commands: Vec<(i64, Command)>,
    access_cb: Option<fn(&mut Workspace, &mut Tree)>,
    pattern: TextField<bool>,
    matcher: SkimMatcherV2,
    updated: bool,
    rect: Option<Rect>,
    state: State,
}

struct Command {
    label: &'static str,
    result: CommandResult,
}

impl Command {
    fn execute(self) -> CommandResult {
        self.result
    }

    fn cfg_open(f: &'static str) -> Option<Self> {
        let mut path = config_dir()?;
        path.push(CONFIG_FOLDER);
        path.push(f);
        Some(Command { label: f, result: CommandResult::Simple(IdiomEvent::OpenAtLine(path, 0).into()) })
    }

    fn pass_event(label: &'static str, event: IdiomEvent) -> Self {
        Command { label, result: CommandResult::Simple(event.into()) }
    }

    fn access_edit(label: &'static str, cb: fn(&mut Workspace, &mut Tree)) -> Self {
        Command { label, result: CommandResult::Complex(cb) }
    }
}

enum CommandResult {
    Simple(PopupMessage),
    Complex(fn(&mut Workspace, &mut Tree)),
}

impl PopupInterface for Pallet {
    fn render(&mut self, gs: &mut GlobalState) {
        let mut rect = gs.screen_rect.top(15).vcenter(100);
        rect.bordered();
        self.rect.replace(rect);
        rect.draw_borders(None, None, gs.backend());
        match rect.next_line() {
            Some(line) => self.pattern.widget(line, gs.backend()),
            None => return,
        }
        let options = self.commands.iter().map(|cmd| cmd.1.label);
        self.state.render_list(options, rect, gs.backend());
    }

    fn key_map(&mut self, key: &KeyEvent, clipboard: &mut Clipboard) -> PopupMessage {
        if self.commands.is_empty() {
            return PopupMessage::Clear;
        }

        if let Some(updated) = self.pattern.map(key, clipboard) {
            if updated {
                for (score, cmd) in self.commands.iter_mut() {
                    *score = match self.matcher.fuzzy_match(cmd.label, &self.pattern.text) {
                        Some(new_score) => new_score,
                        None => i64::MAX,
                    };
                }
                self.commands.sort_by(|(score, _), (rhscore, _)| score.cmp(rhscore));
            }
            return PopupMessage::None;
        }
        match key.code {
            KeyCode::Enter => match self.commands.remove(self.state.selected).1.execute() {
                CommandResult::Simple(msg) => msg,
                CommandResult::Complex(cb) => {
                    self.access_cb.replace(cb);
                    PopupMessage::Event(IdiomEvent::PopupAccessOnce)
                }
            },
            KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') => {
                self.state.prev(self.commands.len());
                PopupMessage::None
            }
            KeyCode::Down | KeyCode::Char('d') | KeyCode::Char('D') => {
                self.state.next(self.commands.len());
                PopupMessage::None
            }
            _ => PopupMessage::None,
        }
    }

    fn mouse_map(&mut self, event: crossterm::event::MouseEvent) -> PopupMessage {
        let (row, column) = match event {
            MouseEvent { kind: MouseEventKind::Up(MouseButton::Left), column, row, .. } => (row, column),
            MouseEvent { kind: MouseEventKind::ScrollUp, .. } => {
                self.state.prev(self.commands.len());
                self.mark_as_updated();
                return PopupMessage::None;
            }
            MouseEvent { kind: MouseEventKind::ScrollDown, .. } => {
                self.state.next(self.commands.len());
                self.mark_as_updated();
                return PopupMessage::None;
            }
            _ => return PopupMessage::None,
        };
        let relative_row = match self.rect.as_ref().and_then(|rect| rect.relative_position(row, column)) {
            Some(pos) => pos.line,
            None => return PopupMessage::None,
        };
        if relative_row < 1 {
            return PopupMessage::None;
        }
        let command_index = self.state.at_line + (relative_row - 1);
        if self.commands.len() <= command_index {
            return PopupMessage::None;
        }
        match self.commands.remove(command_index).1.execute() {
            CommandResult::Simple(msg) => msg,
            CommandResult::Complex(cb) => {
                self.access_cb.replace(cb);
                PopupMessage::Event(IdiomEvent::PopupAccessOnce)
            }
        }
    }

    fn mark_as_updated(&mut self) {
        self.updated = true
    }

    fn collect_update_status(&mut self) -> bool {
        std::mem::take(&mut self.updated)
    }

    fn component_access(&mut self, ws: &mut Workspace, tree: &mut Tree) {
        if let Some(cb) = self.access_cb.take() {
            cb(ws, tree);
        }
    }
}

impl Pallet {
    pub fn new() -> Box<Self> {
        let mut commands = vec![
            (0, Command::pass_event("Open file", IdiomEvent::NewPopup(OpenFileSelector::boxed))),
            (0, Command::access_edit("UPPERCASE", uppercase)),
            (0, Command::access_edit("LOWERCASE", lowercase)),
        ];
        commands.extend(
            [
                Command::cfg_open(EDITOR_CFG_FILE),
                Command::cfg_open(KEY_MAP),
                Command::cfg_open(THEME_FILE),
                Command::cfg_open(THEME_UI),
            ]
            .into_iter()
            .flatten()
            .map(|cmd| (0, cmd)),
        );
        Box::new(Pallet {
            commands,
            access_cb: None,
            pattern: TextField::new(String::new(), Some(true)),
            matcher: SkimMatcherV2::default(),
            updated: true,
            rect: None,
            state: State::new(),
        })
    }
}

fn uppercase(ws: &mut Workspace, _tree: &mut Tree) {
    if let Some(editor) = ws.get_active() {
        if editor.cursor.select_is_none() {
            editor.select_token();
        }
        if editor.cursor.select_is_none() {
            return;
        }
        if let Some(clip) = editor.copy() {
            editor.insert_snippet(clip.to_uppercase(), None);
        }
    }
}

fn lowercase(ws: &mut Workspace, _tree: &mut Tree) {
    if let Some(editor) = ws.get_active() {
        if editor.cursor.select_is_none() {
            editor.select_token();
        }
        if editor.cursor.select_is_none() {
            return;
        }
        if let Some(clip) = editor.copy() {
            editor.insert_snippet(clip.to_lowercase(), None);
        }
    }
}

#[cfg(test)]
mod test {}
