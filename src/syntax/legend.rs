use crate::{configs::FileType, syntax::Theme};
use crossterm::style::Color;
use lsp_types::SemanticTokensServerCapabilities;

#[derive(Clone, Copy, Debug)]
pub enum ColorResult {
    Direct(Color),
    Modifiable { normal: Color, moded: Color },
}

impl Default for ColorResult {
    fn default() -> Self {
        Self::Direct(Color::Reset)
    }
}

#[derive(Debug)]
pub struct Legend {
    legend: Vec<ColorResult>,
    default: Color,
}

impl Default for Legend {
    fn default() -> Self {
        Self { legend: vec![], default: Color::Reset }
    }
}

impl Legend {
    pub fn parse_to_color(&self, token_type: usize, modifier: u32) -> Color {
        match self.legend.get(token_type) {
            Some(ColorResult::Modifiable { normal, moded }) => {
                if modifier != 0 {
                    return *moded;
                }
                *normal
            }
            Some(ColorResult::Direct(c)) => *c,
            None => self.default,
        }
    }

    pub fn map_styles(&mut self, file_type: FileType, theme: &Theme, tc: &SemanticTokensServerCapabilities) {
        self.default = theme.default;
        let legend = match tc {
            SemanticTokensServerCapabilities::SemanticTokensOptions(opt) => &opt.legend,
            SemanticTokensServerCapabilities::SemanticTokensRegistrationOptions(opt) => {
                &opt.semantic_tokens_options.legend
            }
        };

        match file_type {
            FileType::Rust => {
                for token_type in legend.token_types.iter() {
                    let token_type = token_type.as_str();
                    if self.generic_mapping(token_type, theme) {
                        continue;
                    }
                    match token_type {
                        "bitwise" => self.legend.push(ColorResult::Direct(theme.key_words)),
                        "arithmetic" => self.legend.push(ColorResult::default()),
                        "boolean" => self.legend.push(ColorResult::Direct(theme.key_words)),
                        "builtinAttribute" => self.legend.push(ColorResult::Direct(theme.constant)),
                        "builtinType" => self.legend.push(ColorResult::Direct(theme.class_or_struct)),
                        "character" => self.legend.push(ColorResult::Direct(theme.string)),
                        "colon" => self.legend.push(ColorResult::default()),
                        "comma" => self.legend.push(ColorResult::default()),
                        "comparison" => self.legend.push(ColorResult::default()),
                        "constParameter" => self.legend.push(ColorResult::Direct(theme.key_words)),
                        "derive" => self.legend.push(ColorResult::Direct(theme.functions)),
                        "dot" => self.legend.push(ColorResult::default()),
                        "escapeSequence" => self.legend.push(ColorResult::Direct(theme.string_escape)),
                        "invalidEscapeSequence" => self.legend.push(ColorResult::Direct(Color::Red)),
                        "lifetime" => self.legend.push(ColorResult::Direct(theme.key_words)),
                        "macroBang" => self.legend.push(ColorResult::Direct(theme.key_words)),
                        "selfKeyword" => self.legend.push(ColorResult::Direct(theme.key_words)),
                        "selfTypeKeyword" => self.legend.push(ColorResult::Direct(theme.key_words)),
                        "semicolon" => self.legend.push(ColorResult::default()),
                        "typeAlias" => self.legend.push(ColorResult::Direct(theme.class_or_struct)),
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
                        _ => self.legend.push(ColorResult::Direct(theme.default)),
                    }
                }
            }
            FileType::TypeScript => {
                for token_type in legend.token_types.iter() {
                    let token_type = token_type.as_str();
                    if self.generic_mapping(token_type, theme) {
                        continue;
                    }
                    match token_type {
                        "member" => self.legend.push(ColorResult::Direct(theme.functions)),
                        _ => self.legend.push(ColorResult::Direct(theme.default)),
                    }
                }
            }
            _ => {
                for token_type in legend.token_types.iter() {
                    let token_type = token_type.as_str();
                    if !self.generic_mapping(token_type, theme) {
                        self.legend.push(ColorResult::Direct(theme.default));
                    };
                }
            }
        }
    }

    fn generic_mapping(&mut self, token_type: &str, theme: &Theme) -> bool {
        match token_type {
            "namespace" => self.legend.push(ColorResult::Direct(theme.class_or_struct)),
            "type" => self.legend.push(ColorResult::Direct(theme.class_or_struct)),
            "class" => self.legend.push(ColorResult::Direct(theme.class_or_struct)),
            "enum" => self.legend.push(ColorResult::Direct(theme.class_or_struct)),
            "interface" => self.legend.push(ColorResult::Direct(theme.class_or_struct)),
            "struct" => self.legend.push(ColorResult::Direct(theme.class_or_struct)),
            "typeParameter" => self.legend.push(ColorResult::Direct(theme.class_or_struct)),
            "parameter" => self.legend.push(ColorResult::Direct(theme.default)),
            "variable" => self.legend.push(ColorResult::Direct(theme.default)),
            "property" => self.legend.push(ColorResult::Direct(theme.default)),
            "enumMember" => self.legend.push(ColorResult::Direct(theme.constant)),
            "event" => self.legend.push(ColorResult::Direct(theme.flow_control)),
            "function" => self.legend.push(ColorResult::Direct(theme.functions)),
            "method" => self.legend.push(ColorResult::Direct(theme.functions)),
            "macro" => self.legend.push(ColorResult::Direct(theme.key_words)),
            "keyword" => {
                self.legend.push(ColorResult::Modifiable { normal: theme.key_words, moded: theme.flow_control })
            }
            "modifier" => self.legend.push(ColorResult::Direct(theme.key_words)),
            "comment" => self.legend.push(ColorResult::Direct(theme.comment)),
            "string" => self.legend.push(ColorResult::Direct(theme.string)),
            "number" => self.legend.push(ColorResult::Direct(theme.numeric)),
            "regexp" => self.legend.push(ColorResult::Direct(Color::Red)),
            "operator" => self.legend.push(ColorResult::default()),
            "decorator" => self.legend.push(ColorResult::Direct(theme.functions)),
            _ => return false,
        }
        true
    }
}
