use glib::subclass::prelude::*;
use rustc_hash::FxHashMap;

use crate::style;

mod imp {
    use std::cell::{Cell, RefCell};

    use rustc_hash::FxHashMap;

    use glib::prelude::*;
    use glib::subclass::prelude::*;

    #[derive(Clone, Debug, Default)]
    pub struct HighlightDefinitions {
        styles: RefCell<FxHashMap<u64, crate::style::Style>>,
        defaults: Cell<Option<crate::style::Colors>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for HighlightDefinitions {
        const NAME: &'static str = "HighlightDefinitions";
        type Type = super::HighlightDefinitions;
        type ParentType = glib::Object;
    }

    impl ObjectImpl for HighlightDefinitions {}

    impl HighlightDefinitions {
        pub fn get(&self, k: u64) -> &crate::style::Style {
            // SAFETY: already locked by user.
            let styles = unsafe { &*self.styles.as_ptr() };
            styles
                .get(&k)
                .unwrap_or_else(|| styles.get(&0).expect("DefaultHighlights not set yet."))
        }
        pub fn set(&self, k: u64, style: crate::style::Style) {
            let styles = unsafe { &mut *self.styles.as_ptr() };
            styles.insert(k, style);
        }

        pub fn defaults(&self) -> Option<&crate::style::Colors> {
            unsafe { &*self.defaults.as_ptr() }.as_ref()
        }

        pub fn set_defaults(&self, defaults: crate::style::Colors) {
            self.defaults.replace(Some(defaults));
            let styles = unsafe { &mut *self.styles.as_ptr() };
            styles
                .entry(0)
                .or_insert_with(|| crate::style::Style::new(defaults));
        }
    }
}

// impl std::fmt::Debug for HighlightDefinitions {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         f.write_str("HighlightDefinitions: ")?;
//         f.debug_map()
//             .entries(self.styles.iter().map(|(k, v)| (k, v)))
//             .finish()
//     }
// }

glib::wrapper! {
    pub struct HighlightDefinitions(ObjectSubclass<imp::HighlightDefinitions>);
}

impl HighlightDefinitions {
    pub fn new() -> HighlightDefinitions {
        // let styles = FxHashMap::default();
        // HighlightDefinitions::default()
        glib::Object::new::<Self>(&[]).expect("Failed to initialize Timer object")
    }

    fn imp(&self) -> &imp::HighlightDefinitions {
        imp::HighlightDefinitions::from_instance(self)
    }

    pub fn get(&self, k: u64) -> &style::Style {
        self.imp().get(k)
    }

    pub fn set(&self, k: u64, style: style::Style) {
        self.imp().set(k, style);
    }

    pub fn defaults(&self) -> Option<&style::Colors> {
        self.imp().defaults()
    }

    pub fn set_defaults(&self, defaults: style::Colors) {
        self.imp().set_defaults(defaults)
    }
}
