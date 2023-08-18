#![allow(dead_code)]

use std::sync::Arc;

use crate::kmp_file::*;
use bevy::prelude::*;
use std::fmt::Debug;
use undo_2::{Action, Commands as UndoCommands};

pub struct UndoPlugin;

impl Plugin for UndoPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UndoStack>()
            .init_resource::<ModifyAction>();
    }
}

// trait that will be implemented on structs for all the ways we can change the kmp data structure
// - create, delete, modify
pub trait UndoItem: Debug {
    fn undo(&self, kmp: &mut Kmp);
    fn redo(&self, kmp: &mut Kmp);
}

#[derive(Debug)]
pub struct Create<T> {
    pub index: usize,
    pub value: T,
}
impl<T> Create<T> {
    pub fn new(index: usize, value: T) -> Self {
        Self { index, value }
    }
}
impl<T: KmpData + Clone + Debug + 'static> UndoItem for Create<T> {
    fn undo(&self, kmp: &mut Kmp) {
        T::get_section(kmp)
            .expect("UndoItem type should be a KmpData section entry")
            .entries
            .remove(self.index);
    }
    fn redo(&self, kmp: &mut Kmp) {
        T::get_section(kmp)
            .expect("UndoItem type should be a KmpData section entry")
            .entries
            .insert(self.index, self.value.clone())
    }
}

#[derive(Debug)]
pub struct Remove<T> {
    pub index: usize,
    pub value: T,
}
impl<T> Remove<T> {
    pub fn new(index: usize, value: T) -> Self {
        Self { index, value }
    }
}
impl<T: KmpData + Clone + Debug + 'static> UndoItem for Remove<T> {
    fn undo(&self, kmp: &mut Kmp) {
        T::get_section(kmp)
            .expect("UndoItem type should be a KmpData section entry")
            .entries
            .insert(self.index, self.value.clone());
    }
    fn redo(&self, kmp: &mut Kmp) {
        T::get_section(kmp).unwrap().entries.remove(self.index);
    }
}

#[derive(Debug, Clone)]
pub struct Modify<T> {
    pub index: usize,
    pub before: T,
    pub after: T,
}
impl<T> Modify<T> {
    pub fn new(index: usize, before: T, after: T) -> Self {
        Self {
            index,
            before,
            after,
        }
    }
}

impl<T: KmpData + Clone + Debug + 'static> UndoItem for Modify<T> {
    fn undo(&self, kmp: &mut Kmp) {
        T::get_section(kmp)
            .expect("UndoItem type should be a KmpData section entry")
            .entries[self.index] = self.before.clone();
    }
    fn redo(&self, kmp: &mut Kmp) {
        T::get_section(kmp)
            .expect("UndoItem type should be a KmpData section entry")
            .entries[self.index] = self.after.clone();
    }
}

#[derive(Resource, Debug, Default, Clone)]
pub struct ModifyAction {
    pub items: Vec<Modify<Itpt>>,
    pub main_point_itpt_index: Option<usize>,
    pub mouse_screen_offset: Vec2,
}
impl ModifyAction {
    pub fn new(
        items: Vec<Modify<Itpt>>,
        main_point_itpt_index: usize,
        mouse_screen_offset: Vec2,
    ) -> Self {
        Self {
            items,
            main_point_itpt_index: Some(main_point_itpt_index),
            mouse_screen_offset,
        }
    }
}
impl UndoItem for ModifyAction {
    fn undo(&self, kmp: &mut Kmp) {
        for item in self.items.iter() {
            item.undo(kmp);
        }
    }
    fn redo(&self, kmp: &mut Kmp) {
        for item in self.items.iter() {
            item.redo(kmp);
        }
    }
}

// this undo stack contains the various structs above, Arc and Mutex so it can be shared across threads
#[derive(Resource, Deref, DerefMut, Debug, Default)]
pub struct UndoStack(pub UndoCommands<Arc<dyn UndoItem + Send + Sync>>);

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
