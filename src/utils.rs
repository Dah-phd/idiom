pub fn trim_start_inplace(line: &mut String) {
    if let Some(idx) = line.find(|c: char| !c.is_whitespace()) {
        line.replace_range(..idx, "");
    };
}
