pub mod backend;
mod button;
pub mod layout;
mod list_state;
pub mod state;
mod text_field;
pub mod widgets;
pub use button::Button;
pub use list_state::WrappedState;
pub use text_field::TextField;

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
