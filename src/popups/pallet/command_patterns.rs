use crate::{editor_line::EditorLine, global_state::GlobalState, workspace::Workspace};
use std::{
    path::PathBuf,
    process::{Command, Stdio},
};

pub enum Pattern<'a> {
    Select(SelectPat),
    Pipe { cmd: &'a str, target: PipeTarget },
}

impl<'a> Pattern<'a> {
    pub fn parse(text: &'a str) -> Option<Self> {
        let mut chars = text.chars();
        match chars.next()? {
            's' => match chars.next() {
                Some('s') | None => Some(Self::Select(SelectPat::Scope)),
                Some('w') => Some(Self::Select(SelectPat::Word)),
                Some('f' | 'e' | 'a') => Some(Self::Select(SelectPat::File)),
                _ => None,
            },
            '>' => match chars.next()? {
                'f' | 'e' => {
                    let cmd = &text[2..];
                    if cmd.is_empty() {
                        return None;
                    }
                    Some(Self::Pipe { cmd, target: PipeTarget::Null })
                }
                _ => None,
            },
            _ => None,
        }
    }

    pub fn execute(self, ws: &mut Workspace, gs: &mut GlobalState) {
        match self {
            Self::Select(pattern) => {
                let Some(editor) = ws.get_active() else { return };
                match pattern {
                    SelectPat::Scope => editor.select_scope(),
                    SelectPat::File => editor.select_all(),
                    SelectPat::Word => editor.select_word(),
                }
            }
            Self::Pipe { cmd, target } => {
                if cmd.is_empty() {
                    return;
                }
                let name: String =
                    cmd.chars().map(|c| if c.is_ascii_alphabetic() || c.is_ascii_digit() { c } else { '_' }).collect();

                let mut cmd_split = cmd.split(" ");
                let Some(cmd) = cmd_split.next() else {
                    return;
                };
                match PathBuf::from("./").canonicalize() {
                    Ok(base_path) => {
                        let mut path = base_path.clone();
                        path.push(format!("{name}.out"));
                        let mut id = 0_usize;
                        while path.exists() {
                            path = base_path.clone();
                            path.push(format!("{name}_{id}.out"));
                            id += 1;
                        }
                        let child =
                            Command::new(cmd).args(cmd_split).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn();

                        match child.and_then(|c| c.wait_with_output()) {
                            Ok(out) => {
                                let mut content = vec![];
                                if out.status.success() {
                                    // adds errors on top
                                    content.extend(String::from_utf8_lossy(&out.stderr).lines().map(EditorLine::from));
                                    content.extend(String::from_utf8_lossy(&out.stdout).lines().map(EditorLine::from));
                                } else {
                                    // adds out on top
                                    content.extend(String::from_utf8_lossy(&out.stdout).lines().map(EditorLine::from));
                                    content.extend(String::from_utf8_lossy(&out.stderr).lines().map(EditorLine::from));
                                }
                                if !content.is_empty() {
                                    ws.new_text_from_data(path, content, None, gs);
                                }
                            }
                            Err(error) => gs.error(error),
                        }
                    }
                    Err(error) => gs.error(error),
                }
            }
        }
    }
}

pub enum PipeTarget {
    File,
    Term,
    Null,
}

pub enum SelectPat {
    Scope,
    Word,
    File,
}
