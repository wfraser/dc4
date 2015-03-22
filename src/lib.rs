pub struct DC4 {
    x: i32
}

impl DC4 {
    pub fn new() -> DC4 {
        DC4 {
            x: 42,
        }
    }

    pub fn program(&mut self, s: &str) -> i32 {
        //TODO
        self.x += s.len() as i32;
        self.x
    }
}
