mod imp {
    use core::f32;
    use std::cell::{Cell, Ref};
    use std::rc::Rc;

    use glib::translate::{from_glib_none, ToGlibPtr};
    use gtk::{gdk::prelude::*, graphene::Rect, subclass::prelude::*};
    use parking_lot::RwLock;

    use crate::metrics::Metrics;
    use crate::vimview::textbuf::Lines;
    use crate::vimview::TextCell;

    use super::super::highlights::HighlightDefinitions;
    use super::super::TextBuf;

    const PANGO_SCALE: f64 = pango::SCALE as f64;

    #[derive(Clone, Debug)]
    struct CharAttr<'c> {
        c: char,
        cell: &'c TextCell,
        // visible width. how much cell used.
        viswidth: f64,
    }

    // #[derive(Debug)]
    pub struct VimGridView {
        id: Cell<u64>,
        width: Cell<u64>,
        height: Cell<u64>,
        is_float: Cell<bool>,
        textbuf: Cell<TextBuf>,
    }

    impl std::fmt::Debug for VimGridView {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("VimGridView")
                .field("grid", &self.id.get())
                .field("width", &self.width.get())
                .field("height", &self.height.get())
                .field("is-float-window", &self.is_float.get())
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
                is_float: false.into(),
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
            let instant = std::time::Instant::now();
            self.parent_snapshot(widget, snapshot);
            let textbuf = self.textbuf();
            let pctx = textbuf.pango_context();
            pctx.set_base_dir(pango::Direction::Ltr);

            let (width, height) = self.size_required();

            let hldefs = textbuf.hldefs().unwrap();
            let hldefs = hldefs.read();

            let metrics = textbuf.metrics().unwrap().get();

            let rect = Rect::new(0., 0., width as _, height as _);

            let hldef = hldefs.get(HighlightDefinitions::DEFAULT);
            let mut background = hldef
                .map(|style| &style.colors)
                .and_then(|colors| colors.background)
                .unwrap();
            if self.is_float.get() {
                // float window should respect blend for background.
                let blend = hldef.map(|style| style.blend).unwrap_or(0);
                let alpha = (100 - blend) as f32 / 100.;
                background.set_alpha(alpha);
            }
            snapshot.append_color(&background, &rect);

            let cr = snapshot.append_cairo(&rect);

            let mut y = metrics.ascent();

            let rows = textbuf.rows();
            log::debug!("text to render:");
            let desc = pctx.font_description();
            let mut layout = pango::Layout::new(&pctx);
            layout.set_auto_dir(false);
            layout.set_font_description(desc.as_ref());
            let textbuf = self.textbuf();
            let lines = textbuf.lines();
            for lineno in 0..rows {
                cr.move_to(0., y);
                y += metrics.height();
                let line = lines.get(lineno).unwrap();
                let layoutline = if let Some((layout, layoutline)) = line.cache() {
                    unsafe {
                        let layout: *mut pango::ffi::PangoLayout = layout.to_glib_none().0;
                        (*layoutline.to_glib_none().0).layout = layout;
                    };
                    pangocairo::update_layout(&cr, &layout);
                    layoutline
                } else {
                    let layoutline = self.layoutline(&mut layout, &lines, lineno, &metrics);
                    line.set_cache(layout.copy().unwrap(), layoutline.clone());
                    pangocairo::update_layout(&cr, &layout);
                    layoutline
                };
                pangocairo::show_layout_line(&cr, &layoutline);
            }
            let elapsed = instant.elapsed().as_secs_f32() * 1000.;
            log::info!("snapshot used: {:.3}ms", elapsed);
        }

        fn measure(
            &self,
            widget: &Self::Type,
            orientation: gtk::Orientation,
            for_size: i32,
        ) -> (i32, i32, i32, i32) {
            let (w, h) = self.size_required();
            log::debug!(
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
            self.textbuf().set_hldefs(hldefs);
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

        pub(super) fn set_is_float(&self, is_float: bool) {
            self.is_float.replace(is_float);
        }

        pub(super) fn set_metrics(&self, metrics: Rc<Cell<crate::metrics::Metrics>>) {
            self.textbuf().set_metrics(metrics)
        }

        pub(super) fn size_required(&self) -> (i32, i32) {
            let textbuf = self.textbuf();
            let width = textbuf.cols() as f64;
            let height = textbuf.rows() as f64;
            let metrics = textbuf.metrics().unwrap().get();
            let w = width * metrics.width();
            let h = height * metrics.height();
            (w.ceil() as i32, h.ceil() as i32)
        }

        fn layoutline(
            &self,
            layout: &mut pango::Layout,
            lines: &Lines,
            lineno: usize,
            metrics: &Metrics,
        ) -> pango::LayoutLine {
            let line = lines.get(lineno).unwrap();
            let cols = line.len();
            let mut text = String::new();
            let mut chars: Vec<Option<CharAttr>> = vec![None; cols * 2];
            let attrs = pango::AttrList::new();
            for col in 0..cols {
                let cell = line.get(col).expect("Invalid cols and rows");
                if cell.start_index == cell.end_index {
                    continue;
                }
                if chars.len() <= text.len() {
                    chars.resize(chars.len() * 2, None);
                }
                let mut chars_ = cell.text.chars();
                let mut index = text.len();

                if let Some(c) = chars_.next() {
                    chars[index] = {
                        CharAttr {
                            c,
                            cell,
                            viswidth: if cell.double_width {
                                2.
                            } else if pango::is_zero_width(c) {
                                0.
                            } else {
                                1.
                            },
                        }
                    }
                    .into();
                    index += c.to_string().bytes().len();
                } else {
                    continue;
                }

                // normally only one char at here
                for c in chars_ {
                    chars[index] = {
                        CharAttr {
                            c,
                            cell,
                            viswidth: 0.,
                        }
                    }
                    .into();
                    index += c.to_string().bytes().len();
                }
                text.push_str(&cell.text);
                cell.attrs
                    .clone()
                    .into_iter()
                    .for_each(|attr| attrs.change(attr));
            }
            layout.set_text(&text);
            layout.set_attributes(Some(&attrs));
            let unknown_glyphs = layout.unknown_glyphs_count();
            log::trace!(
                "grid {} line {} baseline {} line-height {} space {} char-height {} unknown_glyphs {}",
                self.id.get(),
                lineno,
                layout.baseline(),
                layout.line_readonly(0).unwrap().height(),
                metrics.linespace(),
                metrics.charheight() * PANGO_SCALE,
                unknown_glyphs
            );

            let required_lineheight = metrics.charheight() * PANGO_SCALE;
            let real_lineheight = layout.line_readonly(0).unwrap().height() as f64;
            if required_lineheight != real_lineheight {
                attrs.insert_before({
                    let mut attr =
                        pango::AttrInt::new_line_height_absolute(required_lineheight as i32);
                    attr.set_start_index(0);
                    attr.set_end_index(pango::ATTR_INDEX_TO_TEXT_END);
                    attr
                });

                layout.set_attributes(Some(&attrs));
                layout.context_changed();
            }
            if required_lineheight as i32 != layout.line_readonly(0).unwrap().height() {
                log::debug!("Scale line height failed.");
            }


            let layoutline: pango::LayoutLine = unsafe { self.align(layout, &chars, &metrics) };
            layoutline
        }

        unsafe fn align(
            &self,
            layout: &mut pango::Layout,
            chars: &Vec<Option<CharAttr>>,
            metrics: &Metrics,
        ) -> pango::LayoutLine {
            // let _baseline = pango::ffi::pango_layout_get_baseline(layout.to_glib_none().0);
            let layoutline = pango::ffi::pango_layout_get_line(layout.to_glib_none().0, 0);
            let mut runs = (*layoutline).runs;
            loop {
                let run = (*runs).data as *mut pango::ffi::PangoLayoutRun;
                let item = (*run).item;
                let font = (*item).analysis.font;
                let glyph_string = (*run).glyphs;
                let num_glyphs = (*glyph_string).num_glyphs as usize;
                let log_clusters =
                    { std::slice::from_raw_parts((*glyph_string).log_clusters, num_glyphs) };
                let glyphs = { std::slice::from_raw_parts_mut((*glyph_string).glyphs, num_glyphs) };
                let ink_rect = std::ptr::null_mut();
                let mut logical_rect = pango::ffi::PangoRectangle {
                    x: 0,
                    y: 0,
                    width: 0,
                    height: 0,
                };

                pango::ffi::pango_glyph_string_extents(
                    glyph_string,
                    font,
                    ink_rect,
                    &mut logical_rect,
                );
                log::trace!("{} glyphs item.offset {}", num_glyphs, (*item).offset);
                log::trace!("log_clusters{:?}", log_clusters);
                for (glyph, log_cluster) in glyphs.iter_mut().zip(log_clusters) {
                    let index = ((*item).offset + log_cluster) as usize;
                    let isfirst = index == 0;
                    let charattr = chars.get(index).unwrap().as_ref().unwrap_or_else(|| {
                        // lazy, format is expensive.
                        panic!("index {} out of range, {:?}", index, &chars)
                    });
                    if charattr.viswidth == 0. {
                        log::debug!("Skipping zerowidth: {}", charattr.cell.text);
                        continue;
                    }
                    let width = metrics.charwidth() * charattr.viswidth * PANGO_SCALE;
                    let width = width.ceil() as i32;
                    let geometry = &mut glyph.geometry;
                    // log::info!("{} char-cell {:?}", index, charattr.cell);
                    if geometry.width > 0 && geometry.width != width {
                        let x_offset = if isfirst {
                            geometry.x_offset
                        } else {
                            geometry.x_offset - (geometry.width - width) / 2
                        };
                        let y_offset = geometry.y_offset
                            - (logical_rect.height / pango::SCALE - metrics.height() as i32) / 2;
                        // 啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊
                        log::debug!(
                            "adjusting ({})  width {}->{}  x-offset {}->{} y-offset {} -> {} is-start-char {}",
                            charattr.c,
                            geometry.width,
                            width,
                            geometry.x_offset,
                            x_offset,
                            geometry.y_offset,
                            y_offset,
                            isfirst
                        );
                        geometry.width = width;
                        geometry.x_offset = x_offset;
                        // geometry.y_offset = y_offset;
                    }
                }
                runs = (*runs).next;
                if runs.is_null() {
                    break;
                }
            }
            from_glib_none(layoutline)
        }
    }
}

use std::cell::{Cell, Ref};
use std::rc::Rc;

use glib::subclass::prelude::*;
use gtk::prelude::*;
use parking_lot::RwLock;

use super::{HighlightDefinitions, TextBuf};

glib::wrapper! {
    pub struct VimGridView(ObjectSubclass<imp::VimGridView>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for VimGridView {
    fn default() -> Self {
        VimGridView::new(u64::MAX, 0, 0)
    }
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

    pub fn set_is_float(&self, is_float: bool) {
        self.imp().set_is_float(is_float);
    }

    pub fn set_font_description(&self, desc: &pango::FontDescription) {
        self.pango_context().set_font_description(desc);
    }

    pub fn set_metrics(&self, metrics: Rc<Cell<crate::metrics::Metrics>>) {
        self.imp().set_metrics(metrics);
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
