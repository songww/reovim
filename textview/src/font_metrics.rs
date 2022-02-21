use std::str::FromStr;
use std::sync::Arc;
use std::{collections::HashMap, sync::RwLock};

use glib::{Cast, ObjectExt};
use gtk::traits::WidgetExt;
use once_cell::sync::{Lazy, OnceCell};
use pango::prelude::FontMapExt;

#[derive(Clone, Debug)]
pub enum Coverage {
    PangoLayoutLine(Option<pango::LayoutLine>),
    PangoGlyphString(pango::Font, pango::GlyphString),
    CairoGlyph(pango::Glyph, cairo::ScaledFont),
    Unknown,
}

#[derive(Clone, Debug)]
pub struct CharAnalysis {
    width: f64,
    coverage: Coverage,
    unknown_chars: Option<usize>,
}

impl CharAnalysis {
    pub fn coverage(&self) -> &Coverage {
        &self.coverage
    }
    pub fn width(&self) -> f64 {
        self.width
    }
    pub fn set_width(&mut self, width: f64) {
        self.width = width;
    }
    pub fn set_coverage(&mut self, coverage: Coverage) {
        self.coverage = coverage;
    }
    pub fn set_unknown_chars(&mut self, unknown_chars: usize) {
        self.unknown_chars = unknown_chars.into()
    }
    pub fn new(width: f64, unknown_chars: Option<usize>) -> CharAnalysis {
        CharAnalysis {
            width,
            unknown_chars,
            coverage: Coverage::Unknown,
        }
    }
}

const SINGLE_WIDE_CHARACTERS: &'static str = concat!(
    "  ! \" # $ % & ' ( ) * + , - . / ",
    "0 1 2 3 4 5 6 7 8 9 ",
    ": ; < = > ? @ ",
    "A B C D E F G H I J K L M N O P Q R S T U V W X Y Z ",
    "[ \\ ] ^ _ ` ",
    "a b c d e f g h i j k l m n o p q r s t u v w x y z ",
    "{ | } ~ ",
    ""
);

const PANGO_SCALE: f64 = pango::SCALE as f64;

pub static PANGO_CONTEXT: OnceCell<fragile::Fragile<pango::Context>> = OnceCell::new();

static FONT_ANALYSIS_CACHES: Lazy<
    fragile::Fragile<RwLock<HashMap<pango::Context, RcFontAnalysis>>>,
> = Lazy::new(|| fragile::Fragile::new(RwLock::new(HashMap::new())));

pub struct FontAnalysisCaches;

impl FontAnalysisCaches {
    pub fn get(k: &pango::Context) -> Option<RcFontAnalysis> {
        FONT_ANALYSIS_CACHES
            .get()
            .read()
            .ok()?
            .get(k)
            .map(Arc::clone)
    }

    pub fn set(k: pango::Context, v: RcFontAnalysis) -> Option<RcFontAnalysis> {
        FONT_ANALYSIS_CACHES.get().write().unwrap().insert(k, v)
    }
}

pub type RcFontAnalysis = Arc<FontAnalysis>;

/*
pub struct FontAnalysisRef<'v, 'k> {
    guard: std::sync::RwLockReadGuard<'v, HashMap<pango::Context, FontAnalysis>>,
    k: &'k pango::Context,
}

pub struct FontAnalysisMut<'v, 'k> {
    guard: std::sync::RwLockWriteGuard<'v, HashMap<pango::Context, FontAnalysis>>,
    k: &'k pango::Context,
}

impl<'v, 'k> Deref for FontAnalysisRef<'v, 'k> {
    type Target = FontAnalysis;
    fn deref(&self) -> &Self::Target {
        self.guard.get(self.k).unwrap()
    }
}

impl<'v, 'k> Deref for FontAnalysisMut<'v, 'k> {
    type Target = FontAnalysis;
    fn deref(&self) -> &Self::Target {
        self.guard.get(self.k).unwrap()
    }
}

impl<'v, 'k> DerefMut for FontAnalysisMut<'v, 'k> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.guard.get_mut(self.k).unwrap()
    }
}
*/

#[derive(Clone, Debug)]
pub struct FontAnalysis {
    width: f64,
    height: f64,
    ascent: f64,
    string: String,
    layout: pango::Layout,
    ascii_char_analyses: Box<[Option<CharAnalysis>]>,
    other_char_analyses: HashMap<char, CharAnalysis>,
}

impl FontAnalysis {
    const CACHE_TIMEOUT: usize = 30; // seconds

    pub fn create_for_widget<'a, D: Into<Option<&'a pango::FontDescription>> + 'a>(
        widget: &gtk::Widget,
        font_description: D,
    ) -> RcFontAnalysis {
        let context = widget.pango_context();
        let language = context.language().unwrap();

        let display = widget.display();
        let settings = gtk::Settings::for_display(&display);
        let fontconfig_timestamp = settings.gtk_fontconfig_timestamp();

        FontAnalysis::create_for_context(
            &context,
            font_description,
            &language,
            fontconfig_timestamp,
        )
        // FIXME: gtk4: this uses a per-widget context, while the gtk3 code uses a per-screen
        // one. That means there may be a lot less sharing and a lot more FontInfo's around?
    }

    fn create_for_context<'a, 'k, D: Into<Option<&'a pango::FontDescription>> + 'a>(
        context: &'k pango::Context,
        font_description: D,
        language: &pango::Language,
        fontconfig_timestamp: u32,
    ) -> RcFontAnalysis {
        let context =
            if let Some(true) = context.font_map().map(|fm| fm.is::<pangocairo::FontMap>()) {
                context
            } else {
                /* Ouch, Gtk+ switched over to some drawing system?
                 * Lets just create one from the default font map.
                 */
                PANGO_CONTEXT
                    .get_or_init(|| {
                        fragile::Fragile::new(
                            pangocairo::FontMap::default()
                                .unwrap()
                                .create_context()
                                .unwrap(),
                        )
                    })
                    .get()
            };

        // TODO: this function missing.
        // vte_pango_context_set_fontconfig_timestamp(context.get(), fontconfig_timestamp);

        context.set_base_dir(pango::Direction::Ltr);

        if let Some(desc) = font_description.into() {
            context.set_font_description(desc);
        }

        context.set_language(language);

        /* Make sure our contexts have a font_options set.  We use
         * this invariant in our context hash and equal functions.
         */
        // if (!pango_cairo_context_get_font_options(context.get())) {
        //         cairo_font_options_t *font_options;

        //         font_options = cairo_font_options_create ();
        //         pango_cairo_context_set_font_options(context.get(), font_options);
        //         cairo_font_options_destroy (font_options);
        // }
        let font_options = cairo::FontOptions::new().ok();
        pangocairo::context_set_font_options(context, font_options.as_ref());

        if let Some(cache) = FontAnalysisCaches::get(context) {
            log::debug!("found FontAnalysis in cache");
            cache
        } else {
            context.font_analysis()
        }
    }

    fn cache_ascii(&mut self) -> Option<()> {
        /* We have layout holding most ASCII characters.  We want to
         * cache as much info as we can about the ASCII letters so we don't
         * have to look them up again later */

        /* Don't cache if unknown glyphs found in layout */
        if self.layout.unknown_glyphs_count() > 0 {
            return None;
        }

        let language = self
            .layout
            .context()
            .and_then(|ctx| ctx.language())
            .unwrap_or_default();
        let latin_uses_default_language = language.includes_script(pango::Script::Latin);

        let line = self.layout.line_readonly(0)?;

        let runs = line.runs();

        /* Don't cache if more than one font used for the line */
        if runs.is_empty() {
            return None;
        }

        if runs.len() > 1 {
            return None;
        }

        let text = self.layout.text()?;

        let glyph_item = unsafe { runs.get_unchecked(0) };
        let glyph_string = glyph_item.glyph_string();
        let pango_font = glyph_item.item().analysis().font();
        let scaled_font = pangocairo::traits::FontExt::scaled_font(
            pango_font
                .downcast_ref::<pangocairo::Font>()
                .expect("Can not downcast pango::Font to pangocairo::Font"),
        )?;

        let mut iter = pango::GlyphItemIter::new_start(&glyph_item, &text).ok()?;

        loop {
            /* Only cache simple clusters */
            if iter.start_char() + 1 != iter.end_char()
                || iter.start_index() + 1 != iter.end_index()
                || iter.start_glyph() + 1 != iter.end_glyph()
            {
                continue;
            }

            let start_index = iter.start_index() as usize;
            let end_index = iter.end_index() as usize;
            let c = char::from_str(&text[start_index..end_index]).ok()?;
            let glyph = glyph_string
                .glyph_info()
                .get(iter.start_glyph() as usize)?
                .glyph();
            let geometry = glyph_string
                .glyph_info()
                .get(iter.start_glyph() as usize)?
                .geometry();

            /* If not using the default locale language, only cache non-common
             * characters as common characters get their font from their neighbors
             * and we don't want to force Latin on them. */
            if !latin_uses_default_language
                && unsafe { glib::ffi::g_unichar_get_script(c as u32) }
                    <= glib::ffi::G_UNICODE_SCRIPT_INHERITED
            {
                continue;
            }

            /* Only cache simple glyphs */
            if glyph > 0xFFFF || (geometry.x_offset() | geometry.y_offset()) != 0 {
                continue;
            }

            let width = (geometry.width() as f64 / pango::SCALE as f64).ceil() as usize;
            let ca = unsafe { self.ascii_char_analyses.get_unchecked_mut(c as usize) };
            if !matches!(
                ca.as_ref()
                    .map(|ca| ca.coverage())
                    .unwrap_or(&Coverage::Unknown),
                Coverage::Unknown
            ) {
                continue;
            }
            let coverage = Coverage::CairoGlyph(glyph, scaled_font.clone());
            ca.as_mut().map(|ca| {
                ca.set_width(width);
                ca.set_unknown_chars(0);
                ca.set_coverage(coverage);
            });
            ca.get_or_insert_with(|| CharAnalysis::new(width, 0.into()));

            if !iter.next_cluster() {
                break;
            }
            //if log::log_enabled!(log::Level::Trace) {
            //    self.coverage_count[0] +=1;
            //    self.coverage_count[(unsigned)uinfo->coverage()]+=1;
            //}
        }

        //log::trace!(
        //    "pangocairo: {:p} cached {} ASCII letters",
        //    self,
        //    self.coverage_count[0]
        //);
        Some(())
    }

    pub fn compute(&mut self) -> Option<()> {
        // Measure U+0021..U+007E individually instead of all together and then
        // averaging. For monospace fonts, the results should be the same, but
        // if the user (by design, or trough mis-configuration) uses a proportional
        // font, the latter method will greatly underestimate the required width,
        // leading to unreadable, overlapping characters.
        // https://gitlab.gnome.org/GNOME/vte/issues/138

        let mut max_width: f64 = 1.;
        let mut max_height: f64 = 1.;

        for c in 0x21u8..0x7f {
            // let c = c as char;
            self.layout
                .set_text(unsafe { std::str::from_utf8_unchecked(&[c]) });
            let (_, logical) = self.layout.extents();
            max_width = max_width.max((logical.width() as f64 / PANGO_SCALE).ceil());
            max_height = max_height.max((logical.height() as f64 / PANGO_SCALE).ceil());
        }
        /* Use the sample text to get the baseline */
        self.layout.set_text(SINGLE_WIDE_CHARACTERS);
        let (_, logical) = self.layout.extents();
        /* We don't do CEIL for width since we are averaging;
         * rounding is more accurate */
        self.ascent = (self.layout.baseline() as f64 / PANGO_SCALE).ceil();

        self.width = max_width;
        self.height = max_height;

        /* Now that we shaped the entire ASCII character string, cache glyph
         * info for them */
        self.cache_ascii()
    }

    fn append_(&self, c: char) {
        if (c as u32) < 0x80000001 {
            return;
        }
        if (c as u32) >= 0x80000000 {
            //
        }
        self.string
    }

    fn ensure_char_analysis(&mut self, c: char) -> &mut CharAnalysis {
        if (c as u32) < 128 {
            unsafe { self.ascii_char_analyses.get_unchecked_mut(c as usize) }
                .as_mut()
                .unwrap()
        } else {
            self.other_char_analyses
                .entry(c)
                .or_insert_with(|| CharAnalysis {
                    width: 0.,
                    coverage: Coverage::Unknown,
                    unknown_chars: None,
                })
        }
    }

    pub fn char_analysis(&mut self, c: char) -> &CharAnalysis {
        let ca = self.ensure_char_analysis(c);
        if !matches!(ca.coverage(), Coverage::Unknown) {
            return &*ca;
        }

        self.string.clear();
        // FIXME:
        // _vte_unistr_append_to_string(c, m_string);
        self.layout.set_text(&self.string);
        let (_, logical) = self.layout.extents();

        let width = (logical.width() as f64 / PANGO_SCALE).ceil();

        let line = self.layout.line_readonly(0);

        let unknown_chars = self.layout.unknown_glyphs_count() as usize;

        /* we use PangoLayoutRun rendering unless there is exactly one run in the line. */

        // if (G_UNLIKELY (!line || !line->runs || line->runs->next))
        if line.is_none()
            || line
                .as_ref()
                .map(|l| {
                    let runs = l.runs();
                    runs.is_empty() || runs.len() > 1
                })
                .unwrap()
        {
            ca.set_coverage(Coverage::PangoLayoutLine(line));

            /* we hold a manual reference on layout.  pango currently
             * doesn't work if line->layout is NULL.  ugh! */
            self.layout.set_text(""); /* make layout disassociate from the line */
        // ufi->using_pango_layout_line.line->layout = (PangoLayout *)g_object_ref(m_layout.get());
        } else {
            let line = line.unwrap();
            let glyph_items = line.runs();
            let glyph_item = glyph_items.first().unwrap();
            let pango_font = glyph_item.item().analysis().font();
            let glyph_string = glyph_item.glyph_string();

            /* we use fast cairo path if glyph string has only one real
             * glyph and at origin */
            if ca.unknown_chars.unwrap_or(0) == 0
                && glyph_string.num_glyphs() == 1
                && glyph_string.glyph_info()[0].glyph() <= 0xFFFF
                && (glyph_string.glyph_info()[0].geometry().x_offset()
                    | glyph_string.glyph_info()[0].geometry().y_offset())
                    == 0
            {
                let scaled_font = pangocairo::traits::FontExt::scaled_font(
                    pango_font.downcast_ref::<pangocairo::Font>().unwrap(),
                );

                if scaled_font.is_some() {
                    ca.set_coverage(Coverage::CairoGlyph(
                        glyph_string.glyph_info()[0].glyph(),
                        scaled_font.unwrap(),
                    ));
                }
            }

            /* use pango fast path otherwise */
            if matches!(ca.coverage(), Coverage::Unknown) {
                ca.set_coverage(Coverage::PangoGlyphString(pango_font, glyph_string));
            }
        }

        /* release internal layout resources */
        self.layout.set_text("");

        ca
    }

    pub fn width(&self) -> f64 {
        self.width
    }

    pub fn height(&self) -> f64 {
        self.height
    }

    pub fn ascent(&self) -> f64 {
        self.ascent
    }
}

pub trait ContextExt {
    fn font_analysis(&self) -> RcFontAnalysis;
}

impl ContextExt for pango::Context {
    fn font_analysis(&self) -> RcFontAnalysis {
        let ctx = self;
        let layout = pango::Layout::new(ctx);

        let mut tabs = pango::TabArray::new(1, false);
        tabs.set_tab(0, pango::TabAlign::Left, 1);
        layout.set_tabs(Some(&tabs));

        // let text = String::with_capacity(5);

        let mut fa = FontAnalysis {
            width: 1.,
            height: 1.,
            ascent: 0.,
            layout,
            string: "     ".to_string(),
            ascii_char_analyses: {
                let mut v = Vec::with_capacity(128);
                v.resize(128, None);
                v.into_boxed_slice()
            },
            other_char_analyses: HashMap::new(),
        };
        fa.compute();

        pango::version_check(1, 44, 0).expect("pango version 1.44 is required.");
        // NOTE: required pango >= 1.44
        /* Try using the font's metrics; see issue#163. */
        let metrics = match ctx.metrics(
            None, //use font from context
            None, // use language from context
        ) {
            Some(metrics) => metrics,
            None => {
                FontAnalysisCaches::set(ctx.clone(), Arc::new(fa));
                return FontAnalysisCaches::get(ctx).unwrap();
            }
        };

        /* Use provided metrics if possible */
        let ascent = (metrics.ascent() as f64 / PANGO_SCALE).ceil();
        let height = (metrics.height() as f64 / PANGO_SCALE).ceil();

        /* Note that we cannot use the font's width, since doing so
         * regresses issue#138 (non-monospaced font).
         * FIXME: Make sure the font is monospace before we get
         * here, and then use the font's width too.
         */
        // let width = (metrics.approximate_digit_width() as f64 / PANGO_SCALE).ceil();

        if ascent > 0. && height > fa.height {
            log::debug!("Using pango metrics",);
            fa.ascent = ascent;
            fa.height = height;
        } else if ascent <= 0. && height > 0. {
            log::debug!(
                "Disregarding pango metrics due to incorrect height {} < {}",
                height,
                fa.height
            );
        } else {
            log::debug!("Not using pango metrics due to not providing height or ascent");
        }

        log::debug!("font metrics = {}x{} ({})", fa.width, fa.height, fa.ascent,);

        FontAnalysisCaches::set(ctx.clone(), fa.into());
        FontAnalysisCaches::get(ctx).unwrap()
    }
}
