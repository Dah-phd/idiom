use crate::components::editor::file::Editor;

#[derive(Debug, Default)]
pub struct ActionLogger {
    buffer: Option<Action>,
    done: Vec<Action>,
    undone: Vec<Action>,
}

impl ActionLogger {
    pub fn push(&mut self, action: Action) {
        self.done.push(action)
    }

    fn new_line(&mut self, line: usize) {
        self.push_buffer();
        self.done.push(Action::NewLine {
            line,
            content: String::new(),
        })
    }

    fn push_char(&mut self, position: (usize, usize), ch: char) {
        if let Some(Action::Insert { position: _, content }) = &mut self.buffer {
            content.push(ch)
        }
        self.push_buffer();
        self.buffer = Some(Action::Insert {
            position: position.into(),
            content: String::from(ch),
        });
    }

    fn del_char(&mut self, position: (usize, usize), ch: char) {
        if let Some(Action::Remove { position: _, content }) = &mut self.buffer {
            content.push(ch)
        }
        self.push_buffer();
        self.buffer = Some(Action::Remove {
            position: position.into(),
            content: String::from(ch),
        })
    }

    fn pull_char(&mut self, position: (usize, usize), ch: char) {
        let position_new: CursorPosition = position.into();
        if let Some(Action::Remove { position, content }) = &mut self.buffer {
            content.insert(0, ch);
            (*position) = position_new.clone();
        }
        self.push_buffer();
        self.buffer = Some(Action::Remove {
            position: position_new,
            content: String::from(ch),
        })
    }

    fn push_buffer(&mut self) {
        if let Some(buffer) = self.buffer.take() {
            self.done.push(buffer)
        }
    }

    fn handle_action(action: &Action, editor: &mut Editor) {
        match &action {
            Action::Swap { from, to } => {
                editor.cursor.line = *from;
                if from < to {
                    editor.swap_down()
                } else {
                    editor.swap_up()
                }
            }
            Action::UpdateState { new: _, old } => editor.content = old.lines().map(|line| line.to_owned()).collect(),
            Action::NewLine { line, content } => {
                editor.content.insert(*line, content.to_owned());
            }
            Action::RemoveLine { line, content: _ } => {
                editor.content.remove(*line);
            }
            Action::Insert { position, content } => {
                editor.content[position.line].insert_str(position.char, content);
                editor.cursor.line = position.line;
                editor.cursor.char = position.char + content.len();
            }
            Action::Remove { position, content } => {
                editor.content[position.line].replace_range(position.char..(position.char + content.len()), "");
                editor.cursor.line = position.line;
                editor.cursor.char = position.char;
            }
        }
    }

    fn undo(&mut self, editor: &mut Editor) {
        if let Some(action) = self.done.pop() {
            Self::handle_action(&action, editor);
            self.undone.push(action.reverse())
        }
    }

    fn redo(&mut self, editor: &mut Editor) {
        if let Some(action) = self.undone.pop() {
            Self::handle_action(&action, editor);
            self.done.push(action.reverse())
        }
    }
}

#[derive(Debug)]
pub enum Action {
    NewLine { line: usize, content: String },
    RemoveLine { line: usize, content: String },
    Insert { position: CursorPosition, content: String },
    Remove { position: CursorPosition, content: String },
    Swap { from: usize, to: usize },
    UpdateState { new: String, old: String },
}

impl Action {
    fn reverse(self) -> Self {
        match self {
            Self::NewLine { line, content } => Self::RemoveLine { line, content },
            Self::RemoveLine { line, content } => Self::NewLine { line, content },
            Self::Insert { position, content } => Self::Remove { position, content },
            Self::Remove { position, content } => Self::Insert { position, content },
            Self::Swap { from, to } => Self::Swap { from: to, to: from },
            Self::UpdateState { new, old } => Self::UpdateState { new: old, old: new },
        }
    }
}

#[derive(Debug, Clone)]
struct CursorPosition {
    line: usize,
    char: usize,
}

impl From<(usize, usize)> for CursorPosition {
    fn from(value: (usize, usize)) -> Self {
        Self {
            line: value.0,
            char: value.1,
        }
    }
}
