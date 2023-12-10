#[derive(Default)]
pub struct Prism {
    transforms: Vec<Transform>,
}

impl Prism {
    pub fn add(&mut self) {}
    pub fn remap(&mut self) {}
    pub fn reset(&mut self) {
        self.transforms.clear();
    }
}

enum Transform {
    Offset(),
    Suppress(),
}
