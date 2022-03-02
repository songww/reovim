mod cursor;
// mod state;
// mod vfx;

pub use cursor::{Cursor as VimCursor, CursorMode, CursorShape};
use gtk::prelude::{StyleContextExt, WidgetExt};

use relm4::drawing::DrawContext;
use relm4::{ComponentUpdate, Model, Sender, Widgets};

use crate::app::{AppMessage, AppModel};
use crate::grapheme::Coord;
use crate::vimview::TextCell;

impl Model for VimCursor {
    type Msg = CursorMessage;
    type Widgets = CursorWidgets;
    type Components = ();
}

pub enum CursorMessage {
    Goto(u64, Coord, TextCell),
    SetMode(CursorMode),
    SetCell(TextCell),
}

impl ComponentUpdate<AppModel> for VimCursor {
    fn init_model(parent_model: &AppModel) -> Self {
        VimCursor::new(
            parent_model.pctx.clone(),
            parent_model.metrics.clone(),
            parent_model.hldefs.clone(),
        )
    }

    fn update(
        &mut self,
        message: CursorMessage,
        _components: &(),
        _sender: Sender<CursorMessage>,
        _parent_sender: Sender<AppMessage>,
    ) {
        match message {
            CursorMessage::Goto(grid, coord, cell) => {
                self.cell = cell;
                self.grid = grid;
                self.coord = coord;
            }
            CursorMessage::SetMode(mode) => {
                self.set_mode(mode);
            }
            CursorMessage::SetCell(cell) => {
                self.cell = cell;
            }
        }
    }
}

#[relm_macros::widget(pub)]
impl Widgets<VimCursor, AppModel> for CursorWidgets {
    view! {
        da = gtk::DrawingArea {
            set_widget_name: "cursor",
            set_visible: true,
            set_hexpand: true,
            set_vexpand: true,
            set_can_focus: false,
            set_sensitive: false,
            set_focus_on_click: false,
            set_css_classes: &["blink"],
        }
    }

    additional_fields! {
        dh: relm4::drawing::DrawHandler,
        provider: gtk::CssProvider,
    }

    fn post_init() {
        let provider = gtk::CssProvider::new();
        let mut dh = relm4::drawing::DrawHandler::new().unwrap();
        dh.init(&da);
        da.style_context()
            .add_provider(&provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
    }

    fn pre_view() {
        log::trace!("start cursor view.");
        let instant = std::time::Instant::now();
        self.da.set_opacity(1.);
        self.da.remove_css_class("blink");
        self.da.style_context().remove_provider(&self.provider);
        let cr = self.dh.get_context().unwrap();
        model.drawing(&cr);
        self.da
            .style_context()
            .add_provider(&self.provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
        if let Some(blinking) = model.maybe_blinking() {
            self.provider.load_from_data(blinking.as_bytes());
            self.da
                .style_context()
                .add_provider(&self.provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
            self.da.add_css_class("blink");
        }
        log::trace!(
            "cursor view used {:.3}ms",
            instant.elapsed().as_secs_f32() * 1000.
        );
    }
}

impl VimCursor {
    fn maybe_blinking(&self) -> Option<String> {
        let blinkon = self.blinkon().filter(|blinkon| *blinkon > 0)?;
        let blinkoff = self.blinkoff().filter(|blinkoff| *blinkoff > 0)?;
        let blinkwait = self.blinkwait().filter(|blinkwait| *blinkwait > 0)?;
        let css = format!(
            ".blink {{
  animation-name: blinking;
  animation-delay: {}ms;
  animation-duration: {}ms;
  animation-iteration-count: infinite;
  animation-timing-function: steps(2, start);
}}

@keyframes blinking {{
  {}% {{ opacity: 0; }}
}}
",
            blinkwait,
            blinkon + blinkoff,
            blinkon * 100 / (blinkon + blinkoff)
        );
        log::debug!("css {} {}: \n{}", blinkon, blinkoff, &css);
        Some(css)
    }

    fn drawing(&self, cr: &DrawContext) {
        // clear previous position.
        cr.set_operator(cairo::Operator::Clear);
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
        cr.paint().expect("Couldn't fill context");
        // paintable.
        cr.set_operator(cairo::Operator::Over);
        let bg = self.background();
        let fg = self.foreground();
        let cell = self.cell();
        let metrics = self.metrics.get();
        let (x, y, width, height) = self.rectangle(metrics.width(), metrics.height());
        log::debug!("drawing cursor at {}x{}.", x, y);
        match self.shape {
            CursorShape::Block => {
                use pango::AttrType;
                let attrs = pango::AttrList::new();
                cell.attrs
                    .iter()
                    .filter_map(|attr| match attr.type_() {
                        AttrType::Family
                        | AttrType::Style
                        | AttrType::Weight
                        | AttrType::Variant
                        | AttrType::Underline
                        | AttrType::Strikethrough
                        | AttrType::Overline => {
                            let mut attr = attr.clone();
                            attr.set_start_index(0);
                            attr.set_end_index(pango::ATTR_INDEX_TO_TEXT_END);
                            Some(attr)
                        }
                        _ => None,
                    })
                    .for_each(|attr| attrs.insert(attr));
                log::debug!("cursor cell '{}' wide {}", cell.text, self.width);
                let itemized = &pango::itemize(
                    &self.pctx,
                    &cell.text,
                    0,
                    cell.text.len() as _,
                    &attrs,
                    None,
                )[0];
                let mut glyph_string = pango::GlyphString::new();
                pango::shape(&cell.text, itemized.analysis(), &mut glyph_string);
                let glyphs = glyph_string.glyph_info_mut();
                assert_eq!(glyphs.len(), 1);
                let geometry = glyphs[0].geometry_mut();
                let width = (metrics.width() * self.width).ceil() as i32;
                if geometry.width() > 0 && geometry.width() != width {
                    let x_offset = geometry.x_offset() - (geometry.width() - width) / 2;
                    geometry.set_width(width);
                    geometry.set_x_offset(x_offset);
                    log::debug!("cursor glyph width {}", width);
                }
                // 试试汉字
                cr.save().unwrap();
                cr.rectangle(x, y, width as f64, metrics.height());
                cr.set_source_rgba(
                    bg.red() as f64,
                    bg.green() as f64,
                    bg.blue() as f64,
                    bg.alpha() as f64,
                );
                cr.fill().unwrap();
                cr.restore().unwrap();
                cr.set_source_rgba(
                    fg.red() as f64,
                    fg.green() as f64,
                    fg.blue() as f64,
                    fg.alpha() as f64,
                );
                cr.move_to(x + geometry.width() as f64 / 2., y + metrics.ascent());
                pangocairo::show_glyph_string(cr, &itemized.analysis().font(), &mut glyph_string);
            }
            _ => {
                log::debug!("drawing cursor with {}x{}", width, height);
                cr.set_source_rgba(
                    bg.red() as f64,
                    bg.green() as f64,
                    bg.blue() as f64,
                    bg.alpha() as f64,
                );
                cr.rectangle(x, y, width, height);
                cr.fill().unwrap();
            }
        }
    }
}
