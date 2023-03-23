use glib::subclass::prelude::*;

use crate::color::Colors;
use crate::style;

mod imp {
    use std::cell::{Cell, RefCell};

    use glib::subclass::prelude::*;
    use rustc_hash::FxHashMap;

    use crate::color::Colors;

    #[derive(Debug)]
    pub struct HighlightDefinitions {
        styles: RefCell<FxHashMap<u64, crate::style::Style>>,
        defaults: Cell<Option<Colors>>,
    }

    impl Default for HighlightDefinitions {
        fn default() -> Self {
            let mut styles = FxHashMap::default();
            let defaults = Colors {
                background: crate::color::Color::BLACK.into(),
                foreground: crate::color::Color::WHITE.into(),
                special: crate::color::Color::WHITE.into(),
            };
            styles.insert(0, crate::style::Style::new(defaults));
            HighlightDefinitions {
                styles: RefCell::new(styles),
                defaults: Some(defaults).into(),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for HighlightDefinitions {
        const NAME: &'static str = "HighlightDefinitions";
        type Type = super::HighlightDefinitions;
        type ParentType = glib::Object;
    }

    impl ObjectImpl for HighlightDefinitions {}

    impl HighlightDefinitions {
        pub fn get(&self, k: u64) -> Option<&crate::style::Style> {
            // SAFETY: already locked by user.
            let styles = unsafe { &*self.styles.as_ptr() };
            styles.get(&k)
            // .unwrap_or_else(|| styles.get(&0).expect("DefaultHighlights not set yet."))
        }
        pub fn set(&self, k: u64, style: crate::style::Style) {
            self.styles.borrow_mut().insert(k, style);
        }

        pub fn defaults(&self) -> Option<&Colors> {
            unsafe { &*self.defaults.as_ptr() }.as_ref()
        }

        pub fn set_defaults(&self, defaults: Colors) {
            self.defaults.replace(Some(defaults));
            let styles = unsafe { &mut *self.styles.as_ptr() };
            styles.insert(0, crate::style::Style::new(defaults));
        }
    }
}

glib::wrapper! {
    pub struct HighlightDefinitions(ObjectSubclass<imp::HighlightDefinitions>);
}

impl HighlightDefinitions {
    pub const DEFAULT: u64 = 0;

    pub fn new() -> HighlightDefinitions {
        glib::Object::new()
    }

    fn imp(&self) -> &imp::HighlightDefinitions {
        imp::HighlightDefinitions::from_obj(self)
    }

    pub fn get(&self, k: u64) -> Option<&style::Style> {
        self.imp().get(k)
    }

    pub fn set(&self, k: u64, style: style::Style) {
        self.imp().set(k, style);
    }

    pub fn defaults(&self) -> Option<&Colors> {
        self.imp().defaults()
    }

    pub fn set_defaults(&self, defaults: Colors) {
        self.imp().set_defaults(defaults)
    }
}
