use lsp_types::{CompletionItem, CompletionTextEdit, InsertReplaceEdit, InsertTextFormat, Range};

use super::events::IdiomEvent;

fn insert_replace_completion_event(replace_text: impl Into<String>) -> IdiomEvent {
    CompletionItem {
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        text_edit: Some(CompletionTextEdit::InsertAndReplace(InsertReplaceEdit {
            new_text: replace_text.into(),
            insert: Range::default(),
            replace: Range::default(),
        })),
        ..Default::default()
    }
    .into()
}

fn insert_text_completion_event(replace_text: impl Into<String>) -> IdiomEvent {
    CompletionItem {
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        insert_text: Some(replace_text.into()),
        ..Default::default()
    }
    .into()
}

#[test]
fn test_snippets_insert_text() {
    assert_eq!(
        IdiomEvent::Snippet(String::from("push(value)"), Some((0, 11))),
        insert_text_completion_event("push(${1:value})$0")
    );
    assert_eq!(
        IdiomEvent::Snippet(String::from("min(other)"), Some((0, 10))),
        insert_text_completion_event("min(${1:other})$0")
    );
    assert_eq!(
        IdiomEvent::Snippet(String::from("put_uint_le(n, nbytes)"), Some((0, 22))),
        insert_text_completion_event("put_uint_le(${1:n}, ${2:nbytes})$0")
    );
    assert_eq!(
        IdiomEvent::Snippet(String::from("fn () {\n    \n}"), Some((0, 3))),
        insert_text_completion_event("fn $1($2) {\n    $0\n}")
    );
}

#[test]
fn test_snippets_insert_and_replace() {
    assert_eq!(
        IdiomEvent::Snippet(String::from("push(value)"), Some((0, 11))),
        insert_replace_completion_event("push(${1:value})$0")
    );
    assert_eq!(
        IdiomEvent::Snippet(String::from("min(other)"), Some((0, 10))),
        insert_replace_completion_event("min(${1:other})$0")
    );
    assert_eq!(
        IdiomEvent::Snippet(String::from("put_uint_le(n, nbytes)"), Some((0, 22))),
        insert_replace_completion_event("put_uint_le(${1:n}, ${2:nbytes})$0")
    );
    assert_eq!(
        IdiomEvent::Snippet(String::from("fn () {\n    \n}"), Some((0, 3))),
        insert_replace_completion_event("fn $1($2) {\n    $0\n}")
    );
}
