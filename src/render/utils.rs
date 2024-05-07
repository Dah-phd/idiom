/// truncates str to size, if str is smaller does nothing
pub fn truncate_str(text: &str, len: usize) -> &str {
    if text.len() > len {
        return unsafe { text.get_unchecked(..len) };
    }
    text
}

pub fn truncate_str_start(text: &str, len: usize) -> &str {
    if text.len() > len {
        return unsafe { text.get_unchecked(text.len() - len..) };
    }
    text
}

#[cfg(test)]
mod test {
    use super::{truncate_str, truncate_str_start};

    // double checking unsafe code
    #[test]
    fn test_truncate() {
        let s = "123";
        assert_eq!(truncate_str(s, 12), "123");
        assert_eq!(truncate_str(s, 2), "12");
        assert_eq!(truncate_str(s, 0), "");
    }

    // double checking unsafe code
    #[test]
    fn test_truncate_start() {
        let s = "123";
        assert_eq!(truncate_str_start(s, 12), "123");
        assert_eq!(truncate_str_start(s, 2), "23");
        assert_eq!(truncate_str_start(s, 0), "");
    }
}
