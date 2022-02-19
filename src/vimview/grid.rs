mod imp {
    use core::f32;
    use std::cell::{Cell, Ref, RefCell};
    use std::rc::Rc;
    use std::sync::RwLock;

    use gtk::{gdk::prelude::*, graphene::Rect, prelude::*, subclass::prelude::*};
    use once_cell::sync::OnceCell;

    use crate::cursor::Cursor;

    use super::super::highlights::HighlightDefinitions;
    use super::super::TextBuf;

    // #[derive(Debug)]
    pub struct VimGridView {
        id: Cell<u64>,
        width: Cell<u64>,
        height: Cell<u64>,
        is_float: Cell<bool>,
        textbuf: Cell<TextBuf>,
        cursor: RefCell<Option<Cursor>>,
        hldefs: OnceCell<Rc<RwLock<HighlightDefinitions>>>,
        metrics: OnceCell<Rc<Cell<crate::metrics::Metrics>>>,
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
                cursor: None.into(),
                is_float: false.into(),
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
            self.parent_snapshot(widget, snapshot);
            let textbuf = self.textbuf();
            let pctx = textbuf.pango_context();

            let (width, height) = self.size_required();

            let hldefs = self.hldefs.get().unwrap().read().unwrap();

            let metrics = self.metrics.get().unwrap().get();

            let rect = Rect::new(0., 0., width as _, height as _);

            if self.is_float.get() {
                // float window should use blend and drawing background.
                let hldef = hldefs.get(HighlightDefinitions::DEFAULT);
                let blend = hldef.map(|style| style.blend).unwrap_or(0);
                let alpha = (100 - blend) as f32 / 100.;
                let mut background = hldef
                    .map(|style| &style.colors)
                    .and_then(|colors| colors.background)
                    .unwrap();
                background.set_alpha(alpha);
                snapshot.append_color(&background, &rect);
            }

            let cr = snapshot.append_cairo(&rect);

            pangocairo::update_context(&cr, &pctx);
            pangocairo::context_set_font_options(&pctx, {
                cairo::FontOptions::new()
                    .ok()
                    .map(|mut options| {
                        options.set_antialias(cairo::Antialias::Gray);
                        options.set_hint_style(cairo::HintStyle::Default);
                        options
                    })
                    .as_ref()
            });

            let mut y = 0.;

            let cols = textbuf.cols();
            let rows = textbuf.rows();
            let mut text = String::with_capacity(cols);
            log::debug!("text to render:");
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
                let layout = pango::Layout::new(&pctx);
                layout.set_text(&text);
                layout.set_attributes(Some(&attrs));
                let desc = pctx.font_description().map(|mut desc| {
                    // desc.set_variations_static("wght=200,wdth=5");
                    desc
                });
                layout.set_font_description(desc.as_ref());
                // log::info!(
                //     "{} line {} baseline {} stretch",
                //     self.id.get(),
                //     lineno,
                //     layout.baseline(),
                // );
                pangocairo::update_layout(&cr, &layout);
                pangocairo::show_layout(&cr, &layout);
                log::debug!("{}", text);
            }

            // drawing cursor.
            if let Some(ref cursor) = *self.cursor.borrow() {
                const PANGO_SCALE: f32 = pango::SCALE as f32;
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
        }

        /*
        fn snapshot(&self, widget: &Self::Type, snapshot: &gtk::Snapshot) {
            self.parent_snapshot(widget, snapshot);
            const SCALE: f64 = pango::SCALE as f64;
            let pctx = widget.pango_context();
            let font_desc = pctx.font_description().unwrap();

            let items = pango::itemize(&pctx, "A", 0, 1, &pango::AttrList::new(), None);
            let item = &items[0];
            let mut glyphs = pango::GlyphString::new();
            pango::shape("A", item.analysis(), &mut glyphs);
            let (_, logical) = glyphs.extents(&item.analysis().font());
            let fixed_charwidth = logical.width() as f64;
            log::debug!(
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

            let metrics = self.metrics.get().unwrap().get();
            let linespace = metrics.linespace();

            let rect = Rect::new(0., 0., width as _, height as _);

            if self.is_float.get() {
                // float window should use blend and drawing background.
                let hldef = hldefs.get(HighlightDefinitions::DEFAULT);
                let blend = hldef.map(|style| style.blend).unwrap_or(0);
                let alpha = (100 - blend) as f32 / 100.;
                let mut background = hldef
                    .map(|style| &style.colors)
                    .and_then(|colors| colors.background)
                    .unwrap();
                background.set_alpha(alpha);
                snapshot.append_color(&background, &rect);
                // cr.set_source_rgba(
                //     background.red() as _,
                //     background.green() as _,
                //     background.blue() as _,
                //     alpha,
                // );
            }

            let cr = snapshot.append_cairo(&rect);

            pangocairo::update_context(&cr, &pctx);

            let charheight = metrics.charheight() * SCALE;

            let (texts, attrtable) = self.textbuf().for_itemize(&hldefs);
            // attrs.insert({
            //     log::error!("absolute line height set to {}", charheight);
            //     let mut attr = pango::AttrInt::new_line_height_absolute(charheight as _);
            //     attr.set_start_index(0);
            //     attr.set_end_index(text.len() as _);
            //     attr
            // });
            let mut y = 0.;
            for (lno, text) in texts.iter().enumerate() {
                cr.move_to(0., y);
                y += (charheight + linespace) / SCALE;
                let attrs = attrtable.get(lno).unwrap();
                let mut items = pango::itemize(&pctx, &text, 0, text.len() as _, &attrs, None);
                assert_eq!(items[0].offset(), 0);
                // println!(
                //     "total {} items, {} chars, {} bytes, text len {}",
                //     items.len(),
                //     text.chars().count(),
                //     text.bytes().len(),
                //     text.len()
                // );
                for item in items.iter_mut() {
                    let mut glyph_string = pango::GlyphString::new();
                    // assert_eq!(idx, item.offset() as usize);
                    let start_index = item.offset();
                    let end_index = start_index + item.length();
                    // println!(
                    //     "getting text[{}:{}] floor boundary {} ceil boundary {}",
                    //     start_index,
                    //     end_index,
                    //     text.floor_char_boundary(start_index as _),
                    //     text.ceil_char_boundary(end_index as _)
                    // );
                    // let end_index = text.ceil_char_boundary(end_index as _);
                    let s = if let Some(s) = text.get(start_index as usize..end_index as usize) {
                        s
                    } else {
                        continue;
                    };
                    if s.is_empty() {
                        continue;
                    }
                    // let char_ = s.chars().next().unwrap();
                    // if char_.is_control() || char_.is_whitespace() {
                    //     continue;
                    // }
                    pango::shape(s, item.analysis(), &mut glyph_string);

                    let (ink, logical) = glyph_string.extents(&item.analysis().font());
                    // 需要占用几个cell
                    let n_ink_cells = ink.width() as f64 / fixed_charwidth;
                    let n_logical_cells = logical.width() as f64 / fixed_charwidth;
                    log::error!(
                        "{} logical cells and {} ink cells before round.",
                        n_logical_cells,
                        n_ink_cells
                    );
                    let n_ink_cells = n_ink_cells.ceil();
                    let n_logical_cells = n_logical_cells.round();
                    let ncells = if n_ink_cells != 0. && n_ink_cells < n_logical_cells {
                        n_ink_cells
                    } else {
                        n_logical_cells
                    };
                    let required = ncells * fixed_charwidth;
                    let mut spacing = required - logical.width() as f64;
                    if n_ink_cells != 0. && n_ink_cells < n_logical_cells {
                        spacing -= (metrics.charwidth() * SCALE - fixed_charwidth)
                            * (n_logical_cells - n_ink_cells)
                            * 2.
                    }
                    log::error!(
                        "'{}' used {}/{} cells logical width {} ink width {} required width {} fixed width {} adding {} spaces",
                        s,
                        n_logical_cells,
                        n_ink_cells,
                        logical.width(),
                        ink.width(),
                        required,
                        fixed_charwidth,
                        spacing,
                    );
                    log::error!(
                        "width {} ink {:?} logical {:?}",
                        glyph_string.width(),
                        ink,
                        logical
                    );
                    if spacing != 0. {
                        attrs.change({
                            log::error!("applying letter-space {} for '{}'", spacing, s);
                            let mut attr =
                                pango::AttrInt::new_letter_spacing((spacing).round() as i32);
                            attr.set_start_index(start_index as u32);
                            attr.set_end_index(end_index as u32);
                            attr
                        });
                    }
                    // FIXME: Fix baseline for cjk font.
                    // if ink.height() >= logical.height() {
                    //     let height = logical.height() as f64; // * SCALE;
                    //     attrs.change({
                    //         let mut attr =
                    //             pango::AttrInt::new_line_height_absolute(height.ceil() as i32 / 4);
                    //         attr.set_end_index(text.len() as _);
                    //         attr.set_start_index(0);
                    //         attr
                    //     });
                    // };
                    // if logical.y() < ink.y() {
                    //     let rise = ((logical.y() - ink.y()) as f64 / SCALE).ceil() as _;
                    //     println!("applying rise {} for {}", rise, s);
                    //     attrs.change({
                    //         // let mut attr =
                    //         //     pango::AttrInt::new_rise((logical.y() - ink.y()) - logical.y());
                    //         let mut attr = pango::AttrInt::new_rise(rise);
                    //         attr.set_end_index(end_index as _);
                    //         attr.set_start_index(start_index as _);
                    //         attr
                    //     });
                    //     let factor = ink.height() as f64 / logical.height() as f64;
                    //     let size = (factor * lineheight * 0.9).ceil();
                    //     println!(
                    //         "applying new size {} ink {} logical {} -> {:?}",
                    //         size,
                    //         ink.y(),
                    //         logical.y(),
                    //         ink,
                    //     );
                    //     attrs.change({
                    //         let mut attr = pango::AttrSize::new(size as i32);
                    //         attr.set_end_index(end_index as _);
                    //         attr.set_start_index(start_index as _);
                    //         attr
                    //     });
                    // }
                }
                let layout = pango::Layout::new(&pctx);
                layout.set_text(&text);
                layout.set_attributes(Some(&attrs));
                pangocairo::update_layout(&cr, &layout);
                pangocairo::update_context(&cr, &pctx);
                pangocairo::show_layout(&cr, &layout);
            }
            log::debug!("text to render:\n{}", texts.join("\n"));

            if let Some(ref cursor) = *self.cursor.borrow() {
                let (rows, cols) = cursor.pos;

                let lno = rows as usize;

                let cell = self
                    .textbuf()
                    .cell(lno, cols as usize)
                    .expect("cursor position dose not exists.");
                let text = if cell.text.len() > 1 {
                    cell.text.trim()
                } else {
                    &cell.text
                };
                let end_index = text.len() as u32;
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
                let background = cursor.foreground(default_colors);
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
                let foreground = cursor.background(default_colors);
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
                    (cursor_width) / SCALE as f32,
                    (cursor_height) / SCALE as f32,
                );
                log::debug!(
                    "Drawing cursor<'{}'> color {} bounds {:?}",
                    &text,
                    background.to_str(),
                    bounds
                );

                cr.move_to(x, y);
                pangocairo::update_layout(&cr, &cursor_layout);
                pangocairo::show_layout(&cr, &cursor_layout)
            }
        }
        */

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

        pub(super) fn set_cursor(&self, cursor: Option<Cursor>) {
            self.cursor.replace(cursor);
        }

        pub(super) fn set_is_float(&self, is_float: bool) {
            self.is_float.replace(is_float);
        }

        pub(super) fn set_metrics(&self, metrics: Rc<Cell<crate::metrics::Metrics>>) {
            self.metrics
                .set(metrics)
                .expect("FontMetrics must set only once.");
        }

        pub(super) fn size_required(&self) -> (i32, i32) {
            let width = self.width.get() as f64;
            let height = self.height.get() as f64;
            let metrics = self.metrics.get().unwrap().get();
            let w = width * metrics.width();
            let h = height * metrics.height();
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

use crate::cursor::Cursor;

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

    pub fn set_cursor(&self, cursor: Option<Cursor>) {
        self.imp().set_cursor(cursor);
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
