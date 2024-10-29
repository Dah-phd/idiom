use super::{popup_file_open::OpenFileSelector, PopupInterface};
use crate::{
    configs::{CONFIG_FOLDER, EDITOR_CFG_FILE, KEY_MAP, THEME_FILE, THEME_UI},
    global_state::{Clipboard, GlobalState, IdiomEvent, PopupMessage},
    render::{state::State, TextField},
    tree::Tree,
    workspace::Workspace,
};
use crossterm::event::{KeyCode, KeyEvent};
use dirs::config_dir;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};

pub struct Pallet {
    commands: Vec<(i64, Command)>,
    access_cb: Option<fn(&mut Workspace, &mut Tree)>,
    pattern: TextField<bool>,
    matcher: SkimMatcherV2,
    updated: bool,
    state: State,
}

struct Command {
    label: &'static str,
    direct_call: Option<char>,
    result: PopupMessage,
}

impl Command {
    fn execute(self) -> CommandResult {
        CommandResult::Simple(self.result)
    }

    fn cfg_open(f: &'static str) -> Option<Self> {
        let mut path = config_dir()?;
        path.push(CONFIG_FOLDER);
        path.push(f);
        Some(Command { label: f, direct_call: None, result: IdiomEvent::OpenAtLine(path, 0).into() })
    }

    fn pass_event(label: &'static str, event: IdiomEvent) -> Self {
        Command { label, direct_call: None, result: event.into() }
    }
}

enum CommandResult {
    Simple(PopupMessage),
    Complex(fn(&mut Workspace, &mut Tree)),
}

impl PopupInterface for Pallet {
    fn collect_update_status(&mut self) -> bool {
        std::mem::take(&mut self.updated)
    }

    fn fast_render(&mut self, gs: &mut GlobalState) {
        if self.collect_update_status() {
            self.render(gs);
        }
    }

    fn component_access(&mut self, ws: &mut Workspace, tree: &mut Tree) {
        if let Some(cb) = self.access_cb.take() {
            cb(ws, tree);
        }
    }

    fn key_map(&mut self, key: &KeyEvent, clipboard: &mut Clipboard) -> PopupMessage {
        if self.commands.is_empty() {
            return PopupMessage::Clear;
        }

        if let Some(updated) = self.pattern.map(key, clipboard) {
            self.updated = updated;
            for (score, cmd) in self.commands.iter_mut() {
                *score = match self.matcher.fuzzy_match(cmd.label, &self.pattern.text) {
                    Some(new_score) => new_score,
                    None => i64::MAX,
                };
            }
            self.commands.sort_by(|(score, _), (rhscore, _)| rhscore.cmp(score));
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

    fn mark_as_updated(&mut self) {
        self.updated = true
    }

    fn render(&mut self, gs: &mut GlobalState) {
        let mut rect = gs.screen_rect.top(15).vcenter(100);
        rect.bordered();
        rect.draw_borders(None, None, gs.backend());
        match rect.next_line() {
            Some(line) => self.pattern.widget(line, gs.backend()),
            None => return,
        }
        let options = self.commands.iter().map(|cmd| cmd.1.label);
        self.state.render_list(options, rect, gs.backend());
    }
}

impl Pallet {
    pub fn new() -> Box<Self> {
        let mut commands = vec![(0, Command::pass_event("Open file", IdiomEvent::NewPopup(OpenFileSelector::boxed)))];
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
            state: State::new(),
        })
    }
}

#[cfg(test)]
mod test {}
