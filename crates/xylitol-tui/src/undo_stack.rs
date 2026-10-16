/// UndoStack - generic undo stack for snapshot-based undo.
pub struct UndoStack<T> {
    stack: Vec<T>,
}

impl<T> Default for UndoStack<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> UndoStack<T> {
    pub fn new() -> Self {
        Self { stack: Vec::new() }
    }

    pub fn push(&mut self, state: T) {
        self.stack.push(state);
    }

    pub fn pop(&mut self) -> Option<T> {
        self.stack.pop()
    }

    pub fn clear(&mut self) {
        self.stack.clear();
    }
}
