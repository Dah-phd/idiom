pub struct RenameTestTmp {
    asd: String,
    bsd: String,
}

impl RenameTestTmp {
    fn new() -> Self {
        let asd = String::from("test");
        let bsd = String::from("second_test");
        Self { asd, bsd }
    }

    fn print_asd(&self) {
        println!("{}", self.asd);
    }

    fn print_bsd(&self) {
        println!("{}", self.bsd);
    }
}
