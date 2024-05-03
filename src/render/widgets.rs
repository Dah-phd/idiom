use crate::render::{
    backend::{Backend, Style},
    layout::Rect,
};

#[allow(dead_code)]
pub fn paragraph<'a>(area: Rect, text: impl Iterator<Item = &'a str>, backend: &mut Backend) -> std::io::Result<()> {
    let mut lines = area.into_iter();
    for text_line in text {
        match lines.next() {
            Some(mut line) => {
                if text_line.len() > line.width {
                    let mut at_char = 0;
                    let mut remaining = text_line.len();
                    while remaining != 0 {
                        let width = line.width;
                        if let Some(text_slice) = text_line.get(at_char..at_char + width) {
                            line.render(text_slice, backend)?;
                        } else {
                            line.render(text_line[at_char..].as_ref(), backend)?;
                            break;
                        }
                        if let Some(next_line) = lines.next() {
                            line = next_line;
                            at_char += line.width;
                            remaining = remaining.saturating_sub(width);
                        } else {
                            return Ok(());
                        }
                    }
                } else {
                    line.render(&text_line, backend)?;
                };
            }
            None => return Ok(()),
        }
    }
    for remaining_line in lines {
        remaining_line.render_empty(backend)?;
    }
    Ok(())
}

pub fn paragraph_styled<'a>(
    area: Rect,
    text: impl Iterator<Item = (&'a str, Style)>,
    backend: &mut Backend,
) -> std::io::Result<()> {
    let mut lines = area.into_iter();
    for (text_line, style) in text {
        match lines.next() {
            Some(mut line) => {
                if text_line.len() > line.width {
                    let mut at_char = 0;
                    let mut remaining = text_line.len();
                    while remaining != 0 {
                        let width = line.width;
                        if let Some(text_slice) = text_line.get(at_char..at_char + width) {
                            line.render_styled(text_slice, style, backend)?;
                        } else {
                            line.render_styled(text_line[at_char..].as_ref(), style, backend)?;
                            break;
                        }
                        if let Some(next_line) = lines.next() {
                            line = next_line;
                            at_char += line.width;
                            remaining = remaining.saturating_sub(width);
                        } else {
                            return Ok(());
                        }
                    }
                } else {
                    line.render_styled(&text_line, style, backend)?;
                };
            }
            None => return Ok(()),
        }
    }
    for remaining_line in lines {
        remaining_line.render_empty(backend)?;
    }
    Ok(())
}
