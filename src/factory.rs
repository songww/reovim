use std::cell::RefCell;
use std::fmt::Debug;

use gtk::glib::Sender;
use relm4::factory::{Factory, FactoryPrototype, FactoryView};
use rustc_hash::FxHashMap;
use vector_map::VecMap;

struct Widgets<Widgets: Debug, Root: Debug> {
    widgets: Widgets,
    root: Root,
}

impl<WidgetsType: Debug, Root: Debug> Debug for Widgets<WidgetsType, Root> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Widgets")
            .field("widgets", &self.widgets)
            .field("root", &self.root)
            .finish()
    }
}

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
#[allow(clippy::type_complexity)]
#[derive(Default, Debug)]
pub struct FactoryMap<Data>
where
    Data: FactoryPrototype,
{
    data: FxHashMap<u64, Data>,
    widgets: RefCell<
        FxHashMap<u64, Widgets<Data::Widgets, <Data::View as FactoryView<Data::Root>>::Root>>,
    >,
    staged: RefCell<VecMap<u64, ChangeType>>,
    flushes: RefCell<VecMap<u64, ChangeType>>,
}

impl<Data> FactoryMap<Data>
where
    Data: FactoryPrototype,
{
    /// Create a new [`FactoryMap].
    #[must_use]
    pub fn new() -> Self {
        FactoryMap {
            data: FxHashMap::default(),
            widgets: RefCell::new(FxHashMap::default()),
            staged: RefCell::new(VecMap::new()),
            flushes: RefCell::new(VecMap::new()),
        }
    }

    /// Initialize a new [`FactoryMap`] with a normal [`Vec`].
    #[must_use]
    pub fn from_hashmap(data: FxHashMap<u64, Data>) -> Self {
        let length = data.len();

        let mut staged = VecMap::default();
        staged.reserve(length);
        let mut flushes = VecMap::default();
        flushes.reserve(length);
        data.keys().for_each(|k| {
            staged.insert(*k, ChangeType::Add);
        });
        let mut widgets = FxHashMap::default();
        widgets.reserve(length);
        FactoryMap {
            data,
            widgets: RefCell::new(widgets),
            staged: RefCell::new(staged),
            flushes: RefCell::new(flushes),
        }
    }

    /// Get a slice of the internal data of a [`FactoryMap`].
    // #[must_use]
    // pub fn as_slice(&self) -> &[Data] {
    //     self.data.as_slice()
    // }

    /// Get the internal data of the [`FactoryMap`].
    #[must_use]
    pub fn into_hashmap(self) -> FxHashMap<u64, Data> {
        self.data
    }

    /// Remove all data from the [`FactoryMap`].
    pub fn clear(&mut self) {
        let stage = &mut self.staged.borrow_mut();

        for key in self.data.keys() {
            stage.insert(*key, ChangeType::Remove);
        }
        self.data.clear();
    }

    /// Returns the length as amount of elements stored in this type.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns [`true`] if the length of this type is `0`.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Insert an element at the end of a [`FactoryMap`].
    pub fn insert(&mut self, key: u64, data: Data) {
        self.data.insert(key, data);

        let change = match self.staged.borrow().get(&key) {
            Some(ChangeType::Recreate | ChangeType::Remove) => ChangeType::Recreate,
            _ => ChangeType::Add,
        };
        self.staged.borrow_mut().insert(key, change);
    }

    /// Remove an element of a [`FactoryMap].
    pub fn remove(&mut self, key: u64) -> Option<Data> {
        let data = self.data.remove(&key);
        if data.is_some() {
            self.staged.borrow_mut().insert(key, ChangeType::Remove);
        }

        data
    }

    /// Get a reference to data stored by `key`.
    #[must_use]
    pub fn get(&self, key: u64) -> Option<&Data> {
        self.data.get(&key)
    }

    /// Get a mutable reference to data stored at `key`.
    ///
    /// Assumes that the data will be modified and the corresponding widget
    /// needs to be updated.
    #[must_use]
    pub fn get_mut(&mut self, key: u64) -> Option<&mut Data> {
        let mut staged = self.staged.borrow_mut();
        if !staged.contains_key(&key) {
            staged.insert(key, ChangeType::Update);
        }

        self.data.get_mut(&key)
    }

    pub fn flush(&mut self) {
        let mut staged = self.staged.borrow_mut();
        let mut flushes = self.flushes.borrow_mut();
        for (k, v) in staged.iter() {
            flushes.insert(*k, *v);
        }
        staged.clear();
    }
}

impl<Data, View> Factory<Data, View> for FactoryMap<Data>
where
    Data: FactoryPrototype<Factory = Self, View = View>,
    View: FactoryView<Data::Root>,
{
    type Key = u64;

    fn generate(&self, view: &View, sender: Sender<Data::Msg>) {
        for (index, change) in self.flushes.borrow().iter() {
            let mut widgets = self.widgets.borrow_mut();

            match change {
                ChangeType::Add => {
                    let data = self.data.get(index).unwrap();
                    let new_widgets = data.init_view(index, sender.clone());
                    let position = data.position(index);
                    let root = view.add(Data::root_widget(&new_widgets), &position);
                    widgets.insert(
                        *index,
                        Widgets {
                            widgets: new_widgets,
                            root,
                        },
                    );
                }
                ChangeType::Update => {
                    self.data
                        .get(index)
                        .unwrap()
                        .view(index, &widgets.get(index).unwrap().widgets);
                }
                ChangeType::Remove => {
                    let remove_widget = widgets.remove(index).unwrap();
                    view.remove(&remove_widget.root);
                }
                ChangeType::Recreate => {
                    let remove_widget = widgets.remove(index).unwrap();
                    view.remove(&remove_widget.root);
                    let data = self.data.get(index).unwrap();
                    let new_widgets = data.init_view(index, sender.clone());
                    let position = data.position(index);
                    let root = view.add(Data::root_widget(&new_widgets), &position);
                    widgets.insert(
                        *index,
                        Widgets {
                            widgets: new_widgets,
                            root,
                        },
                    );
                }
            }
        }
        self.flushes.borrow_mut().clear();
    }
}

impl<Data, View> FactoryMap<Data>
where
    Data: FactoryPrototype<Factory = Self, View = View>,
    View: FactoryView<Data::Root>,
{
    /// Get an immutable iterator for this type
    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, u64, Data> {
        self.data.iter()
    }
    /// Get an immutable iterator for this type
    pub fn iter_mut(&mut self) -> std::collections::hash_map::IterMut<'_, u64, Data> {
        self.data.iter_mut()
    }
}
