use crate::render::{
    backend::{Backend, BackendProtocol},
    widgets::Writable,
};

use super::Text;

#[test]
fn test_basic_text() {
    let inner = String::from("asd🚀aa31ase字as");
    let as_text = Text::from(inner);
    assert_eq!(as_text.char_len(), 14);
    assert_eq!(as_text.width(), 16);
    assert_eq!(as_text.len(), 19);
    let mut backend = Backend::init();
    as_text.print(&mut backend);
    let data = backend.drain().into_iter().next().unwrap().1;
    assert_eq!(&data, "asd🚀aa31ase字as");
}
