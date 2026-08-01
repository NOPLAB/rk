//! Undo stack (full-document snapshots) and append-only command journal.
//!
//! Snapshots also give `Engine::apply` its atomicity: a failed command
//! rolls back to the snapshot taken before execution. The journal records
//! every applied command for auditing and future branching/checkpoints.

use crate::command::Command;
use crate::document::Document;

const MAX_UNDO_STEPS: usize = 50;

/// A full copy of the document plus a description of the change after it
#[derive(Debug, Clone)]
pub(crate) struct Snapshot {
    pub doc: Document,
    pub description: String,
}

/// Snapshot-based undo/redo stacks
#[derive(Debug, Default)]
pub(crate) struct UndoStack {
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
}

impl UndoStack {
    pub fn push(&mut self, snapshot: Snapshot) {
        self.redo.clear();
        self.undo.push(snapshot);
        if self.undo.len() > MAX_UNDO_STEPS {
            self.undo.remove(0);
        }
    }

    pub fn pop_undo(&mut self) -> Option<Snapshot> {
        self.undo.pop()
    }

    pub fn push_redo(&mut self, snapshot: Snapshot) {
        self.redo.push(snapshot);
    }

    pub fn pop_redo(&mut self) -> Option<Snapshot> {
        self.redo.pop()
    }

    pub fn push_undo_only(&mut self, snapshot: Snapshot) {
        self.undo.push(snapshot);
        if self.undo.len() > MAX_UNDO_STEPS {
            self.undo.remove(0);
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo_description(&self) -> Option<&str> {
        self.undo.last().map(|s| s.description.as_str())
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }
}

/// One applied command
#[derive(Debug, Clone)]
pub struct JournalEntry {
    pub seq: u64,
    pub command: Command,
}

/// Append-only log of every successfully applied command
#[derive(Debug, Default)]
pub(crate) struct CommandJournal {
    entries: Vec<JournalEntry>,
    next_seq: u64,
}

impl CommandJournal {
    pub fn record(&mut self, command: Command) -> u64 {
        let seq = self.next_seq;
        self.next_seq += 1;
        self.entries.push(JournalEntry { seq, command });
        seq
    }

    pub fn entries(&self) -> &[JournalEntry] {
        &self.entries
    }
}
