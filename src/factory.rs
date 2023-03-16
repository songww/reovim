use std::cell::RefCell;
use std::fmt::Debug;

use relm4::factory::sync::{ComponentStorage, FactoryBuilder};
use relm4::factory::{FactoryComponent, FactoryView};
use relm4::prelude::*;
use relm4::Sender;
use rustc_hash::FxHashMap;
use vector_map::VecMap;

#[derive(Clone, Copy, Debug)]
enum ChangeType {
    Add,
    Remove,
    Recreate,
    Update,
}

impl Default for ChangeType {
    fn default() -> Self {
        ChangeType::Add
    }
}

/// A container similar to [`HashMap`] that implements [`Factory`].
#[derive(Debug)]
pub struct Factory<C: FactoryComponent> {
    widget: C::ParentWidget,
    parent_sender: Sender<C::ParentInput>,
    components: FxHashMap<u64, ComponentStorage<C>>,
    staged: VecMap<u64, ChangeType>,
    flushes: VecMap<u64, ChangeType>,
}

impl<C: FactoryComponent> Factory<C> {
    /// Create a new [`Factory].
    pub fn new(widget: C::ParentWidget, parent_sender: &Sender<C::ParentInput>) -> Self {
        Factory {
            widget,
            parent_sender: parent_sender.clone(),
            components: FxHashMap::default(),
            staged: VecMap::new(),
            flushes: VecMap::new(),
        }
    }

    /// Initialize a new [`FactoryMap`] with a normal [`Vec`].
    #[must_use]
    pub fn from_hashmap(
        base: FxHashMap<u64, C::Init>,
        widget: C::ParentWidget,
        parent_sender: &Sender<C::ParentInput>,
    ) -> Self {
        let length = base.len();

        let mut staged = VecMap::default();
        let mut flushes = VecMap::default();
        let mut components = FxHashMap::default();
        staged.reserve(length);
        flushes.reserve(length);
        components.reserve(length);
        let mut factory = Factory {
            widget,
            staged,
            flushes,
            components,
            parent_sender: parent_sender.clone(),
        };
        base.into_iter().map(|(k, init)| {
            factory.insert(k, init);
        });
        factory
    }

    /// Get a slice of the internal data of a [`FactoryMap`].
    // #[must_use]
    // pub fn as_slice(&self) -> &[Data] {
    //     self.data.as_slice()
    // }

    /// Get the internal data of the [`FactoryMap`].
    #[must_use]
    pub fn into_hashmap(self) -> FxHashMap<u64, C> {
        self.components
            .into_iter()
            .map(|(k, v)| (k, ComponentStorage::extract(v)))
            .collect()
    }

    /// Remove all data from the [`FactoryMap`].
    pub fn clear(&mut self) {
        let stage = &mut self.staged;

        for key in self.components.keys() {
            stage.insert(*key, ChangeType::Remove);
        }
        self.components.clear();
    }

    /// Returns the length as amount of elements stored in this type.
    pub fn len(&self) -> usize {
        self.components.len()
    }

    /// Returns [`true`] if the length of this type is `0`.
    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }

    /// Insert an element at the end of a [`FactoryMap`].
    pub fn insert(&mut self, key: u64, init: C::Init) {
        let builder = FactoryBuilder::new(&DynamicIndex::new(0), init);

        self.components
            .insert(key, ComponentStorage::Builder(builder));

        let changed = match self.staged.get(&key) {
            Some(ChangeType::Recreate | ChangeType::Remove) => ChangeType::Recreate,
            _ => ChangeType::Add,
        };
        self.staged.insert(key, changed);
    }

    /// Remove an element of a [`FactoryMap].
    pub fn remove(&mut self, key: u64) -> Option<C> {
        let component = self.components.remove(&key).map(ComponentStorage::extract);
        if component.is_some() {
            self.staged.insert(key, ChangeType::Remove);
        }

        component
    }

    /// Get a reference to data stored by `key`.
    #[must_use]
    pub fn get(&self, key: u64) -> Option<&C> {
        self.components.get(&key).map(ComponentStorage::get)
    }

    /// Get a mutable reference to data stored at `key`.
    ///
    /// Assumes that the data will be modified and the corresponding widget
    /// needs to be updated.
    #[must_use]
    pub fn get_mut(&mut self, key: u64) -> Option<&mut C> {
        let mut staged = &mut self.staged;
        if !staged.contains_key(&key) {
            staged.insert(key, ChangeType::Update);
        }

        self.components.get_mut(&key).map(ComponentStorage::get_mut)
    }

    /// Returns the widget all components are attached to.
    pub const fn widget(&self) -> &C::ParentWidget {
        &self.widget
    }

    pub fn flush(&mut self) {
        let mut staged = &mut self.staged;
        let mut flushes = &mut self.flushes;
        for (k, v) in staged.iter() {
            flushes.insert(*k, *v);
        }
        staged.clear();
    }
}

impl<C> Factory<C>
where
    C: FactoryComponent,
{
    fn render_changes(&mut self) where <<C as relm4::factory::FactoryComponent>::ParentWidget as FactoryView>::ReturnedWidget: AsRef<<<C as relm4::factory::FactoryComponent>::ParentWidget as FactoryView>::Children> {
        for (index, change) in self.flushes.drain() {
            let mut widget = &mut self.widget;

            match change {
                ChangeType::Add => {
                    let component = self.components.get(&index).unwrap();
                    let widget = component.returned_widget().unwrap();
                    let position = component.get().position(0);
                    let root = self.widget.factory_append(widget, &position);
                }
                ChangeType::Update => {
                    let component = self.components.get(&index).unwrap();
                    let position = component.get().position(0);
                    self.widget
                        .factory_update_position(component.returned_widget().unwrap(), &position);
                }
                ChangeType::Remove => {
                    let component = self.components.get(&index).unwrap();
                    self.widget
                        .factory_remove(component.returned_widget().unwrap());
                }
                ChangeType::Recreate => {
                    let component = self.components.get(&index).unwrap();
                    let position = component.get().position(0);
                    self.widget
                        .factory_remove(component.returned_widget().unwrap());
                    self.widget
                        .factory_append(&component.returned_widget().unwrap(), &position);
                }
            }
        }
        self.flushes.clear();
    }
}

impl<C: FactoryComponent> Factory<C> {
    /// Get an immutable iterator for this type
    pub fn iter(&self) -> impl Iterator<Item = (u64, &C)> {
        self.components
            .iter()
            .map(|(k, v)| (*k, ComponentStorage::get(v)))
    }
    /// Get an immutable iterator for this type
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (u64, &mut C)> {
        self.components
            .iter_mut()
            .map(|(k, v)| (*k, ComponentStorage::get_mut(v)))
    }
}
