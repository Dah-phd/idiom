use super::Components;
use crate::editor_line::EditorLine;
use std::{
    path::PathBuf,
    process::{Command, Stdio},
};

pub enum Pattern<'a> {
    Select(SelectPat),
    Pipe { cmd: &'a str, src: Option<SelectPat>, target: PipeTarget },
}

impl<'a> Pattern<'a> {
    pub fn parse(text: &'a str) -> Option<Self> {
        let mut chars = text.chars();
        match chars.next()? {
            's' if text.len() == 2 => match chars.next() {
                Some('f' | 'e' | 'a') => Some(Self::Select(SelectPat::File)),
                Some('s') | None => Some(Self::Select(SelectPat::Scope)),
                Some('w') => Some(Self::Select(SelectPat::Word)),
                Some('l') => Some(Self::Select(SelectPat::Line)),
                _ => None,
            },
            '!' if text.len() == 2 => Some(Self::Pipe { cmd: &text[1..], src: None, target: PipeTarget::Term }),
            '!' if text.len() > 2 => {
                let mut src = None;
                let remaining_cmd = match text[1..].split_once('|') {
                    Some((prefix, cmd)) => match prefix.trim() {
                        "f" | "e" | "a" => {
                            src = Some(SelectPat::File);
                            cmd
                        }
                        "s" => {
                            src = Some(SelectPat::Scope);
                            cmd
                        }
                        "w" => {
                            src = Some(SelectPat::Word);
                            cmd
                        }
                        "l" => {
                            src = Some(SelectPat::Line);
                            cmd
                        }
                        _ => &text[1..],
                    },
                    None => &text[1..],
                };
                match remaining_cmd.split_once('>') {
                    Some((cmd, target)) => match target.trim().is_empty() {
                        true => Some(Self::Pipe { cmd, target: PipeTarget::File, src }),
                        false => Some(Self::Pipe { cmd, target: PipeTarget::Term, src }),
                    },
                    None => Some(Self::Pipe { cmd: remaining_cmd, target: PipeTarget::Term, src }),
                }
            }
            _ => None,
        }
    }

    pub fn execute(self, components: &mut Components) {
        let Components { gs, ws, term, .. } = components;
        match self {
            Self::Select(pattern) => {
                let Some(editor) = ws.get_active() else { return };
                match pattern {
                    SelectPat::Scope => editor.select_scope(),
                    SelectPat::File => editor.select_all(),
                    SelectPat::Word => editor.select_word(),
                    SelectPat::Line => editor.select_line(),
                }
            }
            Self::Pipe { cmd, target, src } => {
                let (cmd, arg) = match src {
                    Some(source) => {
                        let Some(editor) = ws.get_active() else { return };
                        match source {
                            SelectPat::Scope => editor.select_scope(),
                            SelectPat::Word => editor.select_word(),
                            SelectPat::Line => editor.select_line(),
                            SelectPat::File => editor.select_all(),
                        };
                        let Some(clip) = editor.copy() else { return };
                        let updated = clip.replace('"', "\\\"");
                        ("echo", format!("\"{updated}\" | {cmd}"))
                    }
                    None => {
                        let Some((cmd, arg)) = cmd.split_once(" ") else { return };
                        (cmd, arg.to_owned())
                    }
                };

                match target {
                    PipeTarget::Term => {
                        gs.push_embeded_command(format!("{cmd} {arg}"), term);
                        return;
                    }
                    PipeTarget::File => (),
                };

                let name: String =
                    cmd.chars().map(|c| if c.is_ascii_alphabetic() || c.is_ascii_digit() { c } else { '_' }).collect();

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
                        let result = Command::new(cmd)
                            .arg(arg)
                            .stdout(Stdio::piped())
                            .stderr(Stdio::piped())
                            .spawn()
                            .and_then(|c| c.wait_with_output());

                        match result {
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

impl ToString for Pattern<'_> {
    fn to_string(&self) -> String {
        match self {
            Self::Select(pat) => format!(" select {} ", pat.as_str()),
            Self::Pipe { cmd, src, target } => match src {
                Some(src) => format!(" pipe {} into {} > {} ", src.as_str(), cmd, target.as_str()),
                None => format!(" run {} > {} ", cmd, target.as_str()),
            },
        }
    }
}

pub enum PipeTarget {
    File,
    Term,
}

impl PipeTarget {
    fn as_str(&self) -> &str {
        match self {
            Self::File => "editor",
            Self::Term => "term",
        }
    }
}

pub enum SelectPat {
    Scope,
    Word,
    File,
    Line,
}

impl SelectPat {
    fn as_str(&self) -> &str {
        match self {
            Self::Scope => "scope",
            Self::Word => "word",
            Self::File => "all",
            Self::Line => "line",
        }
    }
}
