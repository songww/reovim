use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::{atomic, RwLock};

use gtk::prelude::{
    BoxExt, DrawingAreaExt, DrawingAreaExtManual, EventControllerExt, GtkWindowExt, IMContextExt,
    IMContextExtManual, IMMulticontextExt, OrientableExt, WidgetExt,
};
use once_cell::sync::OnceCell;
use pango::FontDescription;
use relm4::{AppUpdate, Model, Sender, Widgets};
use rustc_hash::FxHashMap;

use crate::bridge::{EditorMode, MessageKind, WindowAnchor};
use crate::cursor::{Cursor, CursorMode};
use crate::keys::ToInput;
use crate::vimview::{self, VimGrid};
use crate::{
    bridge::{self, RedrawEvent, UiCommand},
    metrics::Metrics,
    Opts,
};

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

pub struct FloatWindow {
    id: u64,
    anchor: WindowAnchor,
    anchor_grid: u64,
    anchor_row: f64,
    anchor_col: f64,
    focusable: bool,
    rank: Option<u64>,
}

pub struct Message {
    kind: MessageKind,
    content: Vec<(u64, String)>,
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

    pub pctx: OnceCell<Rc<pango::Context>>,
    pub gtksettings: OnceCell<gtk::Settings>,

    pub hldefs: Rc<RwLock<vimview::HighlightDefinitions>>,

    pub flush: Rc<atomic::AtomicBool>,
    pub focused: Rc<atomic::AtomicU64>,
    pub background_changed: Rc<atomic::AtomicBool>,

    pub vgrids: crate::factory::FactoryMap<vimview::VimGrid>,
    // relations about grid with window.
    pub relationships: FxHashMap<u64, GridWindow>,

    // pub floatwindows: crate::factory::FactoryMap<FloatWindow>,
    pub messages: Vec<Message>,

    pub rt: tokio::runtime::Runtime,
}

impl AppModel {
    pub fn new(opts: Opts) -> AppModel {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_time()
            .enable_io()
            .build()
            .unwrap();
        AppModel {
            title: opts.title.clone(),
            default_width: opts.width,
            default_height: opts.height,
            guifont: None,
            guifontset: None,
            guifontwide: None,
            show_tab_line: None,

            metrics: Rc::new(Metrics::new().into()),
            font_description: Rc::new(RefCell::new(FontDescription::from_string("monospace 11"))),
            font_changed: Rc::new(false.into()),

            mode: EditorMode::Normal,

            mouse_on: Rc::new(false.into()),
            cursor: Cell::new(Some(Cursor::new())),
            cursor_at: None,
            cursor_mode: 0,
            cursor_modes: Vec::new(),

            pctx: OnceCell::new(),
            gtksettings: OnceCell::new(),

            hldefs: Rc::new(RwLock::new(vimview::HighlightDefinitions::new())),

            flush: Rc::new(false.into()),
            focused: Rc::new(1.into()),
            background_changed: Rc::new(false.into()),

            vgrids: crate::factory::FactoryMap::new(),
            relationships: FxHashMap::default(),

            // floatwindows: crate::factory::FactoryMap::new(),
            messages: Vec::new(),

            opts,

            rt,
        }
    }

    pub fn recompute(&self) {
        const SINGLE_WIDTH_CHARS: &'static str = concat!(
            "ABC D E F G H I J K L M N O P Q R S T U V W X Y Z ",
            "! \" # $ % & ' ( ) * + , - . / ",
            "0 1 2 3 4 5 6 7 8 9 ",
            ": ; < = > ? @ ",
            "[ \\ ] ^ _ ` ",
            "a b c d e f g h i j k l m n o p q r s t u v w x y z ",
            "{ | } ~ ",
        );
        let desc = self.font_description.borrow_mut();
        // desc.set_weight(pango::Weight::Light);
        // desc.set_stretch(pango::Stretch::Condensed);
        log::error!(
            "----------------------> font desc {} {} {} {}",
            desc.family().unwrap(),
            desc.weight(),
            desc.style(),
            desc.size() / pango::SCALE,
        );
        let pctx = self.pctx.get().unwrap();
        pctx.set_font_description(&desc);
        let layout = pango::Layout::new(pctx);
        let font_metrics = pctx.metrics(Some(&desc), None).unwrap();
        layout.set_text(SINGLE_WIDTH_CHARS);
        let layoutline = layout.line_readonly(0).unwrap();
        let charheight = layoutline.height();
        // let charwidth1 = layoutline.index_to_x(0, false);
        // let charwidth2 = layoutline.index_to_x(1, false);
        // let charwidth3 = layoutline.index_to_x(2, false);
        // log::error!("charwidth {} {} {}", charwidth1, charwidth2, charwidth3);
        let item = pango::itemize(&pctx, "A", 0, 1, &pango::AttrList::new(), None)
            .pop()
            .unwrap();
        let mut glyphs = pango::GlyphString::new();
        pango::shape("A", item.analysis(), &mut glyphs);
        let (_, rect) = glyphs.extents(&item.analysis().font());
        let charwidth = rect.width() as f64 / pango::SCALE as f64;
        let mut metrics = self.metrics.get();
        let charheight = charheight as f64 / pango::SCALE as f64;
        let width = font_metrics.approximate_digit_width() as f64 / pango::SCALE as f64;
        if metrics.charheight() == charheight
            && metrics.charwidth() == charwidth
            && metrics.width() == width
        {
            return;
        }
        metrics.set_width(width);
        metrics.set_charwidth(charwidth);
        metrics.set_charheight(charheight);
        log::error!("char-height {:?}", metrics.charheight());
        log::error!("char-width {:?}", metrics.charwidth());
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
                // sender.send(ui_command).unwrap();
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
                                // desc.set_stretch(pango::Stretch::ExtraExpanded);

                                self.gtksettings.get().map(|settings| {
                                    settings.set_gtk_font_name(Some(&desc.to_str()));
                                });

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
                            //
                        } else if columns.is_negative() {
                            //
                        } else {
                            // rows and columns are both zero.
                            unimplemented!("Should not here.");
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
                            vgrid.set_pango_context({
                                let mut pctx = pango::Context::new();
                                pctx.clone_from(&self.pctx.get().unwrap());
                                pctx
                            });
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
                            vgrid.set_pango_context({
                                let mut pctx = pango::Context::new();
                                pctx.clone_from(&self.pctx.get().unwrap());
                                pctx
                            });
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
                            vgrid.set_pango_context({
                                let mut pctx = pango::Context::new();
                                pctx.clone_from(&self.pctx.get().unwrap());
                                pctx
                            });
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
                        if replace_last {
                            if let Some(last) = self.messages.last_mut() {
                                *last = Message { kind, content }
                            } else {
                                self.messages.push(Message { kind, content })
                            }
                        }
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
                        self.messages.clear();
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
                                cr.rectangle(0., 0., da.width() as _, da.height() as _);
                                cr.set_source_rgba(
                                    background.red() as _,
                                    background.green() as _,
                                    background.blue() as _,
                                    1.,
                                );
                                cr.paint().unwrap();
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
                    add_overlay: messages_container = &gtk::Grid {
                        set_widget_name: "messages-container",
                        set_visible: false,
                        set_hexpand: false,
                        set_vexpand: false,
                    },
                }
            },
            connect_close_request[sender = sender.clone()] => move |_| {
                sender.send(AppMessage::UiCommand(UiCommand::Quit)).ok();
                gtk::Inhibit(false)
            },
        }
    }

    fn post_init() {
        model.pctx.set(vbox.pango_context().into()).ok();
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

        let mut options = cairo::FontOptions::new().ok();
        options.as_mut().map(|options| {
            options.set_antialias(cairo::Antialias::Subpixel);
            options.set_hint_style(cairo::HintStyle::Default);
            // options.set_hint_metrics(cairo::HintMetrics::On);
        });
        main_window.set_font_options(options.as_ref());

        let listener = gtk::EventControllerScroll::builder()
            .flags(gtk::EventControllerScrollFlags::all())
            .name("vimview-scrolling-listener")
            .build();
        listener.connect_scroll(glib::clone!(@strong sender, @strong model.mouse_on as mouse_on => move |c, x, y| {
            if !mouse_on.load(atomic::Ordering::Relaxed) {
                return gtk::Inhibit(false)
            }
            // FIXME: get grid id by neovim current buf.
            let id = 1;
            let direction = c.current_event().unwrap().downcast::<gdk::ScrollEvent>().unwrap().direction().to_string().to_lowercase();
            let command = UiCommand::Scroll { direction: direction.into(), grid_id: id, position: (0, 1) };
            sender.send(AppMessage::UiCommand(command)).unwrap();
            log::error!("scrolling grid {} x: {}, y: {}", id, x, y);
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
        overlay.add_controller(&listener);

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
        overlay.add_controller(&key_controller);
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
