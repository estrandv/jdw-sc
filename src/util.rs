pub struct Counter {
    pub value: i32
}

impl Counter {
    pub fn next(&mut self) -> i32{
        self.value += 1;
        self.value
    }
}