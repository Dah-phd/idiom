use super::formatting::{lowercase, uppercase};
use crate::{
    embeded_term::EditorTerminal,
    ext_tui::CrossTerm,
    global_state::GlobalState,
    tree::tests::mock_tree,
    workspace::{tests::mock_ws, CursorPosition, Workspace},
};
use idiom_tui::{layout::Rect, Backend};

fn to_str_list<'a>(ws: &'a mut Workspace) -> Vec<&'a str> {
    ws.get_active().unwrap().content.iter().map(|el| el.as_str()).collect()
}

#[test]
fn uppercase_test() {
    let mut term = EditorTerminal::new(Some("mock".to_owned()));
    let mut tree = mock_tree();
    let mut gs = GlobalState::new(Rect::new(0, 0, 80, 10), CrossTerm::init());
    let mut ws = mock_ws(vec!["".to_string()]);
    uppercase(&mut gs, &mut ws, &mut tree, &mut term);
    assert_eq!(to_str_list(&mut ws), [""]);

    let mut ws = mock_ws(vec![" test ".to_string()]);
    ws.get_active().unwrap().cursor.char = 2;
    uppercase(&mut gs, &mut ws, &mut tree, &mut term);
    assert_eq!(to_str_list(&mut ws), [" TEST "]);

    let mut ws = mock_ws(vec![" test_part ".to_string()]);
    ws.get_active()
        .unwrap()
        .cursor
        .select_set(CursorPosition { line: 0, char: 4 }, CursorPosition { line: 0, char: 7 });
    uppercase(&mut gs, &mut ws, &mut tree, &mut term);
    assert_eq!(to_str_list(&mut ws), [" tesT_Part "]);

    let mut ws = mock_ws(vec![
        " test on mAny lines".to_string(),
        " mOre lInes ".to_string(),
        " morE lines ".to_string(),
    ]);
    ws.get_active()
        .unwrap()
        .cursor
        .select_set(CursorPosition { line: 0, char: 8 }, CursorPosition { line: 2, char: 5 });
    uppercase(&mut gs, &mut ws, &mut tree, &mut term);
    assert_eq!(to_str_list(&mut ws), [" test on MANY LINES", " MORE LINES ", " MORE lines "]);
}

#[test]
fn lowercase_test() {
    let mut term = EditorTerminal::new(Some("mock".to_owned()));
    let mut tree = mock_tree();
    let mut gs = GlobalState::new(Rect::new(0, 0, 80, 10), CrossTerm::init());
    let mut ws = mock_ws(vec!["".to_string()]);
    lowercase(&mut gs, &mut ws, &mut tree, &mut term);
    assert_eq!(to_str_list(&mut ws), [""]);

    let mut ws = mock_ws(vec![" TEST ".to_string()]);
    ws.get_active().unwrap().cursor.char = 2;
    lowercase(&mut gs, &mut ws, &mut tree, &mut term);
    assert_eq!(to_str_list(&mut ws), [" test "]);

    let mut ws = mock_ws(vec![" TEST_PART ".to_string()]);
    ws.get_active()
        .unwrap()
        .cursor
        .select_set(CursorPosition { line: 0, char: 4 }, CursorPosition { line: 0, char: 7 });
    lowercase(&mut gs, &mut ws, &mut tree, &mut term);
    assert_eq!(to_str_list(&mut ws), [" TESt_pART "]);

    let mut ws = mock_ws(vec![
        " TEST ON mAny lINES".to_string(),
        " mORE LINEs ".to_string(),
        " MORe lines ".to_string(),
    ]);
    ws.get_active()
        .unwrap()
        .cursor
        .select_set(CursorPosition { line: 0, char: 8 }, CursorPosition { line: 2, char: 5 });
    lowercase(&mut gs, &mut ws, &mut tree, &mut term);
    assert_eq!(to_str_list(&mut ws), [" TEST ON many lines", " more lines ", " more lines "]);
}
