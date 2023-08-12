use std::sync::Arc;

use crate::kmp_file::*;
use bevy::prelude::*;
use std::fmt::Debug;
use undo_2::{Action, Commands as UndoCommands};

pub struct UndoPlugin;

impl Plugin for UndoPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UndoStack>();
    }
}

// trait that will be implemented on structs for all the ways we can change the kmp data structure
// - create, delete, modify
trait UndoItem: Debug {
    fn undo(&self, kmp: &mut Kmp);
    fn redo(&self, kmp: &mut Kmp);
}

#[derive(Debug)]
struct Create<T> {
    index: usize,
    value: T,
}
impl<T: KmpData + Clone + Debug> UndoItem for Create<T> {
    fn undo(&self, kmp: &mut Kmp) {
        T::get_section(kmp).unwrap().entries.remove(self.index);
    }
    fn redo(&self, kmp: &mut Kmp) {
        T::get_section(kmp)
            .unwrap()
            .entries
            .insert(self.index, self.value.clone())
    }
}

#[derive(Debug)]
struct Remove<T> {
    index: usize,
    value: T,
}
impl<T: KmpData + Clone + Debug> UndoItem for Remove<T> {
    fn undo(&self, kmp: &mut Kmp) {
        T::get_section(kmp)
            .unwrap()
            .entries
            .insert(self.index, self.value.clone());
    }
    fn redo(&self, kmp: &mut Kmp) {
        T::get_section(kmp).unwrap().entries.remove(self.index);
    }
}

#[derive(Debug)]
struct Modify<T> {
    index: usize,
    before: T,
    after: T,
}
impl<T: KmpData + Clone + Debug> UndoItem for Modify<T> {
    fn undo(&self, kmp: &mut Kmp) {
        T::get_section(kmp).unwrap().entries[self.index] = self.before.clone();
    }
    fn redo(&self, kmp: &mut Kmp) {
        T::get_section(kmp).unwrap().entries[self.index] = self.after.clone();
    }
}

// this undo stack contains the various structs above, Arc and Mutex so it can be shared across threads
#[derive(Resource, Deref, DerefMut)]
struct UndoStack(UndoCommands<Arc<dyn UndoItem + Send + Sync>>);
impl UndoStack {
    pub fn push(&mut self, command: impl UndoItem + Send + Sync + 'static) {
        let command = Arc::new(command);
        self.0.push(command);
    }
    pub fn undo(&mut self, kmp: &mut Kmp) {
        self.interpret_action(kmp);
    }
    pub fn redo(&mut self, kmp: &mut Kmp) {
        self.interpret_action(kmp);
    }
    fn interpret_action(&mut self, kmp: &mut Kmp) {
        for action in self.0.undo() {
            match action {
                Action::Do(x) => x.redo(kmp),
                Action::Undo(x) => x.undo(kmp),
            }
        }
    }
}
impl Default for UndoStack {
    fn default() -> Self {
        Self(UndoCommands::new())
    }
}
