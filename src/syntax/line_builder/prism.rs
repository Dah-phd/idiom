#[derive(Default)]
pub struct Prism {
    transforms: Vec<Transform>,
}

impl Prism {
    pub fn add() {}
    pub fn remap() {}
    pub fn reset(&mut self) {
        self.transforms.clear();
    }
}

enum Transform {
    Offset(),
    Suppress(),
}
