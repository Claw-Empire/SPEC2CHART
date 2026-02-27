use std::collections::VecDeque;

use crate::model::FlowchartDocument;

#[derive(Debug, Clone)]
pub struct UndoStack {
    states: VecDeque<FlowchartDocument>,
    current: usize,
    max_size: usize,
}

impl UndoStack {
    pub fn new(max_size: usize) -> Self {
        Self {
            states: VecDeque::new(),
            current: 0,
            max_size,
        }
    }

    pub fn push(&mut self, doc: &FlowchartDocument) {
        self.states.truncate(self.current);
        self.states.push_back(doc.clone());
        if self.states.len() > self.max_size {
            self.states.pop_front();
        }
        self.current = self.states.len();
    }

    pub fn undo(&mut self) -> Option<&FlowchartDocument> {
        if self.current > 1 {
            self.current -= 1;
            Some(&self.states[self.current - 1])
        } else {
            None
        }
    }

    pub fn redo(&mut self) -> Option<&FlowchartDocument> {
        if self.current < self.states.len() {
            self.current += 1;
            Some(&self.states[self.current - 1])
        } else {
            None
        }
    }

    pub fn can_undo(&self) -> bool {
        self.current > 1
    }

    pub fn can_redo(&self) -> bool {
        self.current < self.states.len()
    }
}
