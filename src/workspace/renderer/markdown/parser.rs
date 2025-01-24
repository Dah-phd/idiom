use crossterm::style::Attribute;

#[derive(Debug, PartialEq)]
pub struct MarkDownToken {
    pub start: usize,
    pub end: usize,
    pub style: Attribute,
    pub offset: usize,
}

impl MarkDownToken {
    fn ital(start: usize, end: usize) -> Self {
        Self { start, end, style: Attribute::Italic, offset: 1 }
    }

    fn bold(start: usize, end: usize) -> Self {
        Self { start, end, style: Attribute::Bold, offset: 2 }
    }

    pub fn roll<A>(&self, iter: &mut impl Iterator<Item = A>) {
        let mut adv = self.offset;
        while adv != 0 {
            let _ = iter.next();
            adv -= 1;
        }
    }
}

#[derive(Default)]
pub struct MarkDownCollector {
    bold: Option<usize>,
    ital: Option<usize>,
    tokens: Vec<MarkDownToken>,
}

impl MarkDownCollector {
    pub fn collect(mut self) -> Vec<MarkDownToken> {
        self.tokens.sort_by(|x, y| x.start.cmp(&y.start));
        self.tokens
    }

    pub fn parse(mut self, content: &str) -> Self {
        let mut content = content.chars().enumerate().skip_while(|(_, ch)| ch.is_whitespace());
        let mut last_whites = false;
        while let Some((idx, ch)) = content.next() {
            last_whites = match ch {
                '*' if last_whites => self.collect_format_from_wspace(&mut content, idx, '*'),
                '_' if last_whites => self.collect_format_from_wspace(&mut content, idx, '_'),
                '*' => self.collect_format(&mut content, idx, '*'),
                '_' => self.collect_format(&mut content, idx, '_'),
                ' ' => true,
                _ => false,
            };
        }
        self
    }

    fn start_parse(mut self, content: &str) -> Self {
        let mut content = content.chars().enumerate().skip_while(|(_, ch)| ch.is_whitespace());
        if let Some((idx, ch)) = content.next() {
            let last_whites = match ch {
                '#' => false,
                '*' => self.collect_format(&mut content, idx, '*'),
                '_' => self.collect_format(&mut content, idx, '_'),
                ' ' => true,
                _ => false,
            };
            return self.secondary_parse(&mut content, last_whites);
        }
        self
    }

    fn secondary_parse(mut self, mut content: &mut impl Iterator<Item = (usize, char)>, mut last_whites: bool) -> Self {
        while let Some((idx, ch)) = content.next() {
            last_whites = match ch {
                '*' if last_whites => self.collect_format_from_wspace(&mut content, idx, '*'),
                '_' if last_whites => self.collect_format_from_wspace(&mut content, idx, '_'),
                '*' => self.collect_format(&mut content, idx, '*'),
                '_' => self.collect_format(&mut content, idx, '_'),
                ' ' => true,
                _ => false,
            };
        }
        self
    }

    fn collect_format_from_wspace(
        &mut self,
        content: &mut impl Iterator<Item = (usize, char)>,
        idx: usize,
        mark_ch: char,
    ) -> bool {
        match content.next() {
            Some((.., ch)) if ch == mark_ch => match content.next() {
                Some((.., next_ch)) if self.bold.is_none() => {
                    self.bold.replace(idx);
                    return next_ch == ' ';
                }
                _ => {}
            },
            Some((.., ' ')) => return true,
            Some(..) if self.ital.is_none() => {
                self.ital.replace(idx);
            }
            _ => {}
        }
        false
    }

    fn collect_format(&mut self, content: &mut impl Iterator<Item = (usize, char)>, idx: usize, mark_ch: char) -> bool {
        match content.next() {
            Some((b_idx, ch)) if ch == mark_ch => match content.next() {
                Some((end_idx, ' ')) => {
                    if let Some(start) = self.bold.take() {
                        self.tokens.push(MarkDownToken::bold(start, end_idx))
                    }
                    return true;
                }
                Some((end_idx, ..)) => match self.bold.take() {
                    Some(start) => self.tokens.push(MarkDownToken::bold(start, end_idx)),
                    None => {
                        self.bold.replace(idx);
                    }
                },
                None => {
                    if let Some(start) = self.bold.take() {
                        self.tokens.push(MarkDownToken::bold(start, b_idx + 1))
                    }
                }
            },
            Some((end_idx, ' ')) => {
                if let Some(start) = self.ital.take() {
                    self.tokens.push(MarkDownToken::ital(start, end_idx))
                }
                return true;
            }
            Some((end_idx, ..)) => match self.ital.take() {
                Some(start) => self.tokens.push(MarkDownToken::ital(start, end_idx)),
                None => {
                    self.ital.replace(idx);
                }
            },
            None => {
                if let Some(start) = self.ital.take() {
                    self.tokens.push(MarkDownToken::ital(start, idx + 1))
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod test {
    use super::{MarkDownCollector, MarkDownToken};

    #[test]
    fn test_bold_uscore() {
        let content = "Markdown with __bold__";
        let span = 14..22;
        assert_eq!(MarkDownCollector::default().parse(content).collect(), [MarkDownToken::bold(span.start, span.end)]);
        assert_eq!(&content[span], "__bold__");

        let content = "Markdown with __bold__ with more text";
        let span = 14..22;
        assert_eq!(MarkDownCollector::default().parse(content).collect(), [MarkDownToken::bold(span.start, span.end)]);
        assert_eq!(&content[span], "__bold__");
    }

    #[test]
    fn test_bold_star() {
        let content = "Markdown with **bold**";
        let span = 14..22;
        assert_eq!(MarkDownCollector::default().parse(content).collect(), [MarkDownToken::bold(span.start, span.end)]);
        assert_eq!(&content[span], "**bold**");

        let content = "Markdown with **bold** with more text";
        let span = 14..22;
        assert_eq!(MarkDownCollector::default().parse(content).collect(), [MarkDownToken::bold(span.start, span.end)]);
        assert_eq!(&content[span], "**bold**");
    }

    #[test]
    fn test_ital_uscore() {
        let content = "Markdown with _ital_";
        let span = 14..20;
        assert_eq!(MarkDownCollector::default().parse(content).collect(), [MarkDownToken::ital(span.start, span.end)]);
        assert_eq!(&content[span], "_ital_");

        let content = "Markdown with _ital_ with more text";
        let span = 14..20;
        assert_eq!(MarkDownCollector::default().parse(content).collect(), [MarkDownToken::ital(span.start, span.end)]);
        assert_eq!(&content[span], "_ital_");
    }

    #[test]
    fn test_ital_star() {
        let content = "Markdown with *ital*";
        let span = 14..20;
        assert_eq!(MarkDownCollector::default().parse(content).collect(), [MarkDownToken::ital(span.start, span.end)]);
        assert_eq!(&content[span], "*ital*");

        let content = "Markdown with *ital* with more text";
        let span = 14..20;
        assert_eq!(MarkDownCollector::default().parse(content).collect(), [MarkDownToken::ital(span.start, span.end)]);
        assert_eq!(&content[span], "*ital*");
    }
}
