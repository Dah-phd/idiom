pub mod bash;
pub mod generic;
pub mod json;
pub mod lobster;
pub mod placeholder;
pub mod python;
pub mod rust;
pub mod ts;
use crate::{
    configs::{FileType, Theme},
    syntax::{tokens::set_tokens, Legend},
    workspace::line::EditorLine,
};
use idiom_tui::UTFSafe;
use logos::{Logos, Span};
use lsp_types::{CompletionItem, InsertTextFormat, SemanticToken};

use super::{
    create_semantic_capabilities,
    utils::{full_tokens, NON_TOKEN_ID},
};
use std::{collections::HashSet, fmt::Debug};

pub type PositionedTokenParser<T> = fn(T, Span, &str) -> PositionedToken<T>;

/// Trait to be implemented on the lang specific token, allowing parsing and deriving builtins
pub trait LangStream: Sized + Debug + PartialEq + Logos<'static> {
    fn type_id(&self) -> u32;
    fn modifier(&self) -> u32 {
        0
    }
    fn parse<'a>(
        text: impl Iterator<Item = &'a str>,
        tokens: &mut Vec<Vec<PositionedToken<Self>>>,
        parser: PositionedTokenParser<Self>,
    );

    fn init_definitions() -> Definitions {
        Definitions::default()
    }

    fn objectify(&self) -> ObjType<'_> {
        ObjType::None
    }

    fn init_tokens(content: &mut Vec<EditorLine>, theme: &Theme, file_type: FileType) {
        let text = content.iter().map(|l| l.to_string()).collect::<Vec<_>>();
        let mut tokens = Vec::new();
        Self::parse(text.iter().map(|t| t.as_str()), &mut tokens, PositionedToken::<Self>::utf32);
        let mut legend = Legend::default();
        legend.map_styles(file_type, theme, &create_semantic_capabilities());
        set_tokens(full_tokens(&tokens), &legend, content);
    }
}

#[derive(Debug, PartialEq)]
pub struct PositionedToken<T: LangStream> {
    pub from: usize,
    pub len: usize,
    pub token_type: u32,
    pub modifier: u32,
    lang_token: T,
}

impl<T: LangStream> PositionedToken<T> {
    pub fn utf32(token: T, span: Span, text: &str) -> PositionedToken<T> {
        // utf32 encodingT
        let from = text[..span.start].char_len();
        let len = text[span.start..span.end].char_len();
        PositionedToken { from, len, token_type: token.type_id(), modifier: token.modifier(), lang_token: token }
    }

    pub fn utf8(token: T, span: Span, _text: &str) -> PositionedToken<T> {
        PositionedToken {
            len: span.len(),
            from: span.start,
            token_type: token.type_id(),
            modifier: token.modifier(),
            lang_token: token,
        }
    }

    pub fn utf16(token: T, span: Span, text: &str) -> PositionedToken<T> {
        let from = text[..span.start].utf16_len();
        let len = text[span.start..span.end].utf16_len();
        PositionedToken { from, len, token_type: token.type_id(), modifier: token.modifier(), lang_token: token }
    }

    #[inline]
    pub fn refresh_type(&mut self) {
        self.token_type = self.lang_token.type_id();
        self.modifier = self.lang_token.modifier();
    }

    #[inline]
    pub fn semantic_token(&self, delta_line: u32, at_char: usize) -> SemanticToken {
        SemanticToken {
            delta_line,
            length: self.len as u32,
            delta_start: (self.from - at_char) as u32,
            token_type: self.token_type,
            token_modifiers_bitset: self.modifier,
        }
    }
}

pub fn init_local_tokens(file_type: FileType, content: &mut Vec<EditorLine>, theme: &Theme) {
    match file_type {
        FileType::Rust => rust::Rustacean::init_tokens(content, theme, file_type),
        FileType::Python => python::PyToken::init_tokens(content, theme, file_type),
        FileType::Lobster => lobster::Pincer::init_tokens(content, theme, file_type),
        FileType::JavaScript | FileType::TypeScript => ts::TSToken::init_tokens(content, theme, file_type),
        _ => generic::GenericToken::init_tokens(content, theme, file_type),
    }
}

#[derive(Default)]
pub struct Definitions {
    types: Vec<Struct>,
    function: Vec<Func>,
    variables: Vec<Var>,
    keywords: Vec<&'static str>,
}

impl Definitions {
    pub fn to_completions<T: LangStream>(&self, tokens: &[Vec<PositionedToken<T>>]) -> Vec<CompletionItem> {
        let mut items = self
            .keywords
            .iter()
            .map(|kward| CompletionItem::new_simple((*kward).to_owned(), String::from("Keyword")))
            .collect::<Vec<_>>();

        let mut fn_set = self.function.iter().map(|func| func.name.to_owned()).collect::<HashSet<_>>();
        let mut var_set = self.variables.iter().map(|var| var.name.to_owned()).collect::<HashSet<_>>();
        let mut type_set = self.types.iter().map(|obj_type| obj_type.name.to_owned()).collect::<HashSet<_>>();

        for tok in tokens.iter().flatten() {
            match tok.lang_token.objectify() {
                ObjType::Var(name) if name.len() > 2 => {
                    var_set.insert(name.to_owned());
                }
                ObjType::Fn(name) if name.len() > 2 => {
                    fn_set.insert(name.to_owned());
                }
                ObjType::Struct(name) if name.len() > 2 => {
                    type_set.insert(name.to_owned());
                }
                _ => (),
            }
        }

        for func in fn_set.into_iter() {
            items.push(CompletionItem {
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                insert_text: Some(format!("{func}($0)")),
                label: func,
                detail: Some("Function".to_owned()),
                ..Default::default()
            });
        }

        for var in var_set.into_iter() {
            items.push(CompletionItem::new_simple(var, "Variable".to_owned()));
        }

        for type_name in type_set.into_iter() {
            items.push(CompletionItem::new_simple(type_name, "Type".to_owned()));
        }

        items
    }
}

pub enum ObjType<'a> {
    Fn(&'a str),
    Var(&'a str),
    Struct(&'a str),
    None,
}

struct Struct {
    name: String,
}

impl Struct {
    fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[derive(Default)]
struct Func {
    name: String,
}

struct Var {
    name: String,
}
