// #[derive(glib::Boxed, Clone, Debug)]
// #[boxed_type(name = "SizeBoxed")]
// pub struct SizeBoxed(Box<Size>);

mod imp {
    use std::cell::{Cell, Ref, RefCell};
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
    // use super::SizeBoxed;

    // #[derive(Debug)]
    pub struct VimGridView {
        grid: Cell<u64>,
        width: Cell<u64>,
        height: Cell<u64>,
        textbuf: Cell<Rc<RefCell<TextBuf>>>,
        hldefs: Cell<Rc<RwLock<HighlightDefinitions>>>,
    }

    impl std::fmt::Debug for VimGridView {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("VimGridView")
                .field("grid", &self.grid)
                // .field("size", unsafe { &*self.size.as_ptr() })
                .finish_non_exhaustive()
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for VimGridView {
        const NAME: &'static str = "VimGridView";
        type ParentType = gtk::Widget;
        type Type = super::VimGridView;

        fn new() -> Self {
            Self {
                grid: 0.into(),
                width: 0.into(),
                height: 0.into(),
                hldefs: Cell::new(Rc::new(RwLock::new(HighlightDefinitions::new()))),
                textbuf: Cell::new(Rc::new(RefCell::new(TextBuf::new()))),
            }
        }
    }

    // Trait shared by all GObjects
    impl ObjectImpl for VimGridView {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            // obj.bind_property("grid", obj, "grid")
            //     .flags(glib::BindingFlags::SYNC_CREATE)
            //     .build();
            // obj.bind_property("size", obj, "size")
            //     .flags(glib::BindingFlags::SYNC_CREATE)
            //     .build();
            // obj.bind_property("hldefs", obj, "hldefs")
            //     .flags(glib::BindingFlags::SYNC_CREATE)
            //     .build();
            // obj.bind_property("replace-text-cells", obj, "replace-text-cells")
            //     .flags(BindingFlags::SYNC_CREATE)
            //     .build();
        }

        fn properties() -> &'static [glib::ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecUInt64::new(
                        "grid",
                        "grid-id",
                        "grid",
                        1,
                        u64::MAX,
                        1,
                        glib::ParamFlags::READWRITE,
                    ),
                    glib::ParamSpecUInt64::new(
                        "width",
                        "cols",
                        "width",
                        1,
                        u64::MAX,
                        1,
                        glib::ParamFlags::READWRITE,
                    ),
                    glib::ParamSpecUInt64::new(
                        "height",
                        "rows",
                        "height",
                        1,
                        u64::MAX,
                        1,
                        glib::ParamFlags::READWRITE,
                    ),
                ]
            });

            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            _obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "grid" => {
                    self.grid.replace(value.get::<u64>().unwrap());
                }
                "width" => {
                    self.width.replace(value.get::<u64>().unwrap());
                    self.textbuf()
                        .resize(self.height.get() as _, self.width.get() as _);
                }
                "height" => {
                    self.height.replace(value.get::<u64>().unwrap());
                    self.textbuf()
                        .resize(self.height.get() as _, self.width.get() as _);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "grid" => self.grid.get().to_value(),
                "width" => self.width.get().to_value(),
                "height" => self.height.get().to_value(),
                _ => unimplemented!(),
            }
        }
    }

    // Trait shared by all widgets
    impl WidgetImpl for VimGridView {
        fn snapshot(&self, widget: &Self::Type, snapshot: &gtk::Snapshot) {
            let pctx = widget.pango_context();
            // NOTE: Apply size required.
            log::info!(
                "height {} width {} cols {} rows {}",
                self.height.get(),
                self.width.get(),
                self.textbuf().cols(),
                self.textbuf().rows()
            );
            assert_eq!(self.height.get(), self.textbuf().rows() as u64);
            assert_eq!(self.width.get(), self.textbuf().cols() as u64);
            let (width, height) = self.size_required(widget);
            log::info!("size required {}x{}", width, height);
            // widget.set_size_request(width, height);

            // let width = widget.width_request();
            // let height = widget.height_request();
            let rect = Rect::new(0., 0., width as _, height as _);
            let cr = snapshot.append_cairo(&rect);
            let hldefs = (unsafe { &*self.hldefs.as_ptr() }).read().unwrap();
            let layout = self.textbuf().layout(&pctx, &hldefs);
            pangocairo::show_layout(&cr, &layout);
            // log::info!("apply layout");
            self.parent_snapshot(widget, snapshot);
            // log::info!("parent snapshot");
        }

        fn measure(
            &self,
            widget: &Self::Type,
            orientation: gtk::Orientation,
            for_size: i32,
        ) -> (i32, i32, i32, i32) {
            let size_required = self.size_required(widget);
            log::error!(
                "measuring VimGridView orientation {} for_size {} size_required {:?}",
                orientation,
                for_size,
                size_required
            );
            match orientation {
                gtk::Orientation::Vertical => (size_required.1, size_required.1, -1, -1),
                gtk::Orientation::Horizontal => (size_required.0, size_required.0, -1, -1),
                _ => self.parent_measure(widget, orientation, for_size),
            }
        }
    }

    impl VimGridView {
        pub(super) fn set_hldefs(&self, hldefs: Rc<RwLock<HighlightDefinitions>>) {
            self.hldefs.replace(hldefs);
        }

        pub(super) fn set_textbuf(&self, textbuf: Rc<RefCell<TextBuf>>) {
            self.textbuf.replace(textbuf);
        }

        pub(super) fn textbuf(&self) -> Ref<TextBuf> {
            unsafe { &*self.textbuf.as_ptr() }.borrow()
        }

        /// width, height
        pub(super) fn size_required(&self, widget: &<Self as ObjectSubclass>::Type) -> (i32, i32) {
            let metrics = widget.pango_context().metrics(None, None).unwrap();
            let lineheight = metrics.height() as f64 / pango::SCALE as f64;
            let charwidth = metrics.approximate_digit_width() as f64 / pango::SCALE as f64;
            (
                (self.width.get() as f64 * charwidth) as i32,
                (self.height.get() as f64 * lineheight) as i32,
            )
        }

        // pub(super) fn set_height(&self, height: u64) {
        //     self.height.replace(height);
        // }

        // pub(super) fn set_width(&self, width: u64) {
        //     self.width.replace(width);
        // }
    }
}

use std::cell::{Ref, RefCell};
use std::rc::Rc;
use std::sync::RwLock;

use glib::subclass::prelude::*;
use gtk::graphene::Size;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

use super::textbuf::TextBuf;
use super::HighlightDefinitions;

glib::wrapper! {
    pub struct VimGridView(ObjectSubclass<imp::VimGridView>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl VimGridView {
    pub fn new(grid: u64, width: u64, height: u64) -> VimGridView {
        glib::Object::new(&[("grid", &grid), ("width", &width), ("height", &height)])
            .expect("Failed to create `VimGridView`.")
    }

    fn imp(&self) -> &imp::VimGridView {
        imp::VimGridView::from_instance(self)
    }

    pub fn set_hldefs(&self, hldefs: Rc<RwLock<HighlightDefinitions>>) {
        self.imp().set_hldefs(hldefs);
    }

    pub fn set_textbuf(&self, textbuf: Rc<RefCell<TextBuf>>) {
        self.imp().set_textbuf(textbuf);
    }

    pub fn set_font_description(&self, desc: &pango::FontDescription) {
        self.pango_context().set_font_description(desc);
    }

    pub fn textbuf(&self) -> Ref<TextBuf> {
        self.imp().textbuf()
    }

    pub fn resize(&self, width: f32, height: f32) {
        // WidgetExt::set_width_request(self, width as i32);
        // WidgetExt::set_height_request(self, height as i32);
        self.imp().textbuf().resize(height as _, width as _);
        self.set_property("width", &width);
        self.set_property("height", &height);
        log::info!(
            " -> resize grid {} from {}x{} to {}x{}",
            self.property::<u64>("grid"),
            self.property::<u64>("width"),
            self.property::<u64>("height"),
            width,
            height
        );
        // self.set_width(width as _);
        // self.set_height(height as _);
    }

    // pub fn set_height(&self, height: u64) {
    //     self.imp().set_height(height);
    // }

    // pub fn set_width(&self, width: u64) {
    //     self.imp().set_width(width);
    // }
}
