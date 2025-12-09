use super::Components;
use crate::{
    app::MIN_FRAMERATE,
    editor::Editor,
    editor_line::EditorLine,
    embeded_term::EditorTerminal,
    error::{IdiomError, IdiomResult},
    ext_tui::pty::PtyShell,
    global_state::GlobalState,
    utils::SHELL,
    workspace::Workspace,
};
use crossterm::event::Event;
use idiom_tui::Backend;
use portable_pty::CommandBuilder;
use std::path::PathBuf;

pub enum Pattern<'a> {
    Select(SelectPat),
    Pipe { cmd: &'a str, src: Option<Source>, target: PipeTarget },
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
                        "#s" => {
                            src = Some(Source::Select { generator: None });
                            cmd
                        }
                        "#sf" | "#se" | "#sa" => {
                            src = Some(Source::Select { generator: Some(SelectPat::File) });
                            cmd
                        }
                        "#ss" => {
                            src = Some(Source::Select { generator: Some(SelectPat::Scope) });
                            cmd
                        }
                        "#sw" => {
                            src = Some(Source::Select { generator: Some(SelectPat::Word) });
                            cmd
                        }
                        "#sl" => {
                            src = Some(Source::Select { generator: Some(SelectPat::Line) });
                            cmd
                        }
                        _ => &text[1..],
                    },
                    None => &text[1..],
                };
                match remaining_cmd.split_once('>') {
                    Some((cmd, target)) => match target.trim() {
                        "#" => Some(Self::Pipe { cmd, target: PipeTarget::New, src }),
                        "#s" => Some(Self::Pipe { cmd, target: PipeTarget::Select, src }),
                        _ => Some(Self::Pipe { cmd, target: PipeTarget::Term, src }),
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
                pattern.execute(editor);
            }
            Self::Pipe { cmd, target, src } => {
                let result = shell_executor(cmd, src, target, ws, term, gs);
                gs.log_if_error(result);
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
    New,
    Select,
    Term,
}

impl PipeTarget {
    const fn as_str(&self) -> &str {
        match self {
            Self::New => "editor",
            Self::Select => "replace select",
            Self::Term => "term",
        }
    }
}

pub enum Source {
    Select { generator: Option<SelectPat> },
}

impl Source {
    const fn as_str(&self) -> &str {
        match self {
            Self::Select { generator: None } => "select",
            Self::Select { generator: Some(pat) } => pat.as_str(),
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
    const fn as_str(&self) -> &str {
        match self {
            Self::Scope => "scope",
            Self::Word => "word",
            Self::File => "all",
            Self::Line => "line",
        }
    }

    fn execute(&self, editor: &mut Editor) {
        match self {
            SelectPat::Scope => editor.select_scope(),
            SelectPat::Word => editor.select_word(),
            SelectPat::Line => editor.select_line(),
            SelectPat::File => editor.select_all(),
        }
    }
}

fn shell_executor(
    cmd: &str,
    src: Option<Source>,
    target: PipeTarget,
    ws: &mut Workspace,
    term: &mut EditorTerminal,
    gs: &mut GlobalState,
) -> IdiomResult<()> {
    let base_cmd = match src {
        Some(source) => {
            let editor = ws.get_active().ok_or(IdiomError::any("No files open in editor!"))?;
            match source {
                Source::Select { generator } => {
                    if let Some(generator) = generator {
                        generator.execute(editor);
                    }
                    let clip = editor.copy().ok_or(IdiomError::any("Unable to pull data from editor!"))?;
                    let updated = clip.replace('"', "\\\"");
                    match cmd.trim().is_empty() {
                        true => format!("echo \"{updated}\""),
                        false => format!("echo \"{updated}\" | {cmd}"),
                    }
                }
            }
        }
        None => cmd.to_owned(),
    };

    if let PipeTarget::Term = target {
        gs.push_embeded_command(base_cmd, term);
        return Ok(());
    }

    let name = base_cmd
        .chars()
        .map(|c| if c.is_ascii_alphabetic() || c.is_ascii_digit() { c } else { '_' })
        .collect::<String>();

    let mut builder_cmd = CommandBuilder::new(SHELL);
    builder_cmd.arg("-c");
    builder_cmd.arg(base_cmd);

    let mut shell = PtyShell::new(builder_cmd, *gs.editor_area())?;

    if let Some(line) = gs.tab_area().get_line(0) {
        line.render(cmd, gs.backend());
    }

    shell.render(gs.backend());
    loop {
        match shell.try_wait()? {
            None => {
                if crossterm::event::poll(MIN_FRAMERATE)? {
                    match crossterm::event::read()? {
                        Event::FocusGained | Event::FocusLost => (),
                        Event::Mouse(..) => (),
                        Event::Paste(clip) => shell.paste(clip)?,
                        Event::Key(key) => shell.map_key(&key, gs.backend())?,
                        Event::Resize(width, height) => {
                            gs.full_resize(ws, term, width, height);
                            shell.resize(*gs.editor_area()).map_err(|err| IdiomError::any(err))?;
                        }
                    }
                }
                gs.backend.freeze();
                shell.fast_render(gs.backend());
                gs.backend.unfreeze();
            }
            Some((status, logs)) => {
                match target {
                    PipeTarget::New => {
                        if !status.success() {
                            gs.error(format!("CMD STATUS: {}", status));
                        }
                    }
                    PipeTarget::Select => {
                        if !status.success() {
                            gs.error(format!("CMD STATUS: {}", status));
                        } else {
                            let editor = ws.get_active().ok_or(IdiomError::any("No files open in editor!"))?;
                            editor.paste(logs, gs);
                        }
                        return Ok(());
                    }
                    _ => {}
                }

                let base_path = PathBuf::from("./").canonicalize()?;
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
                return Ok(());
            }
        }
    }
}
