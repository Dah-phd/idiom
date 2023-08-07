use tui::text::Span;

use super::Lexer;

pub fn rust_processor(lexer: &mut Lexer, content: &str, spans: &mut Vec<Span>) {
    if lexer.lang.mod_import.iter().any(|key| content.starts_with(key)) {
        for (idx, ch) in content.chars().enumerate() {
            match ch {
                ' ' => {
                    lexer.drain_buf_object(idx, spans);
                    lexer.last_token.push(ch);
                }
                '.' | '<' | '>' | ':' | '?' | '&' | '=' | '+' | '-' | ',' | ';' => {
                    lexer.drain_buf_object(idx, spans);
                    lexer.white_char(idx, ch, spans);
                }
                _ => lexer.last_token.push(ch),
            }
        }
        return;
    }
    let mut char_steam = content.chars().enumerate().peekable();
    while let Some((token_end, ch)) = char_steam.next() {
        match ch {
            ' ' => {
                lexer.drain_buf(token_end, spans);
                lexer.last_token.push(ch);
            }
            '.' | '<' | '>' | '?' | '&' | '=' | '+' | '-' | ',' | ';' => {
                lexer.drain_buf(token_end, spans);
                lexer.white_char(token_end, ch, spans);
            }
            ':' => {
                if matches!(char_steam.peek(), Some((_, next_ch)) if next_ch == &':') {
                    lexer.drain_buf_colored(token_end, lexer.theme.class_or_struct, spans);
                    lexer.white_char(token_end, ch, spans);
                } else {
                    lexer.drain_buf(token_end, spans);
                    lexer.white_char(token_end, ch, spans);
                }
            }
            '!' => {
                lexer.last_token.push(ch);
                lexer.drain_buf_colored(token_end, lexer.theme.key_words, spans);
            }
            '(' => {
                lexer.drain_buf_colored(token_end, lexer.theme.functions, spans);
                let color = Lexer::len_to_color(lexer.brackets.len());
                lexer.last_token.push(ch);
                lexer.brackets.push(color);
                lexer.drain_buf_colored(token_end, color, spans);
            }
            ')' => {
                let color = lexer.brackets.pop().unwrap_or(Lexer::default_color());
                lexer.drain_buf(token_end, spans);
                lexer.last_token.push(ch);
                lexer.drain_buf_colored(token_end, color, spans);
            }
            '{' => {
                lexer.drain_buf(token_end, spans);
                let color = Lexer::len_to_color(lexer.curly.len());
                lexer.last_token.push(ch);
                lexer.curly.push(color);
                lexer.drain_buf_colored(token_end, color, spans);
            }
            '}' => {
                let color = lexer.curly.pop().unwrap_or(Lexer::default_color());
                lexer.drain_buf(token_end, spans);
                lexer.last_token.push(ch);
                lexer.drain_buf_colored(token_end, color, spans);
            }
            '[' => {
                lexer.drain_buf(token_end, spans);
                let color = Lexer::len_to_color(lexer.square.len());
                lexer.last_token.push(ch);
                lexer.square.push(color);
                lexer.drain_buf_colored(token_end, color, spans);
            }
            ']' => {
                let color = lexer.square.pop().unwrap_or(Lexer::default_color());
                lexer.drain_buf(token_end, spans);
                lexer.last_token.push(ch);
                lexer.drain_buf_colored(token_end, color, spans);
            }
            _ => lexer.last_token.push(ch),
        }
    }
}
