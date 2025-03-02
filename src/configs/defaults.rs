// MODS
pub const SHIFT: &str = "shift";
pub const CTRL: &str = "ctrl";
pub const ALT: &str = "alt";
pub const SUPER: &str = "super";
pub const HYPER: &str = "hyper";
pub const META: &str = "meta";

// KEYS
pub const BACKSPACE: &str = "backspace";
pub const ENTER: &str = "enter";
pub const LEFT: &str = "left";
pub const RIGHT: &str = "right";
pub const UP: &str = "up";
pub const DOWN: &str = "down";
pub const HOME: &str = "home";
pub const END: &str = "end";
pub const PAGEUP: &str = "pageup";
pub const PAGEDOWN: &str = "pagedown";
pub const TAB: &str = "tab";
pub const BACKTAB: &str = "backtab";
pub const DELETE: &str = "delete";
pub const INSERT: &str = "insert";
pub const F: &str = "f";
pub const ESC: &str = "esc";

pub fn new_line() -> String {
    ENTER.to_owned()
}

pub fn tab() -> String {
    TAB.to_owned()
}

pub fn backspace() -> String {
    BACKSPACE.to_owned()
}

pub fn delete() -> String {
    DELETE.to_owned()
}

pub fn remove_line() -> String {
    format!("{CTRL} && {DELETE} || {CTRL} && h || {CTRL} && {BACKSPACE}")
}

pub fn indent_start() -> String {
    format!("{CTRL} && ]")
}

pub fn unindent() -> String {
    format!("{SHIFT} && {TAB}")
}

pub fn up() -> String {
    UP.to_owned()
}

pub fn down() -> String {
    DOWN.to_owned()
}

pub fn left() -> String {
    LEFT.to_owned()
}

pub fn right() -> String {
    RIGHT.to_owned()
}

pub fn char_c() -> String {
    String::from('c')
}

pub fn char_x() -> String {
    String::from('x')
}

pub fn select_up() -> String {
    format!("{SHIFT} && {UP}")
}

pub fn select_down() -> String {
    format!("{SHIFT} && {DOWN}")
}

pub fn select_left() -> String {
    format!("{SHIFT} && {LEFT}")
}

pub fn select_right() -> String {
    format!("{SHIFT} && {RIGHT}")
}

pub fn select_token() -> String {
    format!("{CTRL} && w")
}

pub fn select_line() -> String {
    format!("{CTRL} && l")
}

pub fn select_all() -> String {
    format!("{CTRL} && a")
}

pub fn scroll_up() -> String {
    format!("{CTRL} && {UP} || {PAGEUP}")
}

pub fn scroll_down() -> String {
    format!("{CTRL} && {DOWN} || {PAGEDOWN}")
}

pub fn swap_up() -> String {
    format!("{ALT} && {UP}")
}

pub fn swap_down() -> String {
    format!("{ALT} && {DOWN}")
}

pub fn jump_left() -> String {
    format!("{CTRL} && {LEFT} || {ALT} && {LEFT}")
}

pub fn jump_left_select() -> String {
    format!("{CTRL} && {SHIFT} && {LEFT} || {ALT} && {SHIFT} && {LEFT}")
}

pub fn jump_right() -> String {
    format!("{CTRL} && {RIGHT} || {ALT} && {RIGHT}")
}

pub fn jump_right_select() -> String {
    format!("{CTRL} && {SHIFT} && {RIGHT} || {ALT} && {SHIFT} && {RIGHT}")
}

pub fn end() -> String {
    END.to_owned()
}

pub fn end_of_file() -> String {
    format!("{CTRL} && {END}")
}

pub fn home() -> String {
    HOME.to_owned()
}

pub fn start_of_file() -> String {
    format!("{CTRL} && {HOME}")
}

pub fn find_references() -> String {
    format!("{F}9")
}

pub fn go_to_declaration() -> String {
    format!("{F}12")
}

pub fn help() -> String {
    format!("{F}1")
}

pub fn refresh() -> String {
    format!("{F}5")
}

pub fn rename() -> String {
    format!("{F}2")
}

pub fn cut() -> String {
    format!("{CTRL} && x")
}

pub fn copy() -> String {
    format!("{CTRL} && c || {CTRL} && {SHIFT} && c")
}

pub fn paste() -> String {
    format!("{CTRL} && v")
}

pub fn undo() -> String {
    format!("{CTRL} && z")
}

pub fn redo() -> String {
    format!("{CTRL} && y")
}

pub fn save() -> String {
    format!("{CTRL} && s")
}

pub fn esc() -> String {
    ESC.to_owned()
}

pub fn close() -> String {
    format!("{CTRL} && q || {CTRL} && d")
}

pub fn comment_out() -> String {
    format!("{CTRL} && /")
}

pub fn select_open_editor() -> String {
    format!("{CTRL} && {UP} || {CTRL} && {DOWN}")
}

pub fn find() -> String {
    format!("{CTRL} && f")
}

pub fn replace() -> String {
    format!("{CTRL} && h")
}

pub fn terminal() -> String {
    format!("{CTRL} && `")
}

pub fn go_to() -> String {
    format!("{CTRL} && g")
}

pub fn hide_file_tree() -> String {
    format!("{CTRL} && e")
}

pub fn tab1() -> String {
    format!("{ALT} && 1")
}
pub fn tab2() -> String {
    format!("{ALT} && 2")
}
pub fn tab3() -> String {
    format!("{ALT} && 3")
}
pub fn tab4() -> String {
    format!("{ALT} && 4")
}
pub fn tab5() -> String {
    format!("{ALT} && 5")
}
pub fn tab6() -> String {
    format!("{ALT} && 6")
}
pub fn tab7() -> String {
    format!("{ALT} && 7")
}
pub fn tab8() -> String {
    format!("{ALT} && 8")
}
pub fn tab9() -> String {
    format!("{ALT} && 9")
}

pub fn expand() -> String {
    format!("{RIGHT} || d || {ENTER}")
}

pub fn shrink() -> String {
    format!("{LEFT} || a")
}

pub fn tree_up() -> String {
    format!("{UP} || w")
}

pub fn tree_down() -> String {
    format!("{DOWN} || d")
}

pub fn tree_delete() -> String {
    format!("{SHIFT} && {DELETE}")
}

pub fn new_file() -> String {
    format!("{CTRL} && n")
}

pub fn tree_size_inc() -> String {
    format!("{CTRL} && {RIGHT}")
}

pub fn tree_size_dec() -> String {
    format!("{CTRL} && {LEFT}")
}

pub const fn get_indent_spaces() -> usize {
    4
}

pub fn get_indent_after() -> String {
    String::from("({[")
}

pub fn get_unident_before() -> String {
    String::from("]})")
}

pub fn pallet() -> String {
    format!("{CTRL} && p")
}
