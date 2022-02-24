use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::{atomic, Arc, RwLock};

use glib::ObjectExt;
//use adw::prelude::*;
use gtk::gdk::prelude::FontMapExt;
use gtk::gdk::{self, ScrollDirection};
use gtk::prelude::{
    BoxExt, DrawingAreaExt, DrawingAreaExtManual, EditableExt, EditableExtManual,
    EventControllerExt, FrameExt, GtkWindowExt, IMContextExt, IMContextExtManual,
    IMMulticontextExt, OrientableExt, WidgetExt,
};
use once_cell::sync::{Lazy, OnceCell};
use pango::FontDescription;
use relm4::*;
use rustc_hash::FxHashMap;

use crate::bridge::{EditorMode, MessageKind, WindowAnchor};
use crate::components::{
    VimCmdPrompt, VimCmdPromptWidgets, VimNotifactions, VimNotifactionsWidgets,
};
use crate::cursor::{Cursor, CursorMode};
use crate::keys::ToInput;
use crate::vimview::{self, VimGrid};
use crate::{
    bridge::{self, RedrawEvent, UiCommand},
    metrics::Metrics,
    Opts,
};

#[allow(non_upper_case_globals)]
pub static GridActived: Lazy<Arc<atomic::AtomicU64>> =
    Lazy::new(|| Arc::new(atomic::AtomicU64::new(0)));

#[derive(Clone, Debug)]
pub enum AppMessage {
    UiCommand(UiCommand),
    RedrawEvent(RedrawEvent),
}

impl From<UiCommand> for AppMessage {
    fn from(cmd: UiCommand) -> Self {
        AppMessage::UiCommand(cmd)
    }
}

#[derive(Debug, Default)]
pub struct GridWindow {
    // window number
    winid: u64,
    // default grid
    //default_grid: u64,
}

pub struct AppModel {
    pub opts: Opts,
    pub title: String,
    pub default_width: i32,
    pub default_height: i32,

    pub guifont: Option<String>,
    pub guifontset: Option<String>,
    pub guifontwide: Option<String>,
    pub metrics: Rc<Cell<Metrics>>,
    pub show_tab_line: Option<u64>,

    pub font_description: Rc<RefCell<pango::FontDescription>>,
    pub font_changed: Rc<atomic::AtomicBool>,

    pub mode: EditorMode,

    pub mouse_on: Rc<atomic::AtomicBool>,
    pub cursor: Cell<Option<Cursor>>,
    pub cursor_at: Option<u64>,
    pub cursor_mode: usize,
    pub cursor_modes: Vec<CursorMode>,

    pub pctx: Rc<pango::Context>,
    pub gtksettings: OnceCell<gtk::Settings>,

    pub hldefs: Rc<RwLock<vimview::HighlightDefinitions>>,

    pub flush: Rc<atomic::AtomicBool>,
    pub focused: Rc<atomic::AtomicU64>,
    pub background_changed: Rc<atomic::AtomicBool>,

    pub vgrids: crate::factory::FactoryMap<vimview::VimGrid>,
    // relations about grid with window.
    pub relationships: FxHashMap<u64, GridWindow>,

    // pub floatwindows: crate::factory::FactoryMap<FloatWindow>,
    pub rt: tokio::runtime::Runtime,
}

impl AppModel {
    pub fn new(opts: Opts) -> AppModel {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_time()
            .enable_io()
            .build()
            .unwrap();
        let font_desc = FontDescription::from_string("monospace 11");
        AppModel {
            title: opts.title.clone(),
            default_width: opts.width,
            default_height: opts.height,
            guifont: None,
            guifontset: None,
            guifontwide: None,
            show_tab_line: None,

            mode: EditorMode::Normal,

            mouse_on: Rc::new(false.into()),
            cursor: Cell::new(Some(Cursor::new())),
            cursor_at: None,
            cursor_mode: 0,
            cursor_modes: Vec::new(),

            pctx: pangocairo::FontMap::default()
                .unwrap()
                .create_context()
                .map(|ctx| {
                    ctx.set_round_glyph_positions(true);
                    ctx.set_font_description(&font_desc);
                    ctx.set_base_dir(pango::Direction::Ltr);
                    ctx.set_language(&pango::Language::default());
                    let mut options = cairo::FontOptions::new().ok();
                    options.as_mut().map(|options| {
                        options.set_antialias(cairo::Antialias::Subpixel);
                        options.set_hint_style(cairo::HintStyle::Full);
                        // options.set_hint_metrics(cairo::HintMetrics::On);
                    });
                    pangocairo::context_set_font_options(&ctx, options.as_ref());
                    ctx
                })
                .unwrap()
                .into(),
            gtksettings: OnceCell::new(),

            metrics: Rc::new(Metrics::new().into()),
            font_description: Rc::new(RefCell::new(font_desc)),
            font_changed: Rc::new(false.into()),

            hldefs: Rc::new(RwLock::new(vimview::HighlightDefinitions::new())),

            flush: Rc::new(false.into()),
            focused: Rc::new(1.into()),
            background_changed: Rc::new(false.into()),

            vgrids: crate::factory::FactoryMap::new(),
            relationships: FxHashMap::default(),

            opts,

            rt,
        }
    }

    pub fn recompute(&self) {
        const PANGO_SCALE: f64 = pango::SCALE as f64;
        const SINGLE_WIDTH_CHARS: &'static str = concat!(
            " ! \" # $ % & ' ( ) * + , - . / ",
            "0 1 2 3 4 5 6 7 8 9 ",
            ": ; < = > ? @ ",
            "A B C D E F G H I J K L M N O P Q R S T U V W X Y Z ",
            "[ \\ ] ^ _ ` ",
            "a b c d e f g h i j k l m n o p q r s t u v w x y z ",
            "{ | } ~ ",
            ""
        );
        let desc = self.font_description.borrow_mut();
        log::error!(
            "----------------------> font desc {} {} {} {}",
            desc.family().unwrap(),
            desc.weight(),
            desc.style(),
            desc.size() / pango::SCALE,
        );
        // self.pctx.set_font_description(&desc);
        let layout = pango::Layout::new(&self.pctx);
        layout.set_font_description(Some(&desc));
        let mut tabs = pango::TabArray::new(1, false);
        tabs.set_tab(0, pango::TabAlign::Left, 1);
        layout.set_tabs(Some(&tabs));
        let mut max_width = 1;
        let mut max_height = 1;

        (0x21u8..0x7f).for_each(|c| {
            // char_
            let text = unsafe { String::from_utf8_unchecked(vec![c]) };
            layout.set_text(&text);
            let (_ink, logical) = layout.extents();
            max_height = logical.height().max(max_height);
            max_width = logical.width().max(max_width);
        });

        layout.set_text(SINGLE_WIDTH_CHARS);
        // let logical = layout.extents().1;
        let ascent = layout.baseline() as f64 / PANGO_SCALE;
        let font_metrics = self.pctx.metrics(Some(&desc), None).unwrap();
        let fm_width = font_metrics.approximate_digit_width();
        let fm_height = font_metrics.height();
        let fm_ascent = font_metrics.ascent();
        log::error!("font-metrics widht: {}", fm_width as f64 / PANGO_SCALE);
        log::error!("font-metrics height: {}", fm_height as f64 / PANGO_SCALE);
        log::error!("font-metrics ascent: {}", fm_ascent as f64 / PANGO_SCALE);
        let mut metrics = self.metrics.get();
        let charwidth = max_width as f64 / PANGO_SCALE;
        let width = charwidth;
        let charheight = if fm_height > 0 {
            fm_height.min(max_height) as f64 / PANGO_SCALE
        } else {
            max_height as f64 / PANGO_SCALE
        };
        // max_height as f64 / PANGO_SCALE;
        if metrics.charheight() == charheight
            && metrics.charwidth() == charwidth
            && metrics.width() == width
        {
            return;
        }
        metrics.set_width(width.ceil());
        metrics.set_ascent(ascent.ceil());
        metrics.set_charwidth(charwidth.ceil());
        metrics.set_charheight(charheight.ceil());
        log::error!("char-width {:?}", metrics.charwidth());
        log::error!("char-height {:?}", metrics.charheight());
        log::error!("char-ascent {:?}", metrics.ascent());
        self.metrics.replace(metrics);
    }
}

impl Model for AppModel {
    type Msg = AppMessage;
    type Widgets = AppWidgets;
    type Components = AppComponents;
}

impl AppUpdate for AppModel {
    fn update(
        &mut self,
        message: AppMessage,
        components: &AppComponents,
        _sender: Sender<AppMessage>,
    ) -> bool {
        // log::info!("message at AppModel::update {:?}", message);
        match message {
            AppMessage::UiCommand(ui_command) => {
                components
                    .messager
                    .sender()
                    .send(ui_command)
                    .expect("send failed");
            }
            AppMessage::RedrawEvent(event) => {
                match event {
                    RedrawEvent::SetTitle { title } => {
                        self.title = title
                            .split("     ")
                            .filter_map(|s| if s.is_empty() { None } else { Some(s.trim()) })
                            .collect::<Vec<_>>()
                            .join("  ")
                    }
                    RedrawEvent::OptionSet { gui_option } => match gui_option {
                        bridge::GuiOption::AmbiWidth(ambi_width) => {
                            log::debug!("unhandled ambi_width {}", ambi_width);
                        }
                        bridge::GuiOption::ArabicShape(arabic_shape) => {
                            log::debug!("unhandled arabic-shape: {}", arabic_shape);
                        }
                        bridge::GuiOption::Emoji(emoji) => {
                            log::debug!("emoji: {}", emoji);
                        }
                        bridge::GuiOption::GuiFont(guifont) => {
                            if !guifont.trim().is_empty() {
                                log::info!("gui font: {}", &guifont);
                                let desc = pango::FontDescription::from_string(
                                    &guifont.replace(":h", " "),
                                );

                                self.gtksettings.get().map(|settings| {
                                    settings.set_gtk_font_name(Some(&desc.to_str()));
                                });
                                self.pctx.set_font_description(&desc);

                                self.guifont.replace(guifont);
                                self.font_description.replace(desc);

                                self.recompute();
                                self.font_changed.store(true, atomic::Ordering::Relaxed);

                                self.vgrids
                                    .iter_mut()
                                    .for_each(|(_, vgrid)| vgrid.reset_cache());
                            }
                        }
                        bridge::GuiOption::GuiFontSet(guifontset) => {
                            self.guifontset.replace(guifontset);
                        }
                        bridge::GuiOption::GuiFontWide(guifontwide) => {
                            self.guifontwide.replace(guifontwide);
                        }
                        bridge::GuiOption::LineSpace(linespace) => {
                            log::info!("line space: {}", linespace);
                            let mut metrics = self.metrics.get();
                            metrics.set_linespace(linespace as _);
                            self.metrics.replace(metrics);
                        }
                        bridge::GuiOption::ShowTabLine(show_tab_line) => {
                            self.show_tab_line.replace(show_tab_line);
                        }
                        bridge::GuiOption::TermGuiColors(term_gui_colors) => {
                            log::debug!("unhandled term gui colors: {}", term_gui_colors);
                        }
                        bridge::GuiOption::Pumblend(pumblend) => {
                            log::debug!("unhandled pumblend: {}", pumblend)
                        }
                        bridge::GuiOption::Unknown(name, value) => {
                            log::debug!("GuiOption({}: {:?}) not supported yet.", name, value)
                        }
                    },
                    RedrawEvent::DefaultColorsSet { colors } => {
                        self.background_changed
                            .store(true, atomic::Ordering::Relaxed);
                        self.hldefs.write().unwrap().set_defaults(colors);
                    }
                    RedrawEvent::HighlightAttributesDefine { id, style } => {
                        self.hldefs.write().unwrap().set(id, style);
                    }
                    RedrawEvent::Clear { grid } => {
                        log::debug!("cleared grid {}", grid);
                        self.vgrids.get_mut(grid).map(|grid| grid.clear());
                    }
                    RedrawEvent::GridLine {
                        grid,
                        row,
                        column_start,
                        cells,
                    } => {
                        // log::info!("grid line {}", grid);
                        let winid = self
                            .relationships
                            .get(&grid)
                            .map(|rel| rel.winid)
                            .unwrap_or(0);
                        // let cells: Vec<_> = cells
                        //     .into_iter()
                        //     .map(|cell| vimview::TextCell {
                        //         text: cell.text,
                        //         hldef: cell.hldef,
                        //         repeat: cell.repeat,
                        //         double_width: cell.double_width,
                        //     })
                        //     .collect();

                        log::info!(
                            "grid line {}/{} - {} cells at {}x{}",
                            grid,
                            winid,
                            cells.len(),
                            row,
                            column_start
                        );

                        let grids: Vec<_> = self.vgrids.iter().map(|(k, _)| *k).collect();
                        self.vgrids
                            .get_mut(grid)
                            .expect(&format!(
                                "grid {} not found, valid grids {:?}",
                                grid, &grids
                            ))
                            .textbuf()
                            .borrow()
                            .set_cells(row as _, column_start as _, &cells);
                    }
                    RedrawEvent::Scroll {
                        grid,
                        top: _,
                        bottom: _,
                        left: _,
                        right: _,
                        rows,
                        columns,
                    } => {
                        let vgrid = self.vgrids.get_mut(grid).unwrap();
                        if rows.is_positive() {
                            vgrid.up(rows.abs() as _);
                        } else if rows.is_negative() {
                            //
                            vgrid.down(rows.abs() as _);
                        } else if columns.is_positive() {
                            unimplemented!("scroll left.");
                        } else if columns.is_negative() {
                            unimplemented!("scroll right.");
                        } else {
                            // rows and columns are both zero.
                            unimplemented!("why here.");
                        }
                    }
                    RedrawEvent::Resize {
                        grid,
                        width,
                        height,
                    } => {
                        self.focused.store(grid, atomic::Ordering::Relaxed);

                        let exists = self.vgrids.get(grid).is_some();
                        if exists {
                            self.vgrids
                                .get_mut(grid)
                                .unwrap()
                                .resize(width as _, height as _);
                        } else {
                            log::info!("Add grid {} to default window at left top.", grid);
                            let vgrid = VimGrid::new(
                                grid,
                                0,
                                (0., 0.).into(),
                                (width, height).into(),
                                self.flush.clone(),
                                self.hldefs.clone(),
                                self.metrics.clone(),
                                self.font_description.clone(),
                            );
                            vgrid.set_pango_context(self.pctx.clone());
                            self.vgrids.insert(grid, vgrid);
                            self.relationships.insert(grid, GridWindow { winid: 0 });
                        };
                    }

                    RedrawEvent::WindowPosition {
                        grid,
                        window,
                        start_row,
                        start_column,
                        width,
                        height,
                    } => {
                        let winid = self.rt.block_on(window.get_number()).unwrap();
                        log::info!("window pos number: {}", winid);
                        let winid = winid as u64;

                        self.focused.store(grid, atomic::Ordering::Relaxed);

                        let metrics = self.metrics.get();
                        let x = start_column as f64 * metrics.width();
                        let y = start_row as f64 * metrics.height(); //;

                        if self.vgrids.get(grid).is_none() {
                            // dose not exists, create
                            let vgrid = VimGrid::new(
                                grid,
                                winid,
                                (x.floor(), y.floor()).into(),
                                (width, height).into(),
                                self.flush.clone(),
                                self.hldefs.clone(),
                                self.metrics.clone(),
                                self.font_description.clone(),
                            );
                            vgrid.set_pango_context(self.pctx.clone());
                            self.vgrids.insert(grid, vgrid);
                            self.relationships.insert(grid, GridWindow { winid });
                            log::info!(
                                "Add grid {} to window {} at {}x{} with {}x{}.",
                                grid,
                                winid,
                                x,
                                y,
                                height,
                                width
                            );
                        } else {
                            let vgrid = self.vgrids.get_mut(grid).unwrap();
                            vgrid.resize(width as _, height as _);
                            vgrid.set_pos(x.floor(), y.floor());
                            log::info!(
                                "Move grid {} of window {} at {}x{} with {}x{}.",
                                grid,
                                winid,
                                x,
                                y,
                                height,
                                width
                            );
                            // make sure grid belongs right window.
                            self.relationships.get_mut(&grid).unwrap().winid = winid;
                            vgrid.show();
                        }

                        log::info!(
                            "window {} position grid {} row-start({}) col-start({}) width({}) height({})",
                            winid, grid, start_row, start_column, width, height,
                        );
                    }
                    RedrawEvent::WindowViewport {
                        grid,
                        window,
                        top_line,
                        bottom_line,
                        current_line,
                        current_column,
                        line_count,
                    } => {
                        let number = self.rt.block_on(window.get_number());
                        let winid = match number {
                            Ok(number) => number,
                            Err(err) => {
                                log::error!(
                                    "viewport grid {} dose not belongs any window: {:?}",
                                    grid,
                                    err
                                );
                                return true;
                            }
                        };

                        struct Rect {
                            x: f64,
                            y: f64,
                            width: usize,
                            height: usize,
                        }
                        type RectResult = Result<Rect, Box<nvim::error::CallError>>;
                        async fn window_rectangle(
                            window: &nvim::Window<crate::bridge::Tx>,
                        ) -> RectResult {
                            let (x, y) = window.get_position().await?;
                            let width = window.get_width().await?;
                            let height = window.get_height().await?;
                            Ok(Rect {
                                x: x as f64,
                                y: y as f64,
                                width: width as usize,
                                height: height as usize,
                            })
                        }

                        log::info!(
                            "window {} viewport grid {} viewport: top({}) bottom({}) highlight-line({}) highlight-column({}) with {} lines",
                             winid, grid, top_line, bottom_line, current_line, current_column, line_count,
                        );

                        let winid = winid as u64;

                        if self.vgrids.get(grid).is_none() {
                            // dose not exists, create
                            let rect: Rect = match self.rt.block_on(window_rectangle(&window)) {
                                Ok(rect) => rect,
                                Err(err) => {
                                    log::error!("vim window {} disappeared on handling WindowViewport event: {}", winid, err);
                                    return true;
                                }
                            };

                            let vgrid = VimGrid::new(
                                grid,
                                winid,
                                (rect.x, rect.y).into(),
                                (rect.width, rect.height).into(),
                                self.flush.clone(),
                                self.hldefs.clone(),
                                self.metrics.clone(),
                                self.font_description.clone(),
                            );
                            vgrid.set_pango_context(self.pctx.clone());
                            self.vgrids.insert(grid, vgrid);
                            self.relationships.insert(grid, GridWindow { winid });
                            log::info!(
                                "Add grid {} to window {} at {}x{}.",
                                grid,
                                winid,
                                rect.height,
                                rect.width
                            );
                        } else {
                            let vgrid = self.vgrids.get_mut(grid).unwrap();
                            // vgrid.resize(width as _, height as _);
                            // vgrid.set_pos(x, y);
                            vgrid.show();
                            // make sure grid belongs right window.
                            self.relationships.get_mut(&grid).unwrap().winid = winid;
                        }
                    }
                    RedrawEvent::WindowHide { grid } => {
                        self.focused
                            .compare_exchange(
                                grid,
                                1,
                                atomic::Ordering::Acquire,
                                atomic::Ordering::Relaxed,
                            )
                            .ok();
                        log::info!("hide {}", grid);
                        self.vgrids.get_mut(grid).unwrap().hide();
                    }
                    RedrawEvent::WindowClose { grid } => {
                        self.focused
                            .compare_exchange(
                                grid,
                                1,
                                atomic::Ordering::Acquire,
                                atomic::Ordering::Relaxed,
                            )
                            .ok();
                        log::info!("removing relations {}", grid);
                        self.relationships.remove(&grid);
                        self.vgrids.remove(grid);
                    }
                    RedrawEvent::Destroy { grid } => {
                        self.focused
                            .compare_exchange(
                                grid,
                                1,
                                atomic::Ordering::Acquire,
                                atomic::Ordering::Relaxed,
                            )
                            .ok();
                        log::info!("destroying relations {}", grid);
                        self.relationships.remove(&grid);
                        self.vgrids.remove(grid);
                    }
                    RedrawEvent::Flush => {
                        self.flush.store(true, atomic::Ordering::Relaxed);
                        self.vgrids.flush();
                    }
                    RedrawEvent::CursorGoto { grid, row, column } => {
                        let cursor_at = self.cursor_at.replace(grid);
                        if let Some(cursor_at) = cursor_at {
                            if cursor_at != grid {
                                let cursor = self.vgrids.get_mut(cursor_at).unwrap().take_cursor();
                                self.vgrids.get_mut(grid).unwrap().set_cursor(cursor);
                            }
                        } else {
                            self.vgrids
                                .get_mut(grid)
                                .map(|vgrid| vgrid.set_cursor(self.cursor.take().unwrap()));
                        };
                        self.vgrids
                            .get_mut(grid)
                            .map(|vgrid| vgrid.cursor_mut().set_pos(row, column));
                        // TODO: Add im_context.set_cursor_location
                    }
                    RedrawEvent::ModeInfoSet { cursor_modes } => {
                        self.cursor_modes = cursor_modes;

                        let mode = &self.cursor_modes[self.cursor_mode];
                        let style = self.hldefs.read().unwrap();
                        let cursor = if let Some(cursor_at) = self.cursor_at {
                            self.vgrids.get_mut(cursor_at).unwrap().cursor_mut()
                        } else {
                            self.cursor.get_mut().as_mut().unwrap()
                        };
                        cursor.change_mode(mode, &style);
                    }
                    RedrawEvent::ModeChange { mode, mode_index } => {
                        self.mode = mode;
                        self.cursor_mode = mode_index as _;
                        let cursor_mode = &self.cursor_modes[self.cursor_mode];
                        log::error!("Mode Change to {:?} {:?}", &self.mode, cursor_mode);
                        let style = self.hldefs.read().unwrap();
                        let cursor = if let Some(cursor_at) = self.cursor_at {
                            self.vgrids.get_mut(cursor_at).unwrap().cursor_mut()
                        } else {
                            self.cursor.get_mut().as_mut().unwrap()
                        };
                        cursor.change_mode(cursor_mode, &style);
                    }
                    RedrawEvent::BusyStart => {
                        log::debug!("Ignored BusyStart.");
                    }
                    RedrawEvent::BusyStop => {
                        log::debug!("Ignored BusyStop.");
                    }
                    RedrawEvent::MouseOn => {
                        self.mouse_on.store(true, atomic::Ordering::Relaxed);
                    }
                    RedrawEvent::MouseOff => {
                        self.mouse_on.store(false, atomic::Ordering::Relaxed);
                    }

                    RedrawEvent::MessageShow {
                        kind,
                        content,
                        replace_last,
                    } => {
                        log::error!("showing message {:?} {:?}", kind, content);
                    }
                    RedrawEvent::MessageShowMode { content } => {
                        log::error!("message show mode: {:?}", content);
                    }
                    RedrawEvent::MessageRuler { content } => {
                        log::error!("message ruler: {:?}", content);
                    }
                    RedrawEvent::MessageSetPosition {
                        grid,
                        row,
                        scrolled,
                        separator_character,
                    } => {
                        log::error!(
                            "message set position: {} {} {} '{}'",
                            grid,
                            row,
                            scrolled,
                            separator_character
                        );
                    }
                    RedrawEvent::MessageShowCommand { content } => {
                        log::error!("message show command: {:?}", content);
                    }
                    RedrawEvent::MessageHistoryShow { entries } => {
                        log::error!("message history: {:?}", entries);
                    }
                    RedrawEvent::MessageClear => {
                        log::error!("message clear all");
                    }

                    RedrawEvent::WindowFloatPosition {
                        grid,
                        anchor,
                        anchor_grid,
                        anchor_row,
                        anchor_column,
                        focusable,
                        sort_order: _,
                    } => {
                        log::debug!(
                            "grid {} is float window exists in vgrids {} anchor {} {:?} pos {}x{} focusable {}",
                            grid,
                            self.vgrids.get(grid).is_some(),
                            anchor_grid,
                            anchor,
                            anchor_column,
                            anchor_row,
                            focusable
                        );
                        let basepos = self.vgrids.get(anchor_grid).unwrap().pos();
                        let (left, top) = (basepos.x, basepos.y);

                        let vgrid = self.vgrids.get_mut(grid).unwrap();

                        let (col, row) = match anchor {
                            WindowAnchor::NorthWest => (anchor_column, anchor_row),
                            WindowAnchor::NorthEast => {
                                (anchor_column - vgrid.width() as f64, anchor_row)
                            }
                            WindowAnchor::SouthWest => {
                                (anchor_column, anchor_row - vgrid.height() as f64)
                            }
                            WindowAnchor::SouthEast => (
                                anchor_column - vgrid.width() as f64,
                                anchor_row - vgrid.height() as f64,
                            ),
                        };

                        let metrics = self.metrics.get();
                        let x = col * metrics.width();
                        let y = row * metrics.height();
                        log::info!("moving float window {} to {}x{}", grid, col, row);
                        vgrid.set_pos(left + x, top + y);
                        vgrid.set_is_float(true);
                        vgrid.set_focusable(focusable);
                    }
                    _ => {
                        log::error!("Unhandled RedrawEvent {:?}", event);
                    }
                }
            }
        }
        true
    }
}

#[derive(relm4::Components)]
pub struct AppComponents {
    messager: relm4::RelmMsgHandler<crate::messager::VimMessager, AppModel>,
    messages: RelmComponent<VimNotifactions, AppModel>,
    cmd_prompt: RelmComponent<VimCmdPrompt, AppModel>,
}

#[relm_macros::widget(pub)]
impl Widgets<AppModel, ()> for AppWidgets {
    view! {
        main_window = gtk::ApplicationWindow {
            set_default_width: model.default_width,
            set_default_height: model.default_height,
            set_title: watch!(Some(&model.title)),
            set_child: vbox = Some(&gtk::Box) {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 0,
                set_hexpand: true,
                set_vexpand: true,
                set_focusable: true,
                set_sensitive: true,
                set_can_focus: true,
                set_can_target: true,
                set_focus_on_click: true,

                // set_child: Add tabline

                append: overlay = &gtk::Overlay {
                    set_focusable: true,
                    set_sensitive: true,
                    set_can_focus: true,
                    set_can_target: true,
                    set_focus_on_click: true,
                    set_child: da = Some(&gtk::DrawingArea) {
                        set_hexpand: true,
                        set_vexpand: true,
                        set_focus_on_click: false,
                        set_overflow: gtk::Overflow::Hidden,
                        connect_resize[sender = sender.clone(), metrics = model.metrics.clone()] => move |da, width, height| {
                            log::info!("da resizing width: {}, height: {}", width, height);
                            let metrics = metrics.get();
                            let rows = da.height() as f64 / metrics.height(); //  + metrics.linespace
                            let cols = da.width() as f64 / metrics.width();
                            log::info!("da resizing rows: {} cols: {}", rows, cols);
                            sender
                                .send(
                                    UiCommand::Resize {
                                        width: cols as _,
                                        height: rows as _,
                                    }
                                    .into(),
                                )
                                .unwrap();
                        },
                        set_draw_func[hldefs = model.hldefs.clone()] => move |da, cr, _, _| {
                            if let Some(background) = hldefs.read().unwrap().defaults().and_then(|defaults| defaults.background) {
                                //cr.rectangle(0., 0., da.width() as _, da.height() as _);
                                //cr.set_source_rgba(
                                //    background.red() as _,
                                //    background.green() as _,
                                //    background.blue() as _,
                                //    1.,
                                //);
                                //cr.paint().unwrap();
                            }
                        }
                    },
                    add_overlay: grids_container = &gtk::Fixed {
                        set_widget_name: "grids-container",
                        set_visible: true,
                        set_focus_on_click: true,
                        factory!(model.vgrids),
                    },
                    add_overlay: float_win_container = &gtk::Fixed {
                        set_widget_name: "float-win-container",
                        set_visible: false,
                        set_hexpand: false,
                        set_vexpand: false,
                    },
                    add_overlay: cursor_drawing_area = &gtk::DrawingArea {
                        set_widget_name: "cursor-drawing-area",
                        set_visible: false,
                        set_hexpand: false,
                        set_vexpand: false,
                        set_focus_on_click: false,
                    },
                    add_overlay: messages_container = &gtk::Frame {
                        set_widget_name: "messages-container",
                        set_visible: false,
                        set_hexpand: false,
                        set_vexpand: false,
                        set_child: Some(components.messages.root_widget()),
                    },
                    add_overlay: commnad_prompt = &gtk::Frame {
                        set_visible: false,
                        set_child: Some(components.cmd_prompt.root_widget()),
                    }
                }
            },
            connect_close_request[sender = sender.clone()] => move |_| {
                sender.send(AppMessage::UiCommand(UiCommand::Quit)).ok();
                gtk::Inhibit(false)
            },
        }
    }

    fn post_init() {
        model.gtksettings.set(overlay.settings()).ok();
        model.recompute();

        let im_context = gtk::IMMulticontext::new();
        im_context.set_use_preedit(false);
        im_context.set_client_widget(Some(&overlay));
        im_context.set_input_purpose(gtk::InputPurpose::Terminal);
        im_context.set_cursor_location(&gdk::Rectangle::new(0, 0, 5, 10));
        im_context.connect_preedit_start(|_| {
            log::debug!("preedit started.");
        });
        im_context.connect_preedit_end(|im_context| {
            log::debug!("preedit done, '{}'", im_context.preedit_string().0);
        });
        im_context.connect_preedit_changed(|im_context| {
            log::debug!("preedit changed, '{}'", im_context.preedit_string().0);
        });
        im_context.connect_commit(glib::clone!(@strong sender => move |ctx, text| {
            log::debug!("im-context({}) commit '{}'", ctx.context_id(), text);
            sender
                .send(UiCommand::Keyboard(text.replace("<", "<lt>").into()).into())
                .unwrap();
        }));

        main_window.set_focus_widget(Some(&overlay));
        main_window.set_default_widget(Some(&overlay));

        let listener = gtk::EventControllerScroll::builder()
            .flags(gtk::EventControllerScrollFlags::all())
            .name("vimview-scrolling-listener")
            .build();
        listener.connect_scroll(glib::clone!(@strong sender, @strong model.mouse_on as mouse_on, @strong grids_container => move |c, x, y| {
            if !mouse_on.load(atomic::Ordering::Relaxed) {
                return gtk::Inhibit(false)
            }
            let event = c.current_event().unwrap().downcast::<gdk::ScrollEvent>().unwrap();
            // let (x, y) = event.position().unwrap();
            // let vgrid = grids_container.first_child().unwrap();
            // let mut id = if vgrid.is_visible() && vgrid.contains(x, y) {
            //     vgrid.property::<u64>("id")
            // } else {
            //     1
            // };
            // while let Some(widget) = vgrid.next_sibling() {
            //     if widget.is_visible() && widget.contains(x, y) {
            //         id = widget.property::<u64>("id");
            //         break;
            //     }
            // }
            let id = GridActived.load(atomic::Ordering::Relaxed);
            let direction = match event.direction() {
                ScrollDirection::Up => {
                    "up"
                },
                    ScrollDirection::Down => {
                    "down"
                }
                ScrollDirection::Left => {
                    "left"
                }
                ScrollDirection::Right => {
                    "right"
                }
                _ => {
                    return gtk::Inhibit(false)
                }
            };
            log::error!("scrolling grid {} x: {}, y: {} {}", id, x, y, &direction);
            let command = UiCommand::Scroll { direction: direction.into(), grid_id: id, position: (0, 1) };
            sender.send(AppMessage::UiCommand(command)).unwrap();
            gtk::Inhibit(false)
        }));
        listener.connect_decelerate(|_c, _vel_x, _vel_y| {
            // let id: u64 = c
            //     .widget()
            //     .dynamic_cast_ref::<VimGridView>()
            //     .unwrap()
            //     .property("id");
            // log::error!("scrolling decelerate grid {} x:{} y:{}.", id, vel_x, vel_y);
        });
        listener.connect_scroll_begin(|_c| {
            // let id: u64 = c
            //     .widget()
            //     .dynamic_cast_ref::<VimGridView>()
            //     .unwrap()
            //     .property("id");
            // log::error!("scrolling begin grid {}.", id);
        });
        listener.connect_scroll_end(|_c| {
            // let id: u64 = c
            //     .widget()
            //     .dynamic_cast_ref::<VimGridView>()
            //     .unwrap()
            //     .property("id");
            // log::error!("scrolling end grid {}.", id);
        });
        main_window.add_controller(&listener);

        let focus_controller = gtk::EventControllerFocus::builder()
            .name("vimview-focus-controller")
            .build();
        focus_controller.connect_enter(
            glib::clone!(@strong sender, @strong im_context => move |_| {
                log::error!("FocusGained");
                im_context.focus_in();
                sender.send(UiCommand::FocusGained.into()).unwrap();
            }),
        );
        focus_controller.connect_leave(
            glib::clone!(@strong sender, @strong im_context  => move |_| {
                log::error!("FocusLost");
                im_context.focus_out();
                sender.send(UiCommand::FocusLost.into()).unwrap();
            }),
        );
        main_window.add_controller(&focus_controller);

        let key_controller = gtk::EventControllerKey::builder()
            .name("vimview-key-controller")
            .build();
        key_controller.set_im_context(&im_context);
        key_controller.connect_key_pressed(
            glib::clone!(@strong sender => move |c, keyval, _keycode, modifier| {
                let event = c.current_event().unwrap();

                if c.im_context().filter_keypress(&event) {
                    log::debug!("keypress handled by im-context.");
                    return gtk::Inhibit(true)
                }
                let keypress = (keyval, modifier);
                if let Some(keypress) = keypress.to_input() {
                    log::debug!("keypress {} sent to neovim.", keypress);
                    sender.send(UiCommand::Keyboard(keypress.into_owned()).into()).unwrap();
                    gtk::Inhibit(true)
                } else {
                    log::debug!("keypress ignored: {:?}", keyval.name());
                    gtk::Inhibit(false)
                }
            }),
        );
        main_window.add_controller(&key_controller);
    }

    fn pre_view() {
        if let Ok(true) = model.background_changed.compare_exchange(
            true,
            false,
            atomic::Ordering::Acquire,
            atomic::Ordering::Relaxed,
        ) {
            self.da.queue_draw();
        }
        if let Ok(true) = model.font_changed.compare_exchange(
            true,
            false,
            atomic::Ordering::Acquire,
            atomic::Ordering::Relaxed,
        ) {
            log::info!(
                "default font name: {}",
                model.font_description.borrow().to_str()
            );
            let metrics = model.metrics.get();
            let rows = self.da.height() as f64 / metrics.height();
            let cols = self.da.width() as f64 / metrics.width();
            log::debug!(
                "trying to resize to {}x{} original {}x{} {:?}",
                rows,
                cols,
                self.da.width(),
                self.da.height(),
                metrics
            );
            sender
                .send(
                    UiCommand::Resize {
                        width: cols as _,
                        height: rows as _,
                    }
                    .into(),
                )
                .unwrap();
        }
        // TODO:
        // self.im_context.set_cursor_location(&gdk::Rectangle::new(0, 0, 5, 10));
    }
}
