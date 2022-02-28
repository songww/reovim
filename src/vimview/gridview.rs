mod imp {
    use core::f32;
    use std::cell::{Cell, Ref};
    use std::rc::Rc;

    use glib::translate::ToGlibPtr;
    use gtk::{gdk::prelude::*, graphene::Rect, subclass::prelude::*};
    use parking_lot::RwLock;

    use super::super::highlights::HighlightDefinitions;
    use super::super::TextBuf;

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
            self.parent_snapshot(widget, snapshot);
            const PANGO_SCALE: f64 = pango::SCALE as f64;
            let textbuf = self.textbuf();
            let pctx = textbuf.pango_context();

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
                // float window should use blend and drawing background.
                let blend = hldef.map(|style| style.blend).unwrap_or(0);
                let alpha = (100 - blend) as f32 / 100.;
                background.set_alpha(alpha);
            }
            snapshot.append_color(&background, &rect);

            let cr = snapshot.append_cairo(&rect);

            let mut y = metrics.ascent();

            let cols = textbuf.cols();
            let rows = textbuf.rows();
            let mut text = String::with_capacity(cols);
            log::debug!("text to render:");
            let desc = pctx.font_description();
            pctx.set_round_glyph_positions(true);
            let layout = pango::Layout::new(&pctx);
            layout.set_font_description(desc.as_ref());
            pangocairo::update_layout(&cr, &layout);
            for lineno in 0..rows {
                cr.move_to(0., y);
                y += metrics.height();
                text.clear();
                let attrs = pango::AttrList::new();
                for col in 0..cols {
                    let cell = self
                        .textbuf()
                        .cell(lineno, col)
                        .expect("Invalid cols and rows");
                    if cell.start_index == cell.end_index {
                        continue;
                    }
                    text.push_str(&cell.text);
                    cell.attrs
                        .clone()
                        .into_iter()
                        .for_each(|attr| attrs.insert(attr));
                }
                layout.set_text(&text);
                layout.set_attributes(Some(&attrs));
                let unknown_glyphs = layout.unknown_glyphs_count();
                log::debug!(
                    "grid {} line {} baseline {} line-height {} space {} char-height {} unknown_glyphs {}",
                    self.id.get(),
                    lineno,
                    layout.baseline(),
                    layout.line_readonly(0).unwrap().height(),
                    metrics.linespace(),
                    metrics.charheight() * PANGO_SCALE as f64,
                    unknown_glyphs
                );

                /*
                if let Some(mut iter) = layout.iter() {
                    loop {
                        if let Some(run) = iter.run() {
                            let mut glyph_string = run.glyph_string();
                            let c_glyph_string = glyph_string.to_glib_none();
                            let log_clusters = unsafe {
                                let ptr = (*c_glyph_string.0).log_clusters;
                                std::slice::from_raw_parts(ptr, glyph_string.num_glyphs() as usize)
                            };

                            for (glyph, log_cluster) in
                                glyph_string.glyph_info_mut().iter_mut().zip(log_clusters)
                            {
                                let index = (run.item().offset() + log_cluster) as usize;
                                let col = text[..index].chars().count();
                                let cell = self.textbuf().cell(lineno, col).unwrap();
                                let width = if cell.double_width {
                                    metrics.charwidth() * 2.
                                } else {
                                    metrics.charwidth()
                                } * PANGO_SCALE;
                                let width = width.ceil() as i32;
                                let geo_width = glyph.geometry().width();
                                if geo_width > 0 && geo_width != width {
                                    let geometry = glyph.geometry_mut();
                                    geometry.set_width(width);
                                    let x_offset = (geo_width - width) / 2;
                                    //log::error!(
                                    //    "adjusting {}x{}  width {}->{}  x-offset {}->{}",
                                    //    lineno,
                                    //    col,
                                    //    geo_width,
                                    //    width,
                                    //    geometry.x_offset(),
                                    //    x_offset
                                    //);
                                    geometry.set_x_offset(x_offset);
                                }
                            }
                        }

                        if !iter.next_run() {
                            break;
                        }
                    }
                }
                */
                unsafe {
                    let mut isfirst = true;
                    let baseline = pango::ffi::pango_layout_get_baseline(layout.to_glib_none().0);
                    let layoutline = pango::ffi::pango_layout_get_line(layout.to_glib_none().0, 0);
                    let mut runs = (*layoutline).runs;
                    loop {
                        let run = (*runs).data as *mut pango::ffi::PangoLayoutRun;
                        let item = (*run).item;
                        let font = (*item).analysis.font;
                        let glyph_string = (*run).glyphs;
                        let num_glyphs = (*glyph_string).num_glyphs as usize;
                        let log_clusters = {
                            std::slice::from_raw_parts((*glyph_string).log_clusters, num_glyphs)
                        };
                        let glyphs =
                            { std::slice::from_raw_parts_mut((*glyph_string).glyphs, num_glyphs) };
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
                        for (glyph, log_cluster) in glyphs.iter_mut().zip(log_clusters) {
                            let index = ((*item).offset + log_cluster) as usize;
                            let c = text[index..].chars().next().unwrap();
                            let width = if glib::ffi::g_unichar_iswide(c as u32) == 1 {
                                2.
                            } else if glib::ffi::g_unichar_iszerowidth(c as u32) == 1 {
                                0.
                            } else {
                                1.
                            };
                            let width = metrics.charwidth() * width * PANGO_SCALE;
                            let width = width.ceil() as i32;
                            let geometry = &mut glyph.geometry;
                            if geometry.width > 0 && geometry.width != width {
                                let x_offset = if isfirst {
                                    geometry.x_offset
                                } else {
                                    geometry.x_offset - (geometry.width - width) / 2
                                };
                                let y_offset = geometry.y_offset
                                    - (logical_rect.height / pango::SCALE
                                        - metrics.height() as i32)
                                        / 2;
                                isfirst = false;
                                // 啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊啊
                                log::debug!(
                                    "adjusting {} ({})  width {}->{}  x-offset {}->{} y-offset {} -> {}",
                                    lineno,
                                    c,
                                    geometry.width,
                                    width,
                                    geometry.x_offset,
                                    x_offset,
                                    geometry.y_offset,
                                    y_offset
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
                    pangocairo::ffi::pango_cairo_show_layout_line(cr.to_raw_none(), layoutline);
                }

                log::debug!("{}", text);
            }
            // log::info!("{}", text);
            // let font_desc = pango::FontDescription::from_string("Monaco Nerd Font Mono 12");
            // let fm = pangocairo::FontMap::default().unwrap();
            // let pctx = fm.create_context().unwrap();
            // pctx.set_language(&pango::Language::default());
            // pctx.set_base_dir(pango::Direction::Ltr);
            // pctx.set_font_description(&font_desc);
            // pctx.set_round_glyph_positions(true);
            //let layout = pango::Layout::new(&pctx);
            //layout.set_text(&text);
            //let style_context = widget.style_context();
            //snapshot.render_layout(&style_context, 0., 0., &layout);

            // drawing cursor.
            /*
            if let Some(ref cursor) = *self.cursor.borrow() {
                let (rows, cols) = cursor.pos;

                let lineno = rows as usize;

                let cell = match self.textbuf().cell(lineno, cols as usize) {
                    Some(cell) => cell,
                    None => {
                        log::error!(
                            "cursor pos {}x{} of grid {} dose not exists.",
                            cols,
                            lineno,
                            self.id.get()
                        );
                        return;
                    }
                };
                let text = &cell.text;
                let y = rows as f64 * metrics.height();
                let x = cols as f64 * metrics.width();

                let guard = self.hldefs.get().unwrap().read().unwrap();
                let default_hldef = guard.get(0).unwrap();
                let default_colors = guard.defaults().unwrap();
                let mut hldef = default_hldef;
                if let Some(ref id) = cell.hldef {
                    let style = hldefs.get(*id);
                    if let Some(style) = style {
                        hldef = style;
                    }
                }
                let end_index = text.len() as u32;
                let attrs = pango::AttrList::new();
                if hldef.italic {
                    let mut attr = pango::AttrInt::new_style(pango::Style::Italic);
                    attr.set_start_index(0);
                    attr.set_end_index(end_index);
                    attrs.insert(attr);
                }
                if hldef.bold {
                    let mut attr = pango::AttrInt::new_weight(pango::Weight::Bold);
                    attr.set_start_index(0);
                    attr.set_end_index(end_index);
                    attrs.insert(attr);
                }
                const U16MAX: f32 = u16::MAX as f32 + 1.;
                // FIXME: bad color selection.
                let background = cursor.background(default_colors);
                let mut attr = pango::AttrColor::new_background(
                    (background.red() * U16MAX) as _,
                    (background.green() * U16MAX) as _,
                    (background.blue() * U16MAX) as _,
                );
                attr.set_start_index(0);
                attr.set_end_index(end_index);
                attrs.insert(attr);
                let mut attr =
                    pango::AttrInt::new_background_alpha((background.alpha() * U16MAX) as u16);
                attr.set_start_index(0);
                attr.set_end_index(end_index);
                attrs.insert(attr);
                let foreground = cursor.foreground(default_colors);
                let mut attr = pango::AttrColor::new_foreground(
                    (foreground.red() * U16MAX) as _,
                    (foreground.green() * U16MAX) as _,
                    (foreground.blue() * U16MAX) as _,
                );
                attr.set_start_index(0);
                attr.set_end_index(end_index);
                attrs.insert(attr);
                let mut attr =
                    pango::AttrInt::new_foreground_alpha((foreground.alpha() * U16MAX) as u16);
                attr.set_start_index(0);
                attr.set_end_index(end_index);
                attrs.insert(attr);

                let cursor_layout = pango::Layout::new(&pctx);
                // FIXME: Fix letter-spacing

                cursor_layout.set_text(&text);
                cursor_layout.set_attributes(Some(&attrs));
                let pos = cursor_layout.index_to_pos(0);
                let (cursor_width, cursor_height) =
                    cursor.size(pos.width() as f32, pos.height() as f32);
                let bounds = Rect::new(
                    (x) as f32,
                    (y) as f32,
                    (cursor_width) / PANGO_SCALE,
                    (cursor_height) / PANGO_SCALE,
                );
                log::info!(
                    "Drawing cursor<'{}'> color {} bounds {:?}",
                    &text,
                    background.to_str(),
                    bounds
                );

                cr.move_to(x, y);
                pangocairo::update_layout(&cr, &cursor_layout);
                pangocairo::show_layout(&cr, &cursor_layout)
            }
                */
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
