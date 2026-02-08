/// Wrapper around vt100::Parser for managing the virtual terminal screen.
pub struct ScreenState {
    parser: vt100::Parser,
}

impl ScreenState {
    pub fn new(rows: u16, cols: u16) -> Self {
        Self {
            parser: vt100::Parser::new(rows, cols, 0),
        }
    }

    /// Feed raw bytes from the PTY.
    pub fn process(&mut self, bytes: &[u8]) {
        self.parser.process(bytes);
    }

    /// Access the current screen state.
    pub fn screen(&self) -> &vt100::Screen {
        self.parser.screen()
    }

    /// Resize the virtual screen.
    pub fn resize(&mut self, rows: u16, cols: u16) {
        self.parser.set_size(rows, cols);
    }
}
