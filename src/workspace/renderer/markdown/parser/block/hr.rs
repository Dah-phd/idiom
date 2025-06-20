use super::super::Block;
use super::super::Block::Hr;
use lazy_static::lazy_static;
use regex::Regex;

pub fn parse_hr(lines: &[&str]) -> Option<(Block, usize)> {
    lazy_static! {
        static ref HORIZONTAL_RULE: Regex = Regex::new(r"^(===+)$|^(---+)$").expect("Pattern already testsed!");
    }

    if HORIZONTAL_RULE.is_match(lines[0]) {
        return Some((Hr, 1));
    }
    None
}

#[cfg(test)]
mod test {
    use super::super::Block::Hr;
    use super::parse_hr;

    #[test]
    fn finds_hr() {
        assert_eq!(parse_hr(&["-------"]), Some((Hr, 1)));
        assert_eq!(parse_hr(&["---"]), Some((Hr, 1)));
        assert_eq!(parse_hr(&["----------------------------"]), Some((Hr, 1)));
        assert_eq!(parse_hr(&["-------", "abc"]), Some((Hr, 1)));

        assert_eq!(parse_hr(&["======="]), Some((Hr, 1)));
        assert_eq!(parse_hr(&["==="]), Some((Hr, 1)));
        assert_eq!(parse_hr(&["============================"]), Some((Hr, 1)));
        assert_eq!(parse_hr(&["=======", "abc"]), Some((Hr, 1)));
    }

    #[test]
    fn no_false_positives() {
        assert_eq!(parse_hr(&["a-------"]), None);
        assert_eq!(parse_hr(&["--- a"]), None);
        assert_eq!(parse_hr(&["--a-"]), None);
        assert_eq!(parse_hr(&["-------====--------------"]), None);

        assert_eq!(parse_hr(&["a======"]), None);
        assert_eq!(parse_hr(&["=== a"]), None);
        assert_eq!(parse_hr(&["==a="]), None);
        assert_eq!(parse_hr(&["=======---================="]), None);
    }
}
