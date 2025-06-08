use super::super::block::parse_blocks;
use super::super::Block;
use super::super::Block::{Paragraph, UnorderedList};
use super::super::ListItem;
use lazy_static::lazy_static;
use regex::Regex;

pub fn parse_unordered_list(lines: &[&str]) -> Option<(Block, usize)> {
    lazy_static! {
        static ref LIST_BEGIN: Regex =
            Regex::new(r"^(?P<indent> *)(-|\+|\*) (?P<content>.*)").expect("Pattern already testsed!");
        static ref NEW_PARAGRAPH: Regex = Regex::new(r"^ +").expect("Pattern already testsed!");
        static ref INDENTED: Regex = Regex::new(r"^ {0,4}(?P<content>.*)").expect("Pattern already testsed!");
    }

    // if the beginning doesn't match a list don't even bother
    if !LIST_BEGIN.is_match(lines[0]) {
        return None;
    }

    // a vec holding the contents and indentation
    // of each list item
    let mut contents = vec![];
    let mut prev_newline = false;
    let mut is_paragraph = false;

    // counts the number of parsed lines to return
    let mut i = 0;

    let mut line_iter = lines.iter();
    let mut line = line_iter.next();

    // loop for list items
    loop {
        let Some(text) = line else { break };
        let Some(caps) = LIST_BEGIN.captures(text) else { break };

        if prev_newline {
            is_paragraph = true;
            prev_newline = false;
        }

        let mut content = caps.name("content").map(|m| m.as_str().to_owned()).unwrap_or_default();
        let last_indent = caps.name("indent").map(|m| m.as_str().len()).unwrap_or_default();
        i += 1;

        // parse additional lines of the listitem
        loop {
            line = line_iter.next();
            let Some(text) = line else { break };

            if prev_newline && !NEW_PARAGRAPH.is_match(text) {
                break;
            }

            if let Some(caps) = LIST_BEGIN.captures(text) {
                let indent = caps.name("indent").map(|m| m.as_str().len()).unwrap_or_default();
                if indent < 2 || indent <= last_indent {
                    break;
                }
            }

            // newline means we start a new paragraph
            prev_newline = text.is_empty();

            content.push('\n');

            if let Some(cont_match) = INDENTED.captures(text).and_then(|c| c.name("content")) {
                content.push_str(cont_match.as_str());
            }

            i += 1;
        }
        contents.push(parse_blocks(&content));
    }

    let mut list_contents = vec![];

    for c in contents {
        if is_paragraph || c.len() > 1 {
            list_contents.push(ListItem::Paragraph(c));
        } else if let Paragraph(content) = c[0].clone() {
            list_contents.push(ListItem::Simple(content));
        }
    }

    if i > 0 {
        return Some((UnorderedList(list_contents), i));
    }

    None
}

#[cfg(test)]
mod test {
    use super::super::Block::UnorderedList;
    use super::parse_unordered_list;

    #[test]
    fn finds_list() {
        match parse_unordered_list(&["* A list", "* is good"]) {
            Some((UnorderedList(_), 2)) => (),
            x => panic!("Found {:?}", x),
        }

        match parse_unordered_list(&["* A list", "* is good", "laksjdnflakdsjnf"]) {
            Some((UnorderedList(_), 3)) => (),
            x => panic!("Found {:?}", x),
        }
    }

    #[test]
    fn knows_when_to_stop() {
        match parse_unordered_list(&["* A list", "* is good", "", "laksjdnflakdsjnf"]) {
            Some((UnorderedList(_), 3)) => (),
            x => panic!("Found {:?}", x),
        }

        match parse_unordered_list(&["* A list", "", "laksjdnflakdsjnf"]) {
            Some((UnorderedList(_), 2)) => (),
            x => panic!("Found {:?}", x),
        }
    }

    #[test]
    fn no_false_positives() {
        assert_eq!(parse_unordered_list(&["test * test"]), None);
    }

    #[test]
    fn no_early_matching() {
        assert_eq!(parse_unordered_list(&["test", "* whot", "* a list"]), None);
    }
}
