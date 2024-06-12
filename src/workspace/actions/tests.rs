use lsp_types::{Position, Range, TextDocumentContentChangeEvent};

use crate::workspace::actions::{Edit, EditMetaData};
use crate::workspace::cursor::Cursor;
use crate::workspace::line::EditorLine;
use crate::workspace::CursorPosition;
use crate::{configs::IndentConfigs, workspace::line::CodeLine};

pub fn create_content() -> Vec<CodeLine> {
    vec![
        "here comes the text".into(),                                                            // 0
        "more lines of code should be here but only text".into(),                                // 1
        "I don't know how many lines I plan to add to this test but lets say at least 5".into(), // 2
        "Hope nobody is reading this ... knowing my luck someone will".into(),                   // 3
        "🚀 things will get really complicated especially with all the utf8 chars and utf16 pos encoding".into(), // 4
        "there will be 🚀 everywhere in the end".into(),                                         // 5
        "i will have to have some scopes {".into(),                                              // 6
        "    this is the first scope".into(),                                                    // 7
        "}".into(),                                                                              // 8
        "scope is closed!".into(),                                                               // 9
    ]
}

fn match_line(l1: &impl ToString, l2: &impl ToString) {
    assert_eq!(l1.to_string(), l2.to_string())
}

fn assert_initial(content: &[CodeLine]) {
    let init_state = create_content();
    assert_eq!(content.len(), init_state.len());
    for (og, new) in init_state.iter().zip(content.iter()) {
        match_line(og, new);
    }
}

fn assert_edits_applicable(mut content: Vec<CodeLine>, edits: Vec<Edit>) {
    // ensure every event can be undone and redone
    let reseved_content: Vec<CodeLine> = content.iter().map(|cl| CodeLine::new(cl.to_string())).collect();
    for edit in edits.iter().rev() {
        edit.apply_rev(&mut content, &mut vec![]);
    }
    assert_initial(&content);
    for edit in edits.iter() {
        edit.apply(&mut content, &mut vec![]);
    }
    assert_eq!(reseved_content.len(), content.len());
    for (reserved, reupdated) in reseved_content.iter().zip(content.iter()) {
        match_line(reserved, reupdated);
    }
}

/// Edits

#[test]
fn test_new_line() {
    let cfg = IndentConfigs::default();

    let mut content = vec![CodeLine::new("        ".to_owned())];
    let (cursor, edit) = Edit::new_line(CursorPosition { line: 0, char: 8 }, &cfg, &mut content);
    assert_eq!(cursor, CursorPosition { line: 1, char: 8 });
    assert_eq!(content.len(), 2);
    assert_eq!(&content[0].to_string(), "");
    assert_eq!(&content[1].to_string(), "        ");
    edit.apply_rev(&mut content, &mut vec![]);
    assert_eq!(content.len(), 1);
    assert_eq!(&content[0].to_string(), "        ");

    let mut content = create_content();

    // simple new line
    let (cursor, edit) = Edit::new_line(CursorPosition { line: 0, char: 4 }, &cfg, &mut content);
    let mut edits = vec![edit];
    assert_eq!(CursorPosition { line: 1, char: 0 }, cursor);

    // scope
    edits.push(Edit::insert_clip(cursor, "{}".to_owned(), &mut content));
    let (cursor, edit) = Edit::new_line(CursorPosition { line: 1, char: 1 }, &cfg, &mut content);
    edits.push(edit);
    assert_eq!(CursorPosition { line: 2, char: 4 }, cursor);
    assert_eq!(&content[1].to_string(), "{");
    assert_eq!(&content[2].to_string(), "    ");
    assert_eq!(&content[3].to_string(), "} comes the text");

    // double scope
    edits.push(Edit::insert_clip(cursor, "[]".to_owned(), &mut content));
    let (cursor, edit) = Edit::new_line(CursorPosition { line: 2, char: 5 }, &cfg, &mut content);
    edits.push(edit);
    assert_eq!(CursorPosition { line: 3, char: 8 }, cursor);
    assert_eq!(&content[1].to_string(), "{");
    assert_eq!(&content[2].to_string(), "    [");
    assert_eq!(&content[3].to_string(), "        ");
    assert_eq!(&content[4].to_string(), "    ]");
    assert_eq!(&content[5].to_string(), "} comes the text");
    assert_edits_applicable(content, edits);
}

#[test]
fn test_swap_lines() {
    let mut content = create_content();
    let cfg = IndentConfigs::default();
    let (.., edit) = Edit::swap_down(7, &cfg, &mut content);
    match_line(&content[7], &"}");
    match_line(&content[8], &"    this is the first scope");
    edit.apply_rev(&mut content, &mut vec![]);
    match_line(&content[7], &"    this is the first scope");
    match_line(&content[8], &"}");
    let (.., edit) = Edit::swap_down(6, &cfg, &mut content);
    match_line(&content[6], &"    this is the first scope");
    match_line(&content[7], &"    i will have to have some scopes {");
    edit.apply_rev(&mut content, &mut vec![]);
    match_line(&content[6], &"i will have to have some scopes {");
    match_line(&content[7], &"    this is the first scope");
}

#[test]
fn test_merge_next_line() {
    let mut content = create_content();
    let mut edits = vec![];
    edits.push(Edit::merge_next_line(0, &mut content));
    match_line(&content[0], &"here comes the textmore lines of code should be here but only text");
    edits.push(Edit::merge_next_line(5, &mut content));
    edits.push(Edit::merge_next_line(5, &mut content));
    match_line(&content[5], &"i will have to have some scopes {    this is the first scope}");
    assert_edits_applicable(content, edits);
}

#[test]
fn test_indent_unindent() {
    let mut content = create_content();
    let cfg = IndentConfigs::default();
    Edit::unindent(7, &mut content[7], &cfg.indent);
    match_line(&content[7], &"this is the first scope");
    let mut this_line: CodeLine = "     text".into();
    Edit::unindent(0, &mut this_line, &cfg.indent);
    match_line(&this_line, &"    text");
}

#[test]
fn test_record_inline_insert() {
    let this_line: CodeLine = "text".into();
    let mut content = vec![this_line];
    let test_ins = String::from("    ");
    content[0].insert_str(0, &test_ins);
    let edit = Edit::record_in_line_insertion(Position::new(0, 0), test_ins.clone());
    match_line(&content[0], &"    text");
    edit.apply_rev(&mut content, &mut vec![]);
    match_line(&content[0], &"text");
    content[0].insert_str(2, &test_ins);
    let edit = Edit::record_in_line_insertion(Position::new(0, 2), test_ins);
    match_line(&content[0], &"te    xt");
    edit.apply_rev(&mut content, &mut vec![]);
    match_line(&content[0], &"text");
}

#[test]
fn test_remove_from_line() {
    let mut content = create_content();
    let mut edits = vec![Edit::remove_from_line(5, 2, 4, &mut content[5])];
    match_line(&content[5], &"the will be 🚀 everywhere in the end");
    edits.push(Edit::remove_from_line(5, 13, 20, &mut content[5]));
    match_line(&content[5], &"the will be 🚀here in the end");
    edits.push(Edit::remove_from_line(5, 10, 13, &mut content[5]));
    match_line(&content[5], &"the will bhere in the end");
    assert_edits_applicable(content, edits);
}

#[test]
fn test_insert_clip() {
    let mut content = create_content();
    let clippy = "text".to_owned();
    let big_clippy = "text\n\ntext\n".to_owned();
    let mut edits = vec![];
    edits.push(Edit::insert_clip(CursorPosition { line: 5, char: 15 }, clippy, &mut content));
    match_line(&content[5], &"there will be 🚀text everywhere in the end");
    edits.push(Edit::insert_clip(CursorPosition { line: 5, char: 14 }, big_clippy, &mut content));
    match_line(&content[5], &"there will be text");
    match_line(&content[6], &"");
    match_line(&content[7], &"text");
    match_line(&content[8], &"🚀text everywhere in the end");
    assert_edits_applicable(content, edits);
}

#[test]
fn test_remove_line() {
    let mut content = create_content();
    let edits = vec![Edit::remove_line(4, &mut content), Edit::remove_line(4, &mut content)];
    match_line(&content[4], &"i will have to have some scopes {");
    assert_edits_applicable(content, edits);
}

#[test]
fn test_remove_select() {
    let mut content = create_content();
    let edits = vec![
        Edit::remove_select(CursorPosition { line: 0, char: 0 }, CursorPosition { line: 0, char: 6 }, &mut content),
        Edit::remove_select(CursorPosition { line: 5, char: 15 }, CursorPosition { line: 7, char: 2 }, &mut content),
    ];
    match_line(&content[0], &"omes the text");
    match_line(&content[5], &"there will be 🚀  this is the first scope");
    assert_edits_applicable(content, edits);
}

#[test]
fn test_replace_select() {
    let mut content = create_content();
    let edits = vec![
        Edit::replace_select(
            CursorPosition { line: 0, char: 0 },
            CursorPosition { line: 0, char: 6 },
            "bumba".to_owned(),
            &mut content,
        ),
        Edit::replace_select(
            CursorPosition { line: 5, char: 15 },
            CursorPosition { line: 7, char: 2 },
            "text\ntext\ntext".to_owned(),
            &mut content,
        ),
    ];
    match_line(&content[0], &"bumbaomes the text");
    match_line(&content[5], &"there will be 🚀text");
    match_line(&content[6], &"text");
    match_line(&content[7], &"text  this is the first scope");
    assert_edits_applicable(content, edits);
}

#[test]
fn test_replace_token() {
    let mut content = create_content();
    let edits = vec![
        Edit::replace_token(0, 1, "bumba".to_owned(), &mut content),
        Edit::replace_token(1, 7, "tubrak".to_owned(), &mut content),
    ];
    match_line(&content[0], &"bumba comes the text");
    match_line(&content[1], &"more tubrak of code should be here but only text");
    assert_edits_applicable(content, edits);
}

#[test]
fn test_insert_snippet() {
    let mut content = create_content();
    let cfg = IndentConfigs::default();
    let mut cursor = Cursor::default();
    cursor.set_position((7, 5).into());
    let (pos, edit) = Edit::insert_snippet(&cursor, "text() {\n    \n}".to_owned(), Some((1, 0)), &cfg, &mut content);
    let mut edits = vec![edit];
    match_line(&content[7], &"    text() {");
    match_line(&content[8], &"        ");
    match_line(&content[9], &"    } is the first scope");
    assert_eq!(pos, CursorPosition { line: 8, char: 4 });
    cursor.set_position((0, 6).into());
    let (pos, edit) = Edit::insert_snippet(&cursor, "text() {\n    \n}".to_owned(), None, &cfg, &mut content);
    edits.push(edit);
    match_line(&content[0], &"here text() {");
    match_line(&content[1], &"    ");
    match_line(&content[2], &"} the text");
    assert_eq!(pos, CursorPosition { line: 2, char: 1 });
    assert_edits_applicable(content, edits);
}

#[test]
fn test_utf8_lsp_event() {
    let mut content = vec![
        CodeLine::new("test".into()),
        CodeLine::new("test".into()),
        CodeLine::new("test".into()),
        CodeLine::new("test".into()),
    ];
    let edit = Edit::insert_clip((0, 4).into(), "\ntest".to_owned(), &mut content);
    let (.., mut event) = edit.event();
    let (.., mut rev_event) = edit.reverse_event();
    event.utf8_encode_start(&content);
    rev_event.utf8_encode_start(&content);
    assert_eq!(
        TextDocumentContentChangeEvent {
            range: Some(Range::new(Position::new(0, 4), Position::new(0, 4))),
            text: "\ntest".to_owned(),
            range_length: None
        },
        event.utf8_text_change()
    );
    assert_eq!(
        TextDocumentContentChangeEvent {
            range: Some(Range::new(Position::new(0, 4), Position::new(1, 4))),
            text: "".to_owned(),
            range_length: None
        },
        rev_event.utf8_text_change()
    );
    let edit = Edit::replace_select((0, 2).into(), (4, 1).into(), "\nbambi\nbull".to_owned(), &mut content);
    let (.., mut event) = edit.event();
    let (.., mut rev_event) = edit.reverse_event();
    event.utf8_encode_start(&content);
    rev_event.utf8_encode_start(&content);
    assert_eq!(
        TextDocumentContentChangeEvent {
            range: Some(Range::new(Position::new(0, 2), Position::new(4, 1))),
            text: "\nbambi\nbull".to_owned(),
            range_length: None
        },
        event.utf8_text_change()
    );
    assert_eq!(
        TextDocumentContentChangeEvent {
            range: Some(Range::new(Position::new(0, 2), Position::new(2, 4))),
            text: "st\ntest\ntest\ntest\nt".to_owned(),
            range_length: None
        },
        rev_event.utf8_text_change()
    );
}

// LSP events
#[test]
fn test_utf8_lsp_event_emoji() {
    let mut content = vec![
        CodeLine::new("🚀est".into()),
        CodeLine::new("test".into()),
        CodeLine::new("test".into()),
        CodeLine::new("test".into()),
    ];
    let edit = Edit::insert_clip((0, 4).into(), "\ntest🚀".to_owned(), &mut content);
    let (.., mut event) = edit.event();
    let (.., mut rev_event) = edit.reverse_event();
    event.utf8_encode_start(&content);
    rev_event.utf8_encode_start(&content);
    assert_eq!(
        TextDocumentContentChangeEvent {
            range: Some(Range::new(Position::new(0, 7), Position::new(0, 7))),
            text: "\ntest🚀".to_owned(),
            range_length: None
        },
        event.utf8_text_change()
    );
    assert_eq!(
        TextDocumentContentChangeEvent {
            range: Some(Range::new(Position::new(0, 7), Position::new(1, 8))),
            text: "".to_owned(),
            range_length: None
        },
        rev_event.utf8_text_change()
    );
    let edit = Edit::replace_select((0, 2).into(), (4, 1).into(), "\nbambi\nbull🚀".to_owned(), &mut content);
    let (.., mut event) = edit.event();
    let (.., mut rev_event) = edit.reverse_event();
    event.utf8_encode_start(&content);
    rev_event.utf8_encode_start(&content);
    assert_eq!(
        TextDocumentContentChangeEvent {
            range: Some(Range::new(Position::new(0, 5), Position::new(4, 1))),
            text: "\nbambi\nbull🚀".to_owned(),
            range_length: None
        },
        event.utf8_text_change()
    );
    assert_eq!(
        TextDocumentContentChangeEvent {
            range: Some(Range::new(Position::new(0, 5), Position::new(2, 8))),
            text: "st\ntest🚀\ntest\ntest\nt".to_owned(),
            range_length: None
        },
        rev_event.utf8_text_change()
    );
}

#[test]
fn test_utf16_lsp_event() {
    let mut content = vec![
        CodeLine::new("test".into()),
        CodeLine::new("test".into()),
        CodeLine::new("test".into()),
        CodeLine::new("test".into()),
    ];
    let edit = Edit::insert_clip((0, 4).into(), "\ntest".to_owned(), &mut content);
    let (.., mut event) = edit.event();
    let (.., mut rev_event) = edit.reverse_event();
    event.utf16_encode_start(&content);
    rev_event.utf16_encode_start(&content);
    assert_eq!(
        TextDocumentContentChangeEvent {
            range: Some(Range::new(Position::new(0, 4), Position::new(0, 4))),
            text: "\ntest".to_owned(),
            range_length: None
        },
        event.utf16_text_change()
    );
    assert_eq!(
        TextDocumentContentChangeEvent {
            range: Some(Range::new(Position::new(0, 4), Position::new(1, 4))),
            text: "".to_owned(),
            range_length: None
        },
        rev_event.utf16_text_change()
    );
    let edit = Edit::replace_select((0, 2).into(), (4, 1).into(), "\nbambi\nbull".to_owned(), &mut content);
    let (.., mut event) = edit.event();
    let (.., mut rev_event) = edit.reverse_event();
    event.utf16_encode_start(&content);
    rev_event.utf16_encode_start(&content);
    assert_eq!(
        TextDocumentContentChangeEvent {
            range: Some(Range::new(Position::new(0, 2), Position::new(4, 1))),
            text: "\nbambi\nbull".to_owned(),
            range_length: None
        },
        event.utf16_text_change()
    );
    assert_eq!(
        TextDocumentContentChangeEvent {
            range: Some(Range::new(Position::new(0, 2), Position::new(2, 4))),
            text: "st\ntest\ntest\ntest\nt".to_owned(),
            range_length: None
        },
        rev_event.utf16_text_change()
    );
}

#[test]
fn test_utf16_lsp_event_emoji() {
    let mut content = vec![
        CodeLine::new("🚀est".into()),
        CodeLine::new("test".into()),
        CodeLine::new("test".into()),
        CodeLine::new("test".into()),
    ];
    let edit = Edit::insert_clip((0, 4).into(), "\ntest🚀".to_owned(), &mut content);
    let (.., mut event) = edit.event();
    let (.., mut rev_event) = edit.reverse_event();
    event.utf16_encode_start(&content);
    rev_event.utf16_encode_start(&content);
    assert_eq!(
        TextDocumentContentChangeEvent {
            range: Some(Range::new(Position::new(0, 5), Position::new(0, 5))),
            text: "\ntest🚀".to_owned(),
            range_length: None
        },
        event.utf16_text_change()
    );
    assert_eq!(
        TextDocumentContentChangeEvent {
            range: Some(Range::new(Position::new(0, 5), Position::new(1, 6))),
            text: "".to_owned(),
            range_length: None
        },
        rev_event.utf16_text_change()
    );
    let edit = Edit::replace_select((0, 2).into(), (4, 1).into(), "\nbambi\nbull🚀".to_owned(), &mut content);
    let (.., mut event) = edit.event();
    let (.., mut rev_event) = edit.reverse_event();
    event.utf16_encode_start(&content);
    rev_event.utf16_encode_start(&content);
    assert_eq!(
        TextDocumentContentChangeEvent {
            range: Some(Range::new(Position::new(0, 3), Position::new(4, 1))),
            text: "\nbambi\nbull🚀".to_owned(),
            range_length: None
        },
        event.utf16_text_change()
    );
    assert_eq!(
        TextDocumentContentChangeEvent {
            range: Some(Range::new(Position::new(0, 3), Position::new(2, 6))),
            text: "st\ntest🚀\ntest\ntest\nt".to_owned(),
            range_length: None
        },
        rev_event.utf16_text_change()
    );
}

#[test]
fn test_utf32_lsp_event() {
    let mut content = vec![
        CodeLine::new("test".into()),
        CodeLine::new("test".into()),
        CodeLine::new("test".into()),
        CodeLine::new("test".into()),
    ];
    let edit = Edit::insert_clip((0, 4).into(), "\ntest".to_owned(), &mut content);
    let (.., event) = edit.event();
    let (.., rev_event) = edit.reverse_event();
    assert_eq!(
        TextDocumentContentChangeEvent {
            range: Some(Range::new(Position::new(0, 4), Position::new(0, 4))),
            text: "\ntest".to_owned(),
            range_length: None
        },
        event.utf32_text_change()
    );
    assert_eq!(
        TextDocumentContentChangeEvent {
            range: Some(Range::new(Position::new(0, 4), Position::new(1, 4))),
            text: "".to_owned(),
            range_length: None
        },
        rev_event.utf32_text_change()
    );
    let edit = Edit::replace_select((0, 2).into(), (4, 1).into(), "\nbambi\nbull".to_owned(), &mut content);
    let (.., event) = edit.event();
    let (.., rev_event) = edit.reverse_event();
    assert_eq!(
        TextDocumentContentChangeEvent {
            range: Some(Range::new(Position::new(0, 2), Position::new(4, 1))),
            text: "\nbambi\nbull".to_owned(),
            range_length: None
        },
        event.utf32_text_change()
    );
    assert_eq!(
        TextDocumentContentChangeEvent {
            range: Some(Range::new(Position::new(0, 2), Position::new(2, 4))),
            text: "st\ntest\ntest\ntest\nt".to_owned(),
            range_length: None
        },
        rev_event.utf32_text_change()
    );
}

#[test]
fn test_utf32_lsp_event_emoji() {
    let mut content = vec![
        CodeLine::new("🚀est".into()),
        CodeLine::new("test".into()),
        CodeLine::new("test".into()),
        CodeLine::new("test".into()),
    ];
    let edit = Edit::insert_clip((0, 4).into(), "\ntest🚀".to_owned(), &mut content);
    let (.., event) = edit.event();
    let (.., rev_event) = edit.reverse_event();
    assert_eq!(
        TextDocumentContentChangeEvent {
            range: Some(Range::new(Position::new(0, 4), Position::new(0, 4))),
            text: "\ntest🚀".to_owned(),
            range_length: None
        },
        event.utf32_text_change()
    );
    assert_eq!(
        TextDocumentContentChangeEvent {
            range: Some(Range::new(Position::new(0, 4), Position::new(1, 5))),
            text: "".to_owned(),
            range_length: None
        },
        rev_event.utf32_text_change()
    );
    let edit = Edit::replace_select((0, 2).into(), (4, 1).into(), "\nbambi\nbull🚀".to_owned(), &mut content);
    let (.., event) = edit.event();
    let (.., rev_event) = edit.reverse_event();
    assert_eq!(
        TextDocumentContentChangeEvent {
            range: Some(Range::new(Position::new(0, 2), Position::new(4, 1))),
            text: "\nbambi\nbull🚀".to_owned(),
            range_length: None
        },
        event.utf32_text_change()
    );
    assert_eq!(
        TextDocumentContentChangeEvent {
            range: Some(Range::new(Position::new(0, 2), Position::new(2, 5))),
            text: "st\ntest🚀\ntest\ntest\nt".to_owned(),
            range_length: None
        },
        rev_event.utf32_text_change()
    );
}

/// MetaData

#[test]
fn add_meta_data() {
    assert_eq!(
        EditMetaData::line_changed(1) + EditMetaData::line_changed(1),
        EditMetaData { start_line: 1, from: 0, to: 1 }
    );
    assert_eq!(
        EditMetaData::line_changed(1) + EditMetaData { start_line: 1, from: 1, to: 3 },
        EditMetaData { start_line: 1, from: 0, to: 3 }
    );
    assert_eq!(
        EditMetaData { start_line: 1, from: 2, to: 1 } + EditMetaData { start_line: 1, from: 1, to: 3 },
        EditMetaData { start_line: 1, from: 0, to: 3 }
    );
    assert_eq!(
        EditMetaData { start_line: 1, from: 1, to: 2 } + EditMetaData { start_line: 1, from: 1, to: 3 },
        EditMetaData { start_line: 1, from: 0, to: 3 }
    );
    assert_eq!(
        EditMetaData { start_line: 2, from: 1, to: 3 } + EditMetaData { start_line: 0, from: 3, to: 1 },
        EditMetaData { start_line: 0, from: 0, to: 3 }
    );
    assert_eq!(
        EditMetaData { start_line: 0, from: 1, to: 10 } + EditMetaData { start_line: 2, from: 2, to: 1 },
        EditMetaData { start_line: 0, from: 0, to: 9 },
    );
}

#[test]
fn add_assign_meta_data() {
    let mut edit = EditMetaData::line_changed(1);
    edit += EditMetaData::line_changed(1);
    assert_eq!(edit, EditMetaData { start_line: 1, from: 0, to: 1 });
    let mut edit = EditMetaData::line_changed(1);
    edit += EditMetaData { start_line: 1, from: 1, to: 3 };
    assert_eq!(edit, EditMetaData { start_line: 1, from: 0, to: 3 });
    let mut edit = EditMetaData { start_line: 1, from: 2, to: 1 };
    edit += EditMetaData { start_line: 1, from: 1, to: 3 };
    assert_eq!(edit, EditMetaData { start_line: 1, from: 0, to: 3 });
    let mut edit = EditMetaData { start_line: 1, from: 1, to: 2 };
    edit += EditMetaData { start_line: 1, from: 1, to: 3 };
    assert_eq!(edit, EditMetaData { start_line: 1, from: 0, to: 3 });
    let mut edit = EditMetaData { start_line: 2, from: 1, to: 3 };
    edit += EditMetaData { start_line: 0, from: 3, to: 1 };
    assert_eq!(edit, EditMetaData { start_line: 0, from: 0, to: 3 });
    let mut edit = EditMetaData { start_line: 0, from: 1, to: 10 };
    edit += EditMetaData { start_line: 2, from: 2, to: 1 };
    assert_eq!(edit, EditMetaData { start_line: 0, from: 0, to: 9 },);
}