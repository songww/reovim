mod imp {
    use core::f32;
    use std::cell::{Cell, Ref};
    use std::rc::Rc;
    use std::sync::RwLock;

    use gtk::{gdk::prelude::*, graphene::Rect, pango, prelude::*, subclass::prelude::*};
    use once_cell::sync::OnceCell;

    use super::super::highlights::HighlightDefinitions;
    use super::super::TextBuf;

    // #[derive(Debug)]
    pub struct VimGridView {
        id: Cell<u64>,
        width: Cell<u64>,
        height: Cell<u64>,
        textbuf: Cell<TextBuf>,
        hldefs: OnceCell<Rc<RwLock<HighlightDefinitions>>>,
        metrics: OnceCell<Rc<Cell<crate::app::FontMetrics>>>,
    }

    impl std::fmt::Debug for VimGridView {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("VimGridView")
                .field("grid", &self.id.get())
                .field("width", &self.width.get())
                .field("height", &self.height.get())
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
                id: 0.into(),
                width: 0.into(),
                height: 0.into(),
                hldefs: OnceCell::new(), // Rc::new(RwLock::new(HighlightDefinitions::new()))),
                metrics: OnceCell::new(),
                textbuf: TextBuf::default().into(),
            }
        }
    }

    // Trait shared by all GObjects
    impl ObjectImpl for VimGridView {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
        }

        fn properties() -> &'static [glib::ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecUInt64::new(
                        "id",
                        "grid-id",
                        "id",
                        1,
                        u64::MAX,
                        1,
                        glib::ParamFlags::READWRITE,
                    ),
                    glib::ParamSpecUInt64::new(
                        "width",
                        "cols",
                        "width",
                        0,
                        u64::MAX,
                        0,
                        glib::ParamFlags::READWRITE,
                    ),
                    glib::ParamSpecUInt64::new(
                        "height",
                        "rows",
                        "height",
                        0,
                        u64::MAX,
                        0,
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
                "id" => {
                    self.id.replace(value.get::<u64>().unwrap());
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
                "id" => self.id.get().to_value(),
                "width" => self.width.get().to_value(),
                "height" => self.height.get().to_value(),
                _ => unimplemented!(),
            }
        }
    }

    // Trait shared by all widgets
    impl WidgetImpl for VimGridView {
        fn snapshot(&self, widget: &Self::Type, snapshot: &gtk::Snapshot) {
            const SCALE: f32 = pango::SCALE as f32;
            let pctx = widget.pango_context();
            let font_desc = widget.pango_context().font_description().unwrap();
            log::debug!(
                "snapshot grid {} font description {}",
                self.id.get(),
                font_desc.to_str()
            );
            log::info!(
                "grid {} height {} width {} cols {} rows {}",
                self.id.get(),
                self.height.get(),
                self.width.get(),
                self.textbuf().cols(),
                self.textbuf().rows()
            );
            assert_eq!(self.height.get(), self.textbuf().rows() as u64);
            assert_eq!(self.width.get(), self.textbuf().cols() as u64);
            let (width, height) = self.size_required();

            let hldefs = self.hldefs.get().unwrap().read().unwrap();

            let layout = pango::Layout::new(&pctx);
            let linespace = self.metrics.get().unwrap().get().linespace();
            if linespace > 0. {
                layout.set_spacing(linespace as _);
            }
            layout.set_font_description(Some(&font_desc));
            self.textbuf().layout(&layout, &hldefs);
            // let (w, h) = layout.size();
            // let (w, h) = (w as f32 / SCALE, h as f32 / SCALE);
            let (w, h) = layout.pixel_size();
            log::info!(
                "snapshoting grid {} size required {}x{}",
                self.id.get(),
                width,
                height
            );
            log::info!("grid {} layout size {}x{}", self.id.get(), w, h);
            log::info!(
                "grid {} layout line-height: {}",
                self.id.get(),
                layout.line(1).unwrap().height() as f32 / SCALE
            );

            /*
                        if let Some(background) = hldefs.defaults().and_then(|defaults| defaults.background) {
                            let style_context = widget.style_context();
                            style_context.save();
                            let provider = gtk::CssProvider::new();
                            const U8MAX: f32 = u8::MAX as f32;
                            let css = format!(
                                ".vim-view-grid-1 {{
                background-color: #{:02x}{:02x}{:02x};
            }}",
                                (background.red() * U8MAX) as u8,
                                (background.green() * U8MAX) as u8,
                                (background.blue() * U8MAX) as u8
                            );
                            log::info!("css: `{}`", &css);
                            provider.load_from_data(css.as_bytes());
                            style_context.add_provider(&provider, 1);
                            snapshot.render_background(&style_context, 0., 0., w as _, h as _);
                            snapshot.render_frame(&style_context, 0., 0., w as _, h as _);
                            style_context.restore();
                        }
                        */

            let rect = Rect::new(0., 0., w as _, h as _);
            let cr = snapshot.append_cairo(&rect);

            pangocairo::update_context(&cr, &pctx);
            pangocairo::update_layout(&cr, &layout);
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
            let (w, h) = self.size_required();
            log::error!(
                "measuring grid {} orientation {} for_size {} size_required {}x{}",
                self.id.get(),
                orientation,
                for_size,
                w,
                h,
            );
            match orientation {
                gtk::Orientation::Vertical => (h, h, -1, -1),
                gtk::Orientation::Horizontal => (w, w, -1, -1),
                _ => self.parent_measure(widget, orientation, for_size),
            }
        }
    }

    impl VimGridView {
        pub(super) fn set_hldefs(&self, hldefs: Rc<RwLock<HighlightDefinitions>>) {
            self.hldefs.set(hldefs).expect("hldefs must set only once.");
        }

        pub(super) fn set_textbuf(&self, textbuf: TextBuf) {
            self.textbuf.replace(textbuf);
        }

        pub(super) fn textbuf(&self) -> Ref<super::super::textbuf::TextBuf> {
            unsafe { &*self.textbuf.as_ptr() }.borrow()
        }

        pub(super) fn set_width(&self, width: u64) {
            self.width.replace(width);
        }

        pub(super) fn set_height(&self, height: u64) {
            self.height.replace(height);
        }

        pub(super) fn set_font_metrics(&self, metrics: Rc<Cell<crate::app::FontMetrics>>) {
            self.metrics
                .set(metrics)
                .expect("FontMetrics must set only once.");
        }

        pub(super) fn size_required(&self) -> (i32, i32) {
            let width = self.width.get() as f64;
            let height = self.height.get() as f64;
            let metrics = self.metrics.get().unwrap().get();
            let w = width * metrics.charwidth();
            let h = height * (metrics.lineheight() + metrics.linespace());
            (w.ceil() as i32, h.ceil() as i32)
        }
    }
}

use std::cell::{Cell, Ref};
use std::rc::Rc;
use std::sync::RwLock;

use glib::subclass::prelude::*;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

use super::{HighlightDefinitions, TextBuf};

glib::wrapper! {
    pub struct VimGridView(ObjectSubclass<imp::VimGridView>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl VimGridView {
    pub fn new(id: u64, width: u64, height: u64) -> VimGridView {
        glib::Object::new(&[("id", &id), ("width", &width), ("height", &height)])
            .expect("Failed to create `VimGridView`.")
    }

    fn imp(&self) -> &imp::VimGridView {
        imp::VimGridView::from_instance(self)
    }

    pub fn set_hldefs(&self, hldefs: Rc<RwLock<HighlightDefinitions>>) {
        self.imp().set_hldefs(hldefs);
    }

    pub fn set_textbuf(&self, textbuf: TextBuf) {
        self.imp().set_textbuf(textbuf);
    }

    pub fn set_font_description(&self, desc: &pango::FontDescription) {
        self.pango_context().set_font_description(desc);
    }

    pub fn set_font_metrics(&self, metrics: Rc<Cell<crate::app::FontMetrics>>) {
        self.imp().set_font_metrics(metrics);
    }

    pub fn textbuf(&self) -> Ref<super::textbuf::TextBuf> {
        self.imp().textbuf()
    }

    pub fn resize(&self, width: u64, height: u64) {
        self.imp().set_width(width);
        self.imp().set_height(height);
        self.imp().textbuf().resize(height as _, width as _);
    }
}
