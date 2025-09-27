use super::super::ModalAction;
use super::snippets::parse_completion_item;
use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionTextEdit, InsertReplaceEdit, InsertTextFormat, Position, Range,
};

fn insert_replace_completion_event(replace_text: impl Into<String>) -> ModalAction {
    parse_completion_item(CompletionItem {
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        text_edit: Some(CompletionTextEdit::InsertAndReplace(InsertReplaceEdit {
            new_text: replace_text.into(),
            insert: Range::default(),
            replace: Range::default(),
        })),
        ..Default::default()
    })
}

fn insert_text_completion_event(replace_text: impl Into<String>) -> ModalAction {
    parse_completion_item(CompletionItem {
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        insert_text: Some(replace_text.into()),
        ..Default::default()
    })
}

#[test]
fn test_snippets_insert_text() {
    assert_eq!(
        ModalAction::Snippet {
            snippet: String::from("push(value)"),
            cursor_offset: Some((0, 11)),
            relative_select: Some(((0, 5), 5))
        },
        insert_text_completion_event("push(${1:value})$0")
    );
    assert_eq!(
        ModalAction::Snippet {
            snippet: String::from("min(other)"),
            cursor_offset: Some((0, 10)),
            relative_select: Some(((0, 4), 5))
        },
        insert_text_completion_event("min(${1:other})$0")
    );
    assert_eq!(
        ModalAction::Snippet {
            snippet: String::from("put_uint_le(n, nbytes)"),
            cursor_offset: Some((0, 22)),
            relative_select: Some(((0, 12), 1))
        },
        insert_text_completion_event("put_uint_le(${1:n}, ${2:nbytes})$0")
    );
    assert_eq!(
        ModalAction::Snippet {
            snippet: String::from("fn () {\n    \n}"),
            cursor_offset: Some((0, 3)),
            relative_select: None
        },
        insert_text_completion_event("fn $1($2) {\n    $0\n}")
    );
}

#[test]
fn test_snippets_insert_and_replace() {
    assert_eq!(
        ModalAction::Snippet {
            snippet: String::from("push(value)"),
            cursor_offset: Some((0, 11)),
            relative_select: Some(((0, 5), 5))
        },
        insert_replace_completion_event("push(${1:value})$0")
    );
    assert_eq!(
        ModalAction::Snippet {
            snippet: String::from("min(other)"),
            cursor_offset: Some((0, 10)),
            relative_select: Some(((0, 4), 5))
        },
        insert_replace_completion_event("min(${1:other})$0")
    );
    assert_eq!(
        ModalAction::Snippet {
            snippet: String::from("put_uint_le(n, nbytes)"),
            cursor_offset: Some((0, 22)),
            relative_select: Some(((0, 12), 1))
        },
        insert_replace_completion_event("put_uint_le(${1:n}, ${2:nbytes})$0")
    );
    assert_eq!(
        ModalAction::Snippet {
            snippet: String::from("fn () {\n    \n}"),
            cursor_offset: Some((0, 3)),
            relative_select: None
        },
        insert_replace_completion_event("fn $1($2) {\n    $0\n}")
    );
}

#[test]
fn test_hard_snippet() {
    // this is not likely to happen in real lsp application but good as testing scenarion
    assert_eq!(
        ModalAction::Snippet { snippet: String::from("echo \"$DATA\""), cursor_offset: None, relative_select: None },
        insert_replace_completion_event("echo \"$DATA\""),
    );
    assert_eq!(
        ModalAction::Snippet {
            snippet: String::from("echo \"$DATA\" + ${something}"),
            cursor_offset: Some((0, 27)),
            relative_select: None
        },
        insert_replace_completion_event("echo \"$DATA\" + ${something}$0")
    );
}

#[test]
fn test_brach_snippet() {
    assert_eq!(
        ModalAction::Snippet {
            snippet: String::from("CursorPosition { line: , char:  }"),
            cursor_offset: Some((0, 23)),
            relative_select: None
        },
        insert_text_completion_event("CursorPosition { line: ${1:()}, char: ${2:()} }$0"),
    )
}

#[test]
fn bad_input_snippets() {
    assert_eq!(
        ModalAction::Snippet {
            snippet: String::from("vary bad ${3:imp\n    }went wrong\n end"),
            cursor_offset: Some((1, 15)),
            relative_select: None
        },
        insert_replace_completion_event("vary bad ${3:imp\n    }went wrong$0\n end"),
    );
    assert_eq!(
        ModalAction::Snippet {
            snippet: String::from("vary bad ${3:imp\n    }went wrongana\n end"),
            cursor_offset: None,
            relative_select: Some(((1, 15), 3)),
        },
        insert_text_completion_event("vary bad ${3:imp\n    }went wrong${1:ana}\n end"),
    );
}

#[test]
fn ref_multi_var_snippet() {
    let completion_item = CompletionItem {
        label: "draw(…)".to_owned(),
        label_details: None,
        kind: Some(CompletionItemKind::METHOD),
        detail: None,
        documentation: None,
        deprecated: None,
        preselect: Some(true),
        sort_text: Some("7fffffff".to_owned()),
        filter_text: Some("draw".to_owned()),
        insert_text: None,
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        insert_text_mode: None,
        text_edit: Some(CompletionTextEdit::InsertAndReplace(InsertReplaceEdit {
            new_text: "draw(${1:&mut workspace}, ${2:&mut tree}, ${3:&mut term});$0".to_owned(),
            insert: Range { start: Position { line: 133, character: 11 }, end: Position { line: 133, character: 11 } },
            replace: Range { start: Position { line: 133, character: 11 }, end: Position { line: 133, character: 11 } },
        })),
        additional_text_edits: None,
        command: None,
        commit_characters: None,
        data: None,
        tags: None,
    };
    assert_eq!(
        ModalAction::Snippet {
            snippet: String::from("draw(&mut workspace, &mut tree, &mut term);"),
            cursor_offset: Some((0, 43)),
            relative_select: Some(((0, 5), 14)),
        },
        parse_completion_item(completion_item),
    );
}
