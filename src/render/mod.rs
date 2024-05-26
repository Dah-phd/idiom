pub mod backend;
mod button;
pub mod layout;
pub mod state;
pub mod text;
mod text_field;
pub mod utils;
pub mod widgets;
pub use button::Button;
pub use text_field::TextField;
pub use utils::UTF8Safe;

/// This can easily gorow to be a framework itself

pub fn count_as_string(len: usize) -> String {
    if len < 10 {
        format!("  {len}")
    } else if len < 100 {
        format!(" {len}")
    } else {
        String::from("99+")
    }
}
