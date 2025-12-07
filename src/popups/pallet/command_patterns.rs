use super::Components;
use crate::{app::MIN_FRAMERATE, editor_line::EditorLine, ext_tui::pty::PtyShell};
use crossterm::event::Event;
use idiom_tui::Backend;
use portable_pty::CommandBuilder;
use std::path::PathBuf;

#[cfg(unix)]
const RUNNER: &str = "sh";
#[cfg(windows)]
const RUNNER: &str = "cmd";

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
                let base_cmd = match src {
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
                        format!("echo \"{updated}\" | {cmd}")
                    }
                    None => cmd.to_owned(),
                };

                match target {
                    PipeTarget::Term => {
                        gs.push_embeded_command(base_cmd, term);
                        return;
                    }
                    PipeTarget::File => (),
                };

                let name: String = base_cmd
                    .chars()
                    .map(|c| if c.is_ascii_alphabetic() || c.is_ascii_digit() { c } else { '_' })
                    .collect();

                let mut builder_cmd = CommandBuilder::new(RUNNER);
                builder_cmd.arg("-c");
                builder_cmd.arg(base_cmd);

                let mut shell = match PtyShell::new(builder_cmd, *gs.editor_area()) {
                    Ok(shell) => shell,
                    Err(error) => {
                        gs.error(error);
                        return;
                    }
                };

                if let Some(line) = gs.tab_area().get_line(0) {
                    line.render(cmd, gs.backend());
                }

                shell.render(gs.backend());
                loop {
                    match shell.try_wait() {
                        Ok(None) => {
                            match crossterm::event::poll(MIN_FRAMERATE) {
                                Ok(true) => match crossterm::event::read() {
                                    Ok(Event::FocusGained | Event::FocusLost) => (),
                                    Ok(Event::Mouse(..)) => (),
                                    Ok(Event::Paste(clip)) => {
                                        if let Err(error) = shell.paste(clip) {
                                            gs.error(error);
                                            return;
                                        }
                                    }
                                    Ok(Event::Resize(width, height)) => {
                                        gs.full_resize(ws, term, width, height);
                                        if let Err(error) = shell.resize(*gs.editor_area()) {
                                            gs.error(error);
                                            return;
                                        };
                                    }
                                    Ok(Event::Key(key)) => {
                                        if let Err(error) = shell.map_key(&key, gs.backend()) {
                                            gs.error(error);
                                            return;
                                        }
                                    }
                                    Err(error) => {
                                        gs.error(error);
                                        return;
                                    }
                                },
                                Ok(false) => (),
                                Err(error) => {
                                    gs.error(error);
                                    return;
                                }
                            }
                            gs.backend.freeze();
                            shell.fast_render(gs.backend());
                            gs.backend.unfreeze();
                        }
                        Ok(Some((status, logs))) => {
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
                                    let content = logs.lines().map(EditorLine::from).collect();
                                    ws.new_text_from_data(path, content, None, gs);
                                }
                                Err(error) => gs.error(error),
                            }
                            return;
                        }
                        Err(error) => {
                            gs.error(error);
                            return;
                        }
                    }
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
