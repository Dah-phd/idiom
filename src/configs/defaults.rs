// MODS
pub const SHIFT: &str = "shift";
pub const CTRL: &str = "ctrl";
pub const ALT: &str = "alt";
pub const SUPER: &str = "super";
pub const HYPER: &str = "hyper";
pub const META: &str = "meta";

// KEYS
pub const MENU: &str = "menu";
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

pub const fn get_indent_spaces() -> usize {
    4
}

pub fn get_indent_after() -> String {
    String::from("({[")
}

pub fn get_unident_before() -> String {
    String::from("]})")
}
