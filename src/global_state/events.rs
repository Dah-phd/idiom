use super::{GlobalState, Mode};
use crate::configs::FileType;
use crate::configs::{EditorAction, TreeAction};
use crate::embeded_term::EditorTerminal;
use crate::embeded_tui::run_embeded_tui;
use crate::lsp::TreeDiagnostics;
use crate::popups::generic_selector::PopupSelector;
use crate::popups::mark_word::render_marked_word;
use crate::popups::pallet::Pallet;
use crate::popups::popup_tree_search::ActiveFileSearch;
use crate::popups::{Popup, PopupChoice};
use crate::tree::Tree;
use crate::workspace::line::EditorLine;
use crate::workspace::CursorPosition;
use crate::workspace::Workspace;
use lsp_types::{request::GotoDeclarationResponse, Location, LocationLink, Range, WorkspaceEdit};
use std::path::PathBuf;

#[derive(PartialEq, Debug, Clone)]
pub enum StartInplacePopup {
    Pop(PopupChoice),
    RefSelector(PopupSelector<(String, PathBuf, Range)>),
    Mesasge(PopupSelector<String>),
    MarkWord,
}

#[derive(PartialEq, Debug, Clone)]
pub enum IdiomEvent {
    CreateFileOrFolder { name: String, from_base: bool },
    RenamedFile { from_path: PathBuf, to_path: PathBuf },
    GoToSelect { from: CursorPosition, to: CursorPosition },
    GoToLine(usize),
    TreeDiagnostics(TreeDiagnostics),
    EditorActionCall(EditorAction),
    TreeActionCall(TreeAction),
    EmbededApp(Option<String>),
    InplacePopup(StartInplacePopup),
    OpenAtLine(PathBuf, usize),
    OpenAtSelect(PathBuf, (CursorPosition, CursorPosition)),
    OpenLSPErrors,
    SelectPath(PathBuf),
    RenameFile(String),
    SearchFiles(String),
    FileUpdated(PathBuf),
    CheckLSP(FileType),
    SetLSP(FileType),
    InsertText(String),
    WorkspaceEdit(WorkspaceEdit),
    ActivateEditor(usize),
    SetMode(Mode),
    IdiomCommand,
    Save,
    Rebase,
}

impl IdiomEvent {
    pub async fn handle(self, gs: &mut GlobalState, ws: &mut Workspace, tree: &mut Tree, term: &mut EditorTerminal) {
        match self {
            IdiomEvent::EditorActionCall(action) => {
                if let Some(editor) = ws.get_active() {
                    let _ = editor.map(action, gs);
                }
            }
            IdiomEvent::TreeActionCall(action) => {
                tree.map_action(action, gs);
            }
            IdiomEvent::EmbededApp(cmd) => {
                if let Err(error) = run_embeded_tui(cmd.as_deref(), ws, term, gs) {
                    gs.error(error);
                };
                gs.draw_callback = super::draw::full_rebuild;
            }
            IdiomEvent::InplacePopup(pop) => match pop {
                StartInplacePopup::Pop(mut popup) => {
                    if let Err(error) = popup.run(gs, ws, tree, term) {
                        gs.error(error);
                    };
                }
                StartInplacePopup::RefSelector(mut popup) => {
                    if let Err(error) = popup.run(gs, ws, tree, term) {
                        gs.error(error);
                    };
                }
                StartInplacePopup::Mesasge(mut popup) => {
                    if let Err(error) = popup.run(gs, ws, tree, term) {
                        gs.error(error);
                    };
                }
                StartInplacePopup::MarkWord => {
                    if let Err(error) = render_marked_word(gs, ws, tree, term) {
                        gs.error(error);
                    }
                }
            },
            IdiomEvent::SearchFiles(pattern) => {
                ActiveFileSearch::run(pattern, gs, ws, tree, term);
            }
            IdiomEvent::OpenAtLine(path, line) => {
                let select_result = tree.select_by_path(&path);
                gs.log_if_error(select_result);
                match ws.new_at_line(path, line, gs).await {
                    Ok(..) => gs.insert_mode(),
                    Err(error) => gs.error(error),
                }
            }
            IdiomEvent::OpenAtSelect(path, (from, to)) => {
                let select_result = tree.select_by_path(&path);
                gs.log_if_error(select_result);
                match ws.new_from(path, gs).await {
                    Ok(..) => {
                        gs.insert_mode();
                        if let Some(editor) = ws.get_active() {
                            editor.go_to_select(from, to);
                        };
                    }
                    Err(error) => gs.error(error),
                };
            }
            IdiomEvent::OpenLSPErrors => match PathBuf::from("./").canonicalize() {
                Ok(base_path) => {
                    let mut path = base_path.clone();
                    path.push("editor_error.log");
                    let mut id = 0_usize;
                    while path.exists() {
                        path = base_path.clone();
                        path.push(format!("editor_error_{id}.log"));
                        id += 1;
                    }
                    let content: Vec<EditorLine> =
                        gs.messages.get_logs().map(ToOwned::to_owned).map(EditorLine::from).collect();
                    if !content.is_empty() {
                        ws.new_text_from_data(path, content, None, gs);
                    } else {
                        gs.success(" >> no error logs found!");
                    }
                }
                Err(error) => gs.error(error),
            },
            IdiomEvent::GoToLine(line) => {
                if let Some(editor) = ws.get_active() {
                    editor.go_to(line);
                }
            }
            IdiomEvent::GoToSelect { from, to } => {
                if let Some(editor) = ws.get_active() {
                    editor.go_to_select(from, to);
                }
            }
            IdiomEvent::SelectPath(path) => {
                let result = tree.select_by_path(&path);
                gs.log_if_error(result);
            }
            IdiomEvent::TreeDiagnostics(new) => {
                tree.push_diagnostics(new);
            }
            IdiomEvent::CreateFileOrFolder { name, from_base } => {
                if name.is_empty() {
                    gs.error("File creation requires input!");
                } else {
                    let result = match from_base {
                        true => tree.create_file_or_folder_base(name),
                        false => tree.create_file_or_folder(name),
                    };
                    match result {
                        Ok(new_path) => {
                            tree.sync(gs);
                            if !new_path.is_dir() {
                                match ws.new_at_line(new_path.clone(), 0, gs).await {
                                    Ok(..) => {
                                        gs.insert_mode();
                                        if let Some(editor) = ws.get_active() {
                                            editor.update_status.deny();
                                        }
                                    }
                                    Err(error) => gs.error(error),
                                };
                            }
                            tree.sync(gs);
                            let result = tree.select_by_path(&new_path);
                            gs.log_if_error(result);
                        }
                        Err(error) => gs.error(error),
                    }
                }
            }
            IdiomEvent::RenameFile(name) => {
                if name.is_empty() {
                    gs.error("Rename requires input!");
                } else if let Some(result) = tree.rename_path(name) {
                    match result {
                        Ok((from_path, to_path)) => ws.rename_editors(from_path, to_path, gs),
                        Err(error) => gs.error(error),
                    }
                };
            }
            IdiomEvent::RenamedFile { from_path, to_path } => {
                ws.rename_editors(from_path, to_path, gs);
            }
            IdiomEvent::WorkspaceEdit(edits) => ws.apply_edits(edits, gs),
            IdiomEvent::SetMode(mode) => match mode {
                Mode::Select => gs.select_mode(),
                Mode::Insert => gs.insert_mode(),
            },
            IdiomEvent::CheckLSP(file_type) => {
                ws.check_lsp(file_type, gs).await;
            }
            IdiomEvent::SetLSP(file_type) => {
                if let Err(error) = ws.force_lsp_type_on_active(file_type, gs).await {
                    if !matches!(error, crate::error::IdiomError::LSP(crate::lsp::LSPError::Null)) {
                        gs.error(error);
                    }
                };
            }
            IdiomEvent::FileUpdated(path) => {
                ws.notify_update(path, gs);
            }
            IdiomEvent::InsertText(insert) => {
                if let Some(editor) = ws.get_active() {
                    editor.insert_text_with_relative_offset(insert);
                };
            }
            IdiomEvent::ActivateEditor(idx) => {
                ws.activate_editor(idx, gs);
                gs.insert_mode();
            }
            IdiomEvent::IdiomCommand => {
                if ws.is_empty() && matches!(gs.mode, Mode::Insert) {
                    return;
                }
                Pallet::run_as_command(gs, ws, tree, term);
            }
            IdiomEvent::Rebase => {
                if let Some(editor) = ws.get_active() {
                    editor.rebase(gs);
                }
            }
            IdiomEvent::Save => {
                if let Some(editor) = ws.get_active() {
                    editor.save(gs);
                }
            }
        }
    }
}

impl From<PopupChoice> for IdiomEvent {
    fn from(value: PopupChoice) -> Self {
        IdiomEvent::InplacePopup(StartInplacePopup::Pop(value))
    }
}

impl From<Location> for IdiomEvent {
    fn from(loc: Location) -> Self {
        Self::OpenAtSelect(PathBuf::from(loc.uri.path().as_str()), (loc.range.start.into(), loc.range.end.into()))
    }
}

impl From<LocationLink> for IdiomEvent {
    fn from(loc: LocationLink) -> Self {
        Self::OpenAtSelect(
            PathBuf::from(loc.target_uri.path().as_str()),
            (loc.target_range.start.into(), loc.target_range.end.into()),
        )
    }
}

impl From<WorkspaceEdit> for IdiomEvent {
    fn from(value: WorkspaceEdit) -> Self {
        Self::WorkspaceEdit(value)
    }
}

impl TryFrom<GotoDeclarationResponse> for IdiomEvent {
    type Error = ();
    fn try_from(value: GotoDeclarationResponse) -> Result<Self, ()> {
        Ok(match value {
            GotoDeclarationResponse::Scalar(location) => location.into(),
            GotoDeclarationResponse::Array(mut arr) => {
                if arr.is_empty() {
                    return Err(());
                }
                arr.remove(0).into()
            }
            GotoDeclarationResponse::Link(mut links) => {
                if links.is_empty() {
                    return Err(());
                }
                links.remove(0).into()
            }
        })
    }
}

impl From<PopupSelector<String>> for IdiomEvent {
    fn from(value: PopupSelector<String>) -> Self {
        IdiomEvent::InplacePopup(StartInplacePopup::Mesasge(value))
    }
}

impl From<PopupSelector<(String, PathBuf, Range)>> for IdiomEvent {
    fn from(value: PopupSelector<(String, PathBuf, Range)>) -> Self {
        IdiomEvent::InplacePopup(StartInplacePopup::RefSelector(value))
    }
}
