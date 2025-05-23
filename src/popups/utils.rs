pub fn next_option<T: Clone>(options: &[T], state: &mut usize) -> Option<T> {
    if options.is_empty() {
        *state = 0;
        return None;
    }
    if options.len() - 1 > *state {
        *state += 1;
    } else {
        *state = 0;
    }
    options.get(*state).cloned()
}

pub fn prev_option<T: Clone>(options: &[T], state: &mut usize) -> Option<T> {
    if options.is_empty() {
        *state = 0;
        return None;
    }
    if *state > 0 {
        *state -= 1;
    } else {
        *state = options.len() - 1;
    }
    options.get(*state).cloned()
}

pub fn count_as_string<T>(options: &[T]) -> String {
    let len = options.len();
    if len < 10 {
        format!("  {len}")
    } else if len < 100 {
        format!(" {len}")
    } else {
        String::from("99+")
    }
}
