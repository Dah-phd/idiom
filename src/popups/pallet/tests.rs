use crate::configs::CONFIG_FOLDER;
use dirs::config_dir;

#[test]
fn bumb() {
    let mut path = config_dir().unwrap();
    path.push(CONFIG_FOLDER);
    path.push("data");
    panic!("{:?}", path);
}
