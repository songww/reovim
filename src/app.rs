use std::cell::{Cell, RefCell};
use std::sync::RwLock;
use std::{rc::Rc, sync::atomic};

use gdk::prelude::FontFamilyExt;
use gtk::prelude::{
    BoxExt, DrawingAreaExt, GtkWindowExt, OrientableExt, WidgetExt, WidgetExtManual,
};
use once_cell::sync::OnceCell;
use pango::FontDescription;
use relm4::factory::positions::FixedPosition;
use relm4::{send, AppUpdate, Model, RelmApp, Sender, WidgetPlus, Widgets};
use rustc_hash::FxHashMap;

use crate::vimview;
use crate::{
    bridge::{self, RedrawEvent, UiCommand},
    style, Opts,
};

// 最下层的grid
const DEFAULT_WINDOW: u64 = 0;

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
pub struct Relation {
    id: u64,
    // default grid
    default_grid: u64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct FontMetrics {
    charwidth: f64,

    linespace: f64,
    lineheight: f64,
}

impl FontMetrics {
    fn new() -> FontMetrics {
        FontMetrics {
            charwidth: 0.,

            linespace: 0.,
            lineheight: 0.,
        }
    }
}

pub struct AppModel {
    pub opts: Opts,
    pub title: String,
    pub default_width: i32,
    pub default_height: i32,

    pub guifont: Option<String>,
    pub guifontset: Option<String>,
    pub guifontwide: Option<String>,
    pub font_metrics: Rc<Cell<FontMetrics>>,
    pub show_tab_line: Option<u64>,

    pub font_description: Rc<RefCell<pango::FontDescription>>,

    pub pctx: OnceCell<Rc<pango::Context>>,

    pub hldefs: Rc<RwLock<vimview::HighlightDefinitions>>,

    pub vwindows: crate::factory::FactoryMap<vimview::VimWindow>,
    // relations about grid and window.
    pub relationships: FxHashMap<u64, Relation>,

    pub rt: tokio::runtime::Runtime,
}

impl AppModel {
    pub fn new(opts: Opts) -> AppModel {
        AppModel {
            title: opts.title.clone(),
            default_width: opts.width,
            default_height: opts.height,
            guifont: None,
            guifontset: None,
            guifontwide: None,
            show_tab_line: None,

            font_metrics: Rc::new(FontMetrics::new().into()),

            font_description: Rc::new(RefCell::new(FontDescription::from_string("monospace 17"))),

            pctx: OnceCell::new(),

            hldefs: Rc::new(RwLock::new(vimview::HighlightDefinitions::new())),

            vwindows: crate::factory::FactoryMap::new(),
            relationships: FxHashMap::default(),

            opts,

            rt: tokio::runtime::Builder::new_multi_thread()
                .enable_time()
                .enable_io()
                .build()
                .unwrap(),
        }
    }

    /// (width, height)
    fn size_required(&self, cols: u64, rows: u64) -> (u64, u64) {
        let factors = self.font_metrics.get();
        (
            (cols as f64 * factors.charwidth) as u64,
            (rows as f64 * factors.lineheight) as u64,
        )
    }

    pub fn compute(&self) {
        // const SINGLE_WIDTH_CHARS: &'static str = concat!(
        //     "! \" # $ % & ' ( ) * + , - . / ",
        //     "0 1 2 3 4 5 6 7 8 9 ",
        //     ": ; < = > ? @ ",
        //     "A B C D E F G H I J K L M N O P Q R S T U V W X Y Z ",
        //     "[ \\ ] ^ _ ` ",
        //     "a b c d e f g h i j k l m n o p q r s t u v w x y z ",
        //     "{ | } ~ ",
        // );
        // let pctx = pango::Context::new();
        // if let Some(ref desc) = self.font_description {
        //     log::warn!(" --> update font desc {}", desc.to_str());
        //     pctx.set_font_description(desc);
        // }
        let desc = unsafe { &*self.font_description.as_ref().as_ptr() };
        log::debug!(
            "font desc {} {} {} {}pt",
            desc.family().unwrap(),
            desc.weight(),
            desc.style(),
            desc.size() / pango::SCALE,
        );
        self.pctx
            .get()
            .unwrap()
            .set_font_description(unsafe { &*self.font_description.as_ref().as_ptr() });
        // let layout = pango::Layout::new(&self.pctx.get().unwrap());
        // layout.set_font_description(Some(unsafe { &*self.font_description.as_ref().as_ptr() }));
        // layout.set_text(SINGLE_WIDTH_CHARS);
        // log::warn!("--> size {:?}", layout.size());
        // log::warn!("--> extents {:?}", layout.extents());
        // log::warn!("--> pixel size {:?}", layout.pixel_size());
        // log::warn!("--> pixel extents {:?}", layout.pixel_extents());
        // let (_, h) = layout.size();
        // log::warn!("--> pos {:?}", layout.index_to_pos(0));
        // log::warn!("--> x {:?}", layout.index_to_line_x(0, true));
        // let pos = layout.index_to_pos(0);
        // self.lineheight = pos.height() as f64 / pango::SCALE as f64;
        // if let Some(linespace) = self.linespace {
        //     self.lineheight += linespace as f64;
        // }
        // self.charwidth = pos.width() as f64 / pango::SCALE as f64;
        let metrics = self
            .pctx
            .get()
            .unwrap()
            .metrics(
                Some(unsafe { &*self.font_description.as_ref().as_ptr() }),
                None,
            )
            .unwrap();
        let mut font_metrics = self.font_metrics.get();
        font_metrics.lineheight = metrics.height() as f64 / pango::SCALE as f64;
        font_metrics.charwidth = metrics.approximate_digit_width() as f64 / pango::SCALE as f64;
        log::info!("line-height {:?}", font_metrics.lineheight);
        log::info!("char-width {:?}", font_metrics.charwidth);
        self.font_metrics.replace(font_metrics);
        /*
        let s = unsafe { String::from_utf8_unchecked(vec!['1' as u8; cols]) };
        let text = vec![s; rows].join("\n");
        let layout = pango::Layout::new(unsafe { self.pctx.get_unchecked() });
        layout.set_markup(&text);
        log::info!(
            "font desc {}",
            self.pctx
                .get()
                .unwrap()
                .font_description()
                .unwrap()
                .to_str()
        );
        log::info!("line height {}", layout.line(0).unwrap().height());
        log::info!("baseline {}", layout.baseline());
        log::info!("line count {}", layout.line_count());
        log::info!("size {:?}", layout.size());
        log::info!("extents {:?}", layout.extents());
        log::info!("pixel size {:?}", layout.pixel_size());
        log::info!(
            "last size {} {:?}",
            text.len() as i32 - 1,
            layout.index_to_pos(1)
        );
        log::info!("last size {:?}", layout.index_to_pos(text.len() as i32));
        // layout.pixel_size()
        (cols as _, rows as _)
        */
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
        sender: Sender<AppMessage>,
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
                                log::warn!("gui font: {}", &guifont);
                                let desc = pango::FontDescription::from_string(
                                    &guifont.replace(":h", " "),
                                );

                                // self.pctx.get().unwrap().set_font_description(&desc);
                                // unsafe { self.pctx.get_unchecked() }.set_font_description(&desc);

                                self.guifont.replace(guifont);
                                self.font_description.replace(desc);

                                self.compute();
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
                            let mut font_metrics = self.font_metrics.get();
                            font_metrics.linespace = linespace as _;
                            self.font_metrics.replace(font_metrics);
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
                        self.hldefs.write().unwrap().set_defaults(colors);
                    }
                    RedrawEvent::HighlightAttributesDefine { id, style } => {
                        self.hldefs.write().unwrap().set(id, style);
                    }
                    RedrawEvent::Clear { grid } => {
                        log::info!("clearing {}", grid);
                        if let Some(win) = self.relationships.get(&grid) {
                            let window = self.vwindows.get_mut(win.id).unwrap();
                            //if win.is_base {
                            //    log::warn!("clearing base {}", grid);
                            //    window.clear();
                            //    // unimplemented!("clearing base grid {}.", grid);
                            //} else {
                            log::warn!("cleared {}", grid);
                            window.get_mut(grid).unwrap().textbuf().borrow().clear();
                            //}
                        };
                    }
                    RedrawEvent::GridLine {
                        grid,
                        row,
                        column_start,
                        cells,
                    } => {
                        // log::info!("grid line {}", grid);
                        if let Some(win) = self.relationships.get(&grid) {
                            let cells: Vec<_> = cells
                                .into_iter()
                                .map(|cell| vimview::TextCell {
                                    text: cell.text,
                                    hldef: cell.hldef,
                                    repeat: cell.repeat,
                                    double_width: cell.double_width,
                                })
                                .collect();

                            log::info!(
                                "grid line {}/{} - {} cells at {}x{}",
                                grid,
                                win.id,
                                cells.len(),
                                row,
                                column_start
                            );

                            let relations = self
                                .vwindows
                                .iter()
                                .map(|(k, win)| win.grids().iter().map(|(grid, _)| (*grid, *k)))
                                .flatten()
                                .collect::<Vec<_>>();

                            self.vwindows
                                .get_mut(win.id)
                                .unwrap()
                                .get_mut(grid)
                                .expect(&format!("grid {} dose not belongs window {}, somethings wrong: {:?}\n{:?}", grid, win.id, &self.relationships, &relations))
                                .textbuf()
                                .borrow()
                                .set_cells(row as _, column_start as _, &cells);
                        } else {
                            log::error!("grid {} dose not exists.", grid);
                        };
                    }
                    RedrawEvent::Resize {
                        grid,
                        width,
                        height,
                    } => {
                        let rect = gdk::Rectangle::new(0, 0, width as _, height as _);
                        if !self.relationships.contains_key(&grid) {
                            let default_grid = if let Some(win) = self.vwindows.get(DEFAULT_WINDOW)
                            {
                                win.grid
                            } else {
                                let pos = FixedPosition { x: 0., y: 0. };
                                let size = (width, height);
                                let win = vimview::VimWindow::new(
                                    DEFAULT_WINDOW,
                                    grid,
                                    pos,
                                    size,
                                    self.hldefs.clone(),
                                    self.font_description.clone(),
                                );
                                self.vwindows.insert(DEFAULT_WINDOW, win);
                                grid
                            };
                            self.relationships.insert(
                                grid,
                                Relation {
                                    id: DEFAULT_WINDOW,
                                    default_grid,
                                },
                            );
                        }
                        let rel = self.relationships.get(&grid).unwrap();
                        log::info!(
                            "resize grid {} to {}({})x{}({})",
                            grid,
                            rect.width(),
                            width,
                            rect.height(),
                            height
                        );
                        if rel.id != DEFAULT_WINDOW {
                            assert!(self
                                .vwindows
                                .get_mut(DEFAULT_WINDOW)
                                .unwrap()
                                .get(grid)
                                .is_none());
                        }
                        let window = self.vwindows.get_mut(rel.id).unwrap();
                        let exists = window.get(grid).is_some();
                        if exists {
                            window
                                .get_mut(grid)
                                .unwrap()
                                .resize(width as _, height as _);
                        } else {
                            // let rect = gdk::Rectangle::new(0, 0, width as _, height as _);
                            log::info!("Add grid {} to window {} at left top.", grid, rel.id);
                            window.add(grid, width as _, height as _, self.hldefs.clone());
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
                        let window = winid as u64;

                        let x = start_column as f64;
                        let y = start_row as f64;
                        let pos = FixedPosition { x, y };
                        let size = (width, height);
                        // let rect = gdk::Rectangle::new(x, y, width as i32, height as i32);
                        if !self.relationships.contains_key(&grid) {
                            let default_grid = if let Some(win) = self.vwindows.get(window) {
                                win.grid
                            } else {
                                let win = vimview::VimWindow::new(
                                    window,
                                    grid,
                                    pos,
                                    size,
                                    self.hldefs.clone(),
                                    self.font_description.clone(),
                                );
                                self.vwindows.insert(window, win);
                                grid
                            };
                            self.relationships.insert(
                                grid,
                                Relation {
                                    id: window,
                                    default_grid,
                                },
                            );
                        }
                        let rel = self.relationships.get_mut(&grid).unwrap();
                        assert_eq!(rel.id, window);
                        // log::info!(
                        //     "grid {} pos to {}({})x{}({})",
                        //     grid,
                        //     rect.width(),
                        //     width,
                        //     rect.height(),
                        //     height
                        // );
                        let window = self.vwindows.get_mut(window).unwrap();
                        let exists = window.get(grid).is_some();
                        if exists {
                            let gridview = window.get_mut(grid).unwrap();
                            gridview.resize(width as _, height as _);
                        } else {
                            log::info!(
                                "Add grid {} to window {}({}) at {}x{}.",
                                grid,
                                winid,
                                rel.id,
                                height,
                                width
                            );
                            window.add(grid, width as _, height as _, self.hldefs.clone());
                        };
                        let font_metrics = self.font_metrics.get();
                        window.set_pos(
                            start_column as f64 * font_metrics.lineheight,
                            start_row as f64 * font_metrics.charwidth,
                        );
                        let gridview = window.get_mut(grid).unwrap();
                        gridview.set_pos(
                            start_column as f64 * font_metrics.lineheight,
                            start_row as f64 * font_metrics.charwidth,
                        );
                        gridview.show();
                        window.show();
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
                        // self.rt.block_on(async {
                        //     log::info!("viewport window height: {:?}", window.get_height().await);
                        //     log::info!("viewport window width: {:?}", window.get_width().await);
                        //     log::info!(
                        //         "viewport window postion: {:?}",
                        //         window.get_position().await
                        //     );
                        // });

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

                        log::info!(
                            "window {} viewport grid {} viewport: top({}) bottom({}) highlight-line({}) highlight-column({}) with {} lines",
                             winid, grid, top_line, bottom_line, current_line, current_column, line_count,
                        );

                        let window = winid as u64;

                        // let x = start_column as i32;
                        // let y = start_row as i32;
                        // let rect = gdk::Rectangle::new(0, 0, 1, 1);
                        if !self.relationships.contains_key(&grid) {
                            let default_grid = if let Some(win) = self.vwindows.get(window) {
                                win.grid
                            } else {
                                let pos = FixedPosition {
                                    x: 0.,
                                    y: top_line as _,
                                };
                                let size = (80, line_count as _);
                                let win = vimview::VimWindow::new(
                                    window,
                                    grid,
                                    pos,
                                    size,
                                    self.hldefs.clone(),
                                    self.font_description.clone(),
                                );
                                self.vwindows.insert(window, win);
                                grid
                            };
                            self.relationships.insert(
                                grid,
                                Relation {
                                    id: window,
                                    default_grid,
                                },
                            );
                        }
                        let rel = self.relationships.get(&grid).unwrap();
                        assert_eq!(rel.id, window);
                        // log::info!(
                        //     "grid {} pos to {}({})x{}({})",
                        //     grid,
                        //     rect.width(),
                        //     width,
                        //     rect.height(),
                        //     height
                        // );
                        let window = self.vwindows.get_mut(window).unwrap();
                        let exists = window.get(grid).is_some();
                        if !exists {
                            log::info!(
                                "Add grid {} to window {}({}) at left top.",
                                grid,
                                winid,
                                rel.id
                            );
                            window.add(grid, 1, 1, self.hldefs.clone());
                        };
                        // let font_metrics = self.font_metrics.get();
                        // window.set_pos(
                        //     start_column as f64 * font_metrics.lineheight,
                        //     start_row as f64 * font_metrics.charwidth,
                        // );
                        let gridview = window.get_mut(grid).unwrap();
                        // gridview.set_pos(
                        //     start_column as f64 * font_metrics.lineheight,
                        //     start_row as f64 * font_metrics.charwidth,
                        // );
                        gridview.show();
                        window.show();
                    }
                    RedrawEvent::WindowHide { grid } => {
                        log::info!("hide {}", grid);
                        if let Some(win) = self.relationships.get(&grid) {
                            if win.default_grid == grid {
                                let window = self.vwindows.get_mut(win.id).unwrap();
                                window.hide();
                            } else {
                                self.vwindows
                                    .get_mut(win.id)
                                    .unwrap()
                                    .get_mut(grid)
                                    .unwrap()
                                    .hide();
                            }
                        }
                    }
                    RedrawEvent::WindowClose { grid } => {
                        if let Some(win) = self.relationships.get(&grid) {
                            if win.default_grid == grid {
                                log::info!("closing window {} by {}", win.id, grid);
                                self.vwindows.remove(win.id);
                            } else {
                                log::info!("closing grid {} of window {}", grid, win.id);
                                self.vwindows.get_mut(win.id).unwrap().remove(grid);
                            }
                        }
                        log::info!("removing relations {}", grid);
                        self.relationships.remove(&grid);
                    }
                    RedrawEvent::Destroy { grid } => {
                        if let Some(win) = self.relationships.get(&grid) {
                            if win.default_grid == grid {
                                log::info!("destroying window {} by {}", win.id, grid);
                                self.vwindows.remove(win.id);
                            } else {
                                log::info!("destroying grid {} of window {}", grid, win.id);
                                self.vwindows.get_mut(win.id).unwrap().remove(grid);
                            }
                        }
                        log::info!("destroying relations {}", grid);
                        self.relationships.remove(&grid);
                    }
                    RedrawEvent::Flush => {
                        self.vwindows.iter_mut().for_each(|(_, win)| {
                            // TODO
                            // win.quueu_allocate();
                            // win.quueu_resize();
                            win.queue_draw();
                        });
                    }
                    _ => {}
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
                // set_margin_all: 5,

                // set_child: Add tabline

                append: overlay = &gtk::Overlay {
                    set_child: da = Some(&gtk::DrawingArea) {
                        set_hexpand: true,
                        set_vexpand: true,
                        connect_resize[tx = tx, metrics = model.font_metrics.clone()] => move |da, width, height| {
                            log::info!("resizing width: {}, height: {}", width, height);
                            // let metrics = da.pango_context().metrics(unsafe { &*font_desc.as_ptr() }.into(), None).unwrap();
                            // log::info!("resizing metrics line-height {} char-width {}", metrics.height(), metrics.approximate_digit_width());
                            let metrics = metrics.get();
                            let rows = da.height() as f64 / metrics.lineheight;
                            let cols = da.width() as f64 / metrics.charwidth;
                            log::info!("rows: {} cols: {}", rows, cols);
                            tx.send((rows.ceil() as u64, cols.ceil() as u64)).unwrap();
                        },
                        // add_tick_callback[sender = sender.clone(), resized = Rc::clone(&resized), font_metrics = model.font_metrics.clone()] => move |da, _clock| {
                        //     // calculate easing use clock
                        //     let val = resized.compare_exchange(true,
                        //         false,
                        //         atomic::Ordering::Acquire,
                        //         atomic::Ordering::Relaxed
                        //     );
                        //     if let Ok(true) = val {
                        //         // log::info!("content height: {} widget height: {}", da.content_height(), da.height());
                        //         let font_metrics = font_metrics.get();
                        //         if font_metrics.lineheight == 0. || font_metrics.charwidth == 0. {
                        //             return glib::source::Continue(true)
                        //         }
                        //         let rows = da.height() as f64 / font_metrics.lineheight;
                        //         let cols = da.width() as f64 / font_metrics.charwidth;
                        //         log::info!("rows: {} cols: {}", rows, cols);
                        //         sender.send(UiCommand::Resize{ width: cols as _, height: rows as _ }.into()).unwrap();
                        //     }
                        //     glib::source::Continue(true)
                        // }
                    },
                    add_overlay: windows_container = &gtk::Fixed {
                        set_widget_name: "windows-container",
                        factory!(model.vwindows),
                    },
                    add_overlay: windows_float_container = &gtk::Fixed {
                        set_widget_name: "float-windows-container",
                    },
                    add_overlay: message_windows_container = &gtk::Fixed {
                        set_widget_name: "message-windows-container",
                    },
                }
            },
            connect_close_request(sender) => move |_| {
                sender.send(AppMessage::UiCommand(UiCommand::Quit)).ok();
                gtk::Inhibit(false)
            },
        }
    }

    fn pre_init() {
        let (tx, mut rx) = tokio::sync::watch::channel((0, 0));
        {
            let sender = sender.clone();
            model.rt.spawn(async move {
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_millis(65)).await;
                    if let Ok(true) = rx.has_changed() {
                        let (rows, cols) = *rx.borrow_and_update();
                        log::info!("resizing to {}x{}", rows, cols);
                        sender
                            .send(
                                UiCommand::Resize {
                                    width: cols as _,
                                    height: rows as _,
                                }
                                .into(),
                            )
                            .unwrap();
                        log::info!("resizing sent");
                    }
                }
            });
        }
    }

    fn post_init() {
        let pctx = vbox.pango_context();
        model.pctx.set(pctx.into()).ok();
        model.compute();
        // log::info!("metrics after init: {:?}", model.font_metrics.get());
        // TODO: change window size (px) to match vim's viewport (cols and rows).
    }
}
