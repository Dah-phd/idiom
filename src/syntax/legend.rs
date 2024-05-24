use super::Lang;
use crate::render::backend::{color, Color};
use crate::{configs::FileType, syntax::Theme};
use lsp_types::SemanticTokensServerCapabilities;

#[derive(Clone, Copy, Debug)]
pub enum ColorResult {
    Final(Color),
    KeyWord,
}

impl Default for ColorResult {
    fn default() -> Self {
        Self::Final(color::reset())
    }
}

#[derive(Default, Debug)]
pub struct Legend {
    legend: Vec<ColorResult>,
}

impl Legend {
    pub fn parse_to_color(&self, token_type: usize, theme: &Theme, lang: &Lang, word: &str) -> Color {
        match self.legend.get(token_type) {
            Some(ColorResult::KeyWord) => {
                if lang.is_flow(word) {
                    return theme.flow_control;
                }
                theme.key_words
            }
            Some(ColorResult::Final(c)) => *c,
            None => theme.default,
        }
    }

    pub fn map_styles(&mut self, file_type: &FileType, theme: &Theme, tc: &SemanticTokensServerCapabilities) {
        if let SemanticTokensServerCapabilities::SemanticTokensOptions(tokens) = tc {
            match file_type {
                FileType::Rust => {
                    for token_type in tokens.legend.token_types.iter() {
                        let token_type = token_type.as_str();
                        if self.generic_mapping(token_type, theme) {
                            continue;
                        }
                        match token_type {
                            "decorator" => self.legend.push(ColorResult::Final(theme.functions)),
                            "bitwise" => self.legend.push(ColorResult::Final(theme.key_words)),
                            "arithmetic" => self.legend.push(ColorResult::default()),
                            "boolean" => self.legend.push(ColorResult::Final(theme.key_words)),
                            "builtinAttribute" => self.legend.push(ColorResult::Final(theme.constant)),
                            "builtinType" => self.legend.push(ColorResult::Final(theme.class_or_struct)),
                            "character" => self.legend.push(ColorResult::Final(theme.string)),
                            "colon" => self.legend.push(ColorResult::default()),
                            "comma" => self.legend.push(ColorResult::default()),
                            "comparison" => self.legend.push(ColorResult::default()),
                            "constParameter" => self.legend.push(ColorResult::Final(theme.key_words)),
                            "derive" => self.legend.push(ColorResult::Final(theme.functions)),
                            "dot" => self.legend.push(ColorResult::default()),
                            "escapeSequence" => self.legend.push(ColorResult::Final(theme.string_escape)),
                            "invalidEscapeSequence" => self.legend.push(ColorResult::Final(color::red())),
                            "lifetime" => self.legend.push(ColorResult::Final(theme.key_words)),
                            "macroBang" => self.legend.push(ColorResult::Final(theme.key_words)),
                            "selfKeyword" => self.legend.push(ColorResult::Final(theme.key_words)),
                            "selfTypeKeyword" => self.legend.push(ColorResult::Final(theme.key_words)),
                            "semicolon" => self.legend.push(ColorResult::default()),
                            "typeAlias" => self.legend.push(ColorResult::Final(theme.class_or_struct)),
                            // "attributeBracket" => {}
                            // "bracket" => {}
                            // "brace" => {}
                            // "deriveHelper" => {}
                            // "formatSpecifier" => {}
                            // "generic" => {}
                            // "label" => {}
                            // "logical" => {}
                            // "parenthesis" => {}
                            // "punctuation" => {}
                            // "angle" => {}
                            // "toolModule" => {}
                            // "union" => {}
                            // "unresolvedReference" => {},
                            _ => self.legend.push(ColorResult::Final(theme.default)),
                        }
                    }
                }
                _ => {
                    for token_type in tokens.legend.token_types.iter() {
                        let token_type = token_type.as_str();
                        self.generic_mapping(token_type, theme);
                    }
                }
            }
        }
    }

    fn generic_mapping(&mut self, token_type: &str, theme: &Theme) -> bool {
        match token_type {
            "namespace" => self.legend.push(ColorResult::Final(theme.class_or_struct)),
            "type" => self.legend.push(ColorResult::Final(theme.class_or_struct)),
            "class" => self.legend.push(ColorResult::Final(theme.class_or_struct)),
            "enum" => self.legend.push(ColorResult::Final(theme.class_or_struct)),
            "interface" => self.legend.push(ColorResult::Final(theme.class_or_struct)),
            "struct" => self.legend.push(ColorResult::Final(theme.class_or_struct)),
            "typeParameter" => self.legend.push(ColorResult::Final(theme.class_or_struct)),
            "parameter" => self.legend.push(ColorResult::Final(theme.default)),
            "variable" => self.legend.push(ColorResult::Final(theme.default)),
            "property" => self.legend.push(ColorResult::Final(theme.default)),
            "enumMember" => self.legend.push(ColorResult::Final(theme.constant)),
            "event" => self.legend.push(ColorResult::Final(theme.flow_control)),
            "function" => self.legend.push(ColorResult::Final(theme.functions)),
            "method" => self.legend.push(ColorResult::Final(theme.functions)),
            "macro" => self.legend.push(ColorResult::Final(theme.key_words)),
            "keyword" => self.legend.push(ColorResult::KeyWord),
            "modifier" => self.legend.push(ColorResult::Final(theme.key_words)),
            "comment" => self.legend.push(ColorResult::Final(theme.comment)),
            "string" => self.legend.push(ColorResult::Final(theme.string)),
            "number" => self.legend.push(ColorResult::Final(theme.numeric)),
            "regexp" => self.legend.push(ColorResult::Final(color::red())),
            "operator" => self.legend.push(ColorResult::default()),
            "decorator" => self.legend.push(ColorResult::Final(theme.functions)),
            _ => return false,
        }
        true
    }
}
