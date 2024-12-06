use super::snippets::parse_completion_item;
use crate::global_state::IdiomEvent;
use lsp_types::{CompletionItem, CompletionTextEdit, InsertReplaceEdit, InsertTextFormat, Range};

fn insert_replace_completion_event(replace_text: impl Into<String>) -> IdiomEvent {
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

fn insert_text_completion_event(replace_text: impl Into<String>) -> IdiomEvent {
    parse_completion_item(CompletionItem {
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        insert_text: Some(replace_text.into()),
        ..Default::default()
    })
}

#[test]
fn test_snippets_insert_text() {
    assert_eq!(
        IdiomEvent::Snippet {
            snippet: String::from("push(value)"),
            cursor_offset: Some((0, 11)),
            relative_select: Some(((0, 5), 5))
        },
        insert_text_completion_event("push(${1:value})$0")
    );
    assert_eq!(
        IdiomEvent::Snippet {
            snippet: String::from("min(other)"),
            cursor_offset: Some((0, 10)),
            relative_select: Some(((0, 4), 5))
        },
        insert_text_completion_event("min(${1:other})$0")
    );
    assert_eq!(
        IdiomEvent::Snippet {
            snippet: String::from("put_uint_le(n, nbytes)"),
            cursor_offset: Some((0, 22)),
            relative_select: Some(((0, 12), 1))
        },
        insert_text_completion_event("put_uint_le(${1:n}, ${2:nbytes})$0")
    );
    assert_eq!(
        IdiomEvent::Snippet {
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
        IdiomEvent::Snippet {
            snippet: String::from("push(value)"),
            cursor_offset: Some((0, 11)),
            relative_select: Some(((0, 5), 5))
        },
        insert_replace_completion_event("push(${1:value})$0")
    );
    assert_eq!(
        IdiomEvent::Snippet {
            snippet: String::from("min(other)"),
            cursor_offset: Some((0, 10)),
            relative_select: Some(((0, 4), 5))
        },
        insert_replace_completion_event("min(${1:other})$0")
    );
    assert_eq!(
        IdiomEvent::Snippet {
            snippet: String::from("put_uint_le(n, nbytes)"),
            cursor_offset: Some((0, 22)),
            relative_select: Some(((0, 12), 1))
        },
        insert_replace_completion_event("put_uint_le(${1:n}, ${2:nbytes})$0")
    );
    assert_eq!(
        IdiomEvent::Snippet {
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
        IdiomEvent::Snippet { snippet: String::from("echo \"$DATA\""), cursor_offset: None, relative_select: None },
        insert_replace_completion_event("echo \"$DATA\""),
    );
    assert_eq!(
        IdiomEvent::Snippet {
            snippet: String::from("echo \"$DATA\" + ${something}"),
            cursor_offset: Some((0, 27)),
            relative_select: None
        },
        insert_replace_completion_event("echo \"$DATA\" + ${something}$0")
    );
}

#[test]
fn bad_input_snippets() {
    assert_eq!(
        IdiomEvent::Snippet {
            snippet: String::from("vary bad ${3:imp\n    }went wrong\n end"),
            cursor_offset: Some((1, 15)),
            relative_select: None
        },
        insert_replace_completion_event("vary bad ${3:imp\n    }went wrong$0\n end"),
    );
    assert_eq!(
        IdiomEvent::Snippet {
            snippet: String::from("vary bad ${3:imp\n    }went wrongana\n end"),
            cursor_offset: None,
            relative_select: Some(((1, 15), 3)),
        },
        insert_text_completion_event("vary bad ${3:imp\n    }went wrong${1:ana}\n end"),
    );
}
