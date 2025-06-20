use super::super::Block;
use super::super::Block::CodeBlock;
use lazy_static::lazy_static;
use regex::Regex;

pub fn parse_code_block(lines: &[&str]) -> Option<(Block, usize)> {
    lazy_static! {
        static ref CODE_BLOCK_SPACES: Regex = Regex::new(r"^ {4}").expect("Pattern already tested!");
        static ref CODE_BLOCK_TABS: Regex = Regex::new(r"^\t").expect("Pattern already tested!");
        static ref CODE_BLOCK_BACKTICKS: Regex = Regex::new(r"```").expect("Pattern already tested!");
    }

    let mut content = String::new();
    let mut lang: Option<String> = None;
    let mut line_number = 0;
    let mut backtick_opened = false;
    let mut backtick_closed = false;

    for line in lines {
        if !backtick_opened && CODE_BLOCK_SPACES.is_match(line) {
            if line_number > 0 && !content.is_empty() {
                content.push('\n');
            }
            // remove top-level spaces
            content.push_str(&line[4..line.len()]);
            line_number += 1;
        } else if !backtick_opened && CODE_BLOCK_TABS.is_match(line) {
            if line_number > 0 && !content.is_empty() {
                content.push('\n');
            }

            if !(line_number == 0 && line.trim().is_empty()) {
                // remove top-level spaces
                content.push_str(&line[1..line.len()]);
            }
            line_number += 1;
        } else if CODE_BLOCK_BACKTICKS.is_match(line) {
            line_number += 1;

            if backtick_opened {
                backtick_closed = true;
                break;
            }

            if let Some(lang_name) = line.get(3..) {
                backtick_opened = true;
                lang = Some(String::from(lang_name));
            }
        } else if backtick_opened {
            content.push_str(line);
            content.push('\n');

            line_number += 1;
        } else {
            break;
        }
    }

    if line_number > 0 && (backtick_closed || !backtick_opened) {
        return Some((CodeBlock(lang, content.trim_matches('\n').to_owned()), line_number));
    }

    None
}

#[cfg(test)]
mod test {
    use super::super::Block::CodeBlock;
    use super::parse_code_block;

    #[test]
    fn finds_code_block() {
        assert_eq!(parse_code_block(&["    Test"]), Some((CodeBlock(None, "Test".to_owned()), 1)));

        assert_eq!(parse_code_block(&["    Test", "    this"]), Some((CodeBlock(None, "Test\nthis".to_owned()), 2)));

        assert_eq!(
            parse_code_block(&["```testlang", "Test", "this", "```"]),
            Some((CodeBlock(Some(String::from("testlang")), "Test\nthis".to_owned()), 4))
        );
    }

    #[test]
    fn knows_when_to_stop() {
        assert_eq!(
            parse_code_block(&["    Test", "    this", "stuff", "    now"]),
            Some((CodeBlock(None, "Test\nthis".to_owned()), 2))
        );
    }

    #[test]
    fn no_false_positives() {
        assert_eq!(parse_code_block(&["   Test"]), None);
    }

    #[test]
    fn no_early_matching() {
        assert_eq!(parse_code_block(&["Test", "    this", "stuff", "    now"]), None);
    }
}
