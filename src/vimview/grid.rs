mod imp {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    use std::sync::RwLock;

    use gtk::{
        gdk::prelude::*,
        graphene::{Point, Rect, Size},
        pango,
        prelude::*,
        subclass::prelude::*,
    };

    use super::super::highlights::HighlightDefinitions;
    use super::super::textbuf::TextBuf;

    // #[derive(Debug)]
    pub struct VimGridView {
        grid: Cell<u64>,
        size: Cell<Size>,
        textbuf: RefCell<TextBuf>,
        hldefs: Cell<Rc<RwLock<HighlightDefinitions>>>,
    }

    impl std::fmt::Debug for VimGridView {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("VimGridView")
                .field("grid", &self.grid)
                .field("size", &self.size)
                .finish_non_exhaustive()
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for VimGridView {
        const NAME: &'static str = "VimGridView";
        type ParentType = gtk::Widget;
        type Type = super::VimGridView;

        // fn new(grid: u64, size: Size, hldefs: Rc<RwLock<HighlightDefinitions>>) -> Self {
        //     Self {
        //         grid: grid.into(),
        //         size: size.into(),
        //         hldefs: hldefs.into(),
        //         textbuf: TextBuf::new().into(),
        //     }
        // }
        fn new() -> Self {
            Self {
                grid: 0.into(),
                size: Size::new(0., 0.).into(),
                hldefs: Cell::new(Rc::new(RwLock::new(HighlightDefinitions::new()))),
                textbuf: TextBuf::new().into(),
            }
        }
    }

    // Trait shared by all GObjects
    impl ObjectImpl for VimGridView {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            let size = self.size.get();
            obj.set_width_request(size.width() as _);
            obj.set_height_request(size.height() as _);

            obj.bind_property("grid", obj, "grid")
                .flags(glib::BindingFlags::SYNC_CREATE)
                .build();
            obj.bind_property("size", obj, "size")
                .flags(glib::BindingFlags::SYNC_CREATE)
                .build();
            // obj.bind_property("hldefs", obj, "hldefs")
            //     .flags(glib::BindingFlags::SYNC_CREATE)
            //     .build();
            // obj.bind_property("replace-text-cells", obj, "replace-text-cells")
            //     .flags(BindingFlags::SYNC_CREATE)
            //     .build();
        }

        fn set_property(
            &self,
            obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "grid" => {
                    self.grid.replace(value.get::<u64>().unwrap());
                }
                "size" => {
                    self.size.replace(value.get::<Size>().unwrap());
                }
                // "hldefs" => {
                //     self.hldefs
                //         .replace(value.get::<Rc<RwLock<HighlightDefinitions>>>().unwrap());
                //     obj.queue_draw();
                // }
                // "replace-text-cells" => {
                //     self.hldefs.update(value.get::<TextCells>().unwrap());
                // }
                "flush" => {
                    obj.queue_draw();
                }
                _ => unimplemented!(),
            }
        }

        // fn property(&self, _obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        //     match pspec.name() {
        //         "rmr" => self.inner.borrow().rmr.to_value(),
        //         "x-lines-interval" => self.inner.borrow().x_lines_interval.to_value(),
        //         _ => unimplemented!(),
        //     }
        // }
    }

    // Trait shared by all widgets
    impl WidgetImpl for VimGridView {
        fn snapshot(&self, widget: &Self::Type, snapshot: &gtk::Snapshot) {
            let size = self.size.get();
            let rect = Rect::new(0., 0., size.width(), size.height());
            let cr = snapshot.append_cairo(&rect);
            let pctx = widget.pango_context();
            let hldefs = (unsafe { &*self.hldefs.as_ptr() }).read().unwrap();
            let layout = self.textbuf.borrow().layout(&pctx, &hldefs);
            pangocairo::show_layout(&cr, &layout);
            self.parent_snapshot(widget, snapshot);
        }
    }

    impl VimGridView {
        fn set_hldefs(&self, hldefs: Rc<RwLock<HighlightDefinitions>>) {
            self.hldefs.replace(hldefs);
        }

        fn set_textbuf(&self, textbuf: TextBuf) {
            self.textbuf.replace(textbuf);
        }
    }
}

use gtk::gdk::prelude::*;
use gtk::graphene::Size;

glib::wrapper! {
    pub struct VimGridView(ObjectSubclass<imp::VimGridView>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl VimGridView {
    pub fn new(grid: u64, width: u64, height: u64) -> VimGridView {
        glib::Object::new(&[
            ("grid", &grid),
            ("size", &Size::new(width as _, height as _)),
        ])
        .expect("Failed to create `VimGridView`.")
    }
}
