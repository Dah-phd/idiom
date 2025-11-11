use super::super::Block;

pub fn parse_hr<'a>(line: &'a str) -> Option<Block<'a>> {
    if line.len() < 3 {
        return None;
    }
    if !line.chars().all(|c| c == '-') && !line.chars().all(|c| c == '=') {
        return None;
    }
    Some(Block::Hr)
}

#[cfg(test)]
mod test {
    use super::super::Block::Hr;
    use super::parse_hr;

    #[test]
    fn finds_hr() {
        assert_eq!(parse_hr("-------"), Some(Hr));
        assert_eq!(parse_hr("---"), Some(Hr));
        assert_eq!(parse_hr("----------------------------"), Some(Hr));
        assert_eq!(parse_hr("-------"), Some(Hr));

        assert_eq!(parse_hr("======="), Some(Hr));
        assert_eq!(parse_hr("==="), Some(Hr));
        assert_eq!(parse_hr("============================"), Some(Hr));
    }

    #[test]
    fn no_false_positives() {
        assert_eq!(parse_hr("a-------"), None);
        assert_eq!(parse_hr("--- a"), None);
        assert_eq!(parse_hr("--a-"), None);
        assert_eq!(parse_hr("-------====--------------"), None);

        assert_eq!(parse_hr("a======"), None);
        assert_eq!(parse_hr("=== a"), None);
        assert_eq!(parse_hr("==a="), None);
        assert_eq!(parse_hr("=======---================="), None);
    }
}
