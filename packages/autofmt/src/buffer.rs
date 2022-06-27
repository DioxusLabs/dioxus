use std::fmt::Result;

struct Buffer {
    buf: String,
    cur_line: usize,
    cur_indent: usize,
}

impl Buffer {
    fn write_tabs(&mut self, num: usize) -> Result {
        todo!()
    }

    fn new_line(&mut self) -> Result {
        todo!()
    }
}
