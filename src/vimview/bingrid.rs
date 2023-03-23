mod imp {
    use core::f32;
    use std::cell::{Cell, Ref};
    use std::rc::Rc;
    use std::sync::RwLock;

    use glib::{ParamSpec, Value as GValue};
    use gtk::prelude::*;
    use gtk::{graphene::Rect, subclass::prelude::*};
    use tracing::{info, trace};

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
    pub struct BinGrid {
        gid: Cell<u64>,
        width: Cell<u64>,
        height: Cell<u64>,
        is_float_window: Cell<bool>,
        textbuf: Cell<TextBuf>,
    }

    impl std::fmt::Debug for BinGrid {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("BinGrid")
                .field("grid", &self.gid.get())
                .field("width", &self.width.get())
                .field("height", &self.height.get())
                .field("is-float-window", &self.is_float_window.get())
                .finish_non_exhaustive()
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for BinGrid {
        const NAME: &'static str = "VimGridView";
        type ParentType = gtk::Widget;
        type Type = super::BinGrid;

        fn new() -> Self {
            Self {
                gid: 0.into(),
                width: 0.into(),
                height: 0.into(),
                is_float_window: false.into(),
                textbuf: TextBuf::default().into(),
            }
        }
    }

    // Trait shared by all GObjects
    impl ObjectImpl for BinGrid {
        fn properties() -> &'static [glib::ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                let mut id_builder = glib::ParamSpecUInt64::builder("id")
                    .minimum(1)
                    .maximum(u64::MAX)
                    .default_value(1);
                id_builder.set_nick("grid-id".into());
                id_builder.set_blurb("gid".into());
                id_builder.set_flags(glib::ParamFlags::READWRITE);
                let id = id_builder.build();
                let mut width_builder = glib::ParamSpecUInt64::builder("width")
                    .minimum(0)
                    .maximum(u64::MAX)
                    .default_value(0);
                width_builder.set_nick("cols".into());
                width_builder.set_blurb("width".into());
                width_builder.set_flags(glib::ParamFlags::READWRITE);
                let width = width_builder.build();

                let mut height_builder = glib::ParamSpecUInt64::builder("height")
                    .minimum(0)
                    .maximum(u64::MAX)
                    .default_value(0);
                height_builder.set_nick("rows".into());
                height_builder.set_blurb("height".into());
                height_builder.set_flags(glib::ParamFlags::READWRITE);
                let height = height_builder.build();
                vec![id, width, height]
            });

            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &GValue, pspec: &ParamSpec) {
            match pspec.name() {
                "gid" => {
                    self.gid.replace(value.get::<u64>().unwrap());
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

        fn property(&self, _id: usize, pspec: &ParamSpec) -> GValue {
            match pspec.name() {
                "gid" => self.gid.get().to_value(),
                "width" => self.width.get().to_value(),
                "height" => self.height.get().to_value(),
                _ => unimplemented!(),
            }
        }
    }

    // Trait shared by all widgets
    impl WidgetImpl for BinGrid {
        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let instant = std::time::Instant::now();
            self.parent_snapshot(snapshot);
            let textbuf = self.textbuf();
            let pctx = textbuf.pango_context();
            pctx.set_base_dir(pango::Direction::Ltr);

            let (width, height) = self.size_required();

            let hldefs = textbuf.hldefs().unwrap();
            let hldefs = hldefs.read().unwrap();

            let metrics = textbuf.metrics().unwrap().get();

            let rect = Rect::new(0., 0., width as _, height as _);

            let hldef = hldefs.get(HighlightDefinitions::DEFAULT);
            let mut background = hldef
                .map(|style| &style.colors)
                .and_then(|colors| colors.background)
                .unwrap();
            if self.is_float_window.get() {
                // float window should respect blend for background.
                let blend = hldef.map(|style| style.blend).unwrap_or(0);
                let alpha = (100 - blend) as f32 / 100.;
                background.set_alpha(alpha);
            }
            snapshot.append_color(&background, &rect);

            let scale_factor = self.obj().scale_factor();
            // let scale_factor = self.scale_factor();
            let cr = snapshot.append_cairo(&rect);
            cr.target()
                .set_device_scale(scale_factor as f64, scale_factor as f64);

            let mut y = metrics.ascent();

            let rows = textbuf.rows();
            trace!("text to render:");
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
                    let layout = layout.as_ptr();
                    unsafe {
                        (*layoutline.as_ptr()).layout = layout;
                    };
                    layoutline
                } else {
                    let layoutline = self.layoutline(&mut layout, &lines, lineno, &metrics);
                    line.set_cache(layout.copy(), layoutline.clone());
                    layoutline
                };
                pangocairo::update_layout(&cr, &layout);
                pangocairo::show_layout_line(&cr, &layoutline);
            }
            let elapsed = instant.elapsed().as_secs_f32() * 1000.;
            info!("snapshot used: {:.3}ms", elapsed);
        }

        fn measure(&self, orientation: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
            let (w, h) = self.size_required();
            trace!(
                id = self.gid.get(),
                for_size,
                w,
                h,
                "measuring orientation = {}",
                orientation,
            );
            match orientation {
                gtk::Orientation::Vertical => (h, h, -1, -1),
                gtk::Orientation::Horizontal => (w, w, -1, -1),
                _ => self.parent_measure(orientation, for_size),
            }
        }
    }

    impl adw::subclass::prelude::BinImpl for BinGrid {}

    impl BinGrid {
        pub(super) fn set_gid(&self, gid: u64) {
            self.gid.replace(gid);
        }

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

        pub(super) fn set_float(&self, float: bool) {
            self.is_float_window.replace(float);
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
            trace!(
                grid = self.gid.get(),
                lineno,
                baseline = layout.baseline(),
                line_height = layout.line_readonly(0).unwrap().height(),
                linespace = metrics.linespace(),
                char_height = metrics.charheight() * PANGO_SCALE,
                unknown_glyphs,
                "layouting line",
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
                info!("Scale line height failed.");
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
            let ll = layout.line(0).unwrap();
            let layoutline = ll.as_ptr();
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
                trace!("{} glyphs item.offset {}", num_glyphs, (*item).offset);
                trace!("log_clusters {:?}", log_clusters);
                for (glyph, log_cluster) in glyphs.iter_mut().zip(log_clusters) {
                    let index = ((*item).offset + log_cluster) as usize;
                    let isfirst = index == 0;
                    let charattr = chars.get(index).unwrap().as_ref().unwrap_or_else(|| {
                        // lazy, format is expensive.
                        panic!("index {} out of range, {:?}", index, &chars)
                    });
                    if charattr.viswidth == 0. {
                        trace!("Skipping zerowidth: {}", charattr.cell.text);
                        continue;
                    }
                    let width = metrics.charwidth() * charattr.viswidth * PANGO_SCALE;
                    let width = width.ceil() as i32;
                    let geometry = &mut glyph.geometry;
                    // info!("{} char-cell {:?}", index, charattr.cell);
                    if geometry.width > 0 && geometry.width != width {
                        let x_offset = if isfirst {
                            geometry.x_offset
                        } else {
                            geometry.x_offset - (geometry.width - width) / 2
                        };
                        let y_offset = geometry.y_offset
                            - (logical_rect.height / pango::SCALE - metrics.height() as i32) / 2;
                        // 啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊
                        trace!(
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
            ll
        }
    }
}

use std::cell::{Cell, Ref};
use std::rc::Rc;
use std::sync::RwLock;

use glib::subclass::prelude::*;
use gtk::prelude::*;

use super::{HighlightDefinitions, TextBuf};

glib::wrapper! {
    pub struct BinGrid(ObjectSubclass<imp::BinGrid>)
        @extends adw::Bin,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget, gtk::Widget;
}

impl Default for BinGrid {
    fn default() -> Self {
        BinGrid::new(0, 0, 0)
    }
}

impl BinGrid {
    pub fn new(gid: u64, width: u64, height: u64) -> BinGrid {
        glib::Object::builder()
            .property("gid", &gid)
            .property("width", &width)
            .property("height", &height)
            .build()
    }

    fn imp(&self) -> &imp::BinGrid {
        imp::BinGrid::from_obj(self)
    }

    pub fn set_gid(&self, gid: u64) {
        self.imp().set_gid(gid);
    }

    pub fn set_width(&self, width: u64) {
        self.imp().set_width(width);
    }
    pub fn set_height(&self, height: u64) {
        self.imp().set_height(height);
    }

    pub fn set_hldefs(&self, hldefs: Rc<RwLock<HighlightDefinitions>>) {
        self.imp().set_hldefs(hldefs);
    }

    pub fn set_textbuf(&self, textbuf: TextBuf) {
        self.imp().set_textbuf(textbuf);
    }

    pub fn set_float(&self, float: bool) {
        self.imp().set_float(float);
    }

    pub fn set_font_description(&self, desc: &pango::FontDescription) {
        self.pango_context().set_font_description(Some(desc));
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
