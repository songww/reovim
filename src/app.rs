use std::cell::{Cell, RefCell};
use std::sync::RwLock;
use std::{rc::Rc, sync::atomic};

use gdk::prelude::FontFamilyExt;
use gtk::prelude::{
    BoxExt, DrawingAreaExt, GtkWindowExt, OrientableExt, WidgetExt, WidgetExtManual,
};
use once_cell::sync::OnceCell;
use pango::FontDescription;
use relm4::{send, AppUpdate, Model, RelmApp, Sender, WidgetPlus, Widgets};
use rustc_hash::FxHashMap;

use crate::vimview;
use crate::{
    bridge::{self, RedrawEvent, UiCommand},
    style, Opts,
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
pub struct Relation {
    id: u64,
    is_base: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct FontMetrics {
    linespace: f64,
    lineheight: f64,
    charwidth: f64,
}

impl FontMetrics {
    fn new() -> FontMetrics {
        FontMetrics {
            linespace: 0.,
            lineheight: 0.,
            charwidth: 0.,
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

            font_description: Rc::new(RefCell::new(FontDescription::from_string("monospace h14"))),

            pctx: OnceCell::new(),

            hldefs: Rc::new(RwLock::new(vimview::HighlightDefinitions::new())),

            vwindows: crate::factory::FactoryMap::new(),
            relationships: FxHashMap::default(),

            opts,
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
                            log::error!("line space: {}", linespace);
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
                        log::debug!("grid line {}", grid);
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

                            log::debug!(
                                "grid line {} - {} cells at {}x{}",
                                grid,
                                cells.len(),
                                row,
                                column_start
                            );

                            self.vwindows
                                .get_mut(win.id)
                                .unwrap()
                                .get_mut(grid)
                                .unwrap()
                                .textbuf()
                                .borrow()
                                .set_cells(row as _, column_start as _, &cells);
                        };
                    }
                    RedrawEvent::Resize {
                        grid,
                        width,
                        height,
                    } => {
                        // let (width_px, height_px) = (
                        //     (width as f64 * self.charwidth) as i32,
                        //     (height as f64 * self.lineheight) as i32,
                        // );

                        let rect = gdk::Rectangle::new(0, 0, width as _, height as _);
                        if !self.relationships.contains_key(&grid) {
                            self.relationships.insert(
                                grid,
                                Relation {
                                    id: 1,
                                    is_base: false,
                                },
                            );
                            if self.vwindows.get(1).is_none() {
                                let mut win = vimview::VimWindow::new(
                                    1,
                                    grid,
                                    rect,
                                    self.hldefs.clone(),
                                    self.font_description.clone(),
                                );
                                // win.add(grid, width, height, self.hldefs.clone());
                                self.vwindows.insert(1, win);
                            };
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
                        let window = self.vwindows.get_mut(rel.id).unwrap();
                        let exists = window.get(grid).is_some();
                        if exists {
                            window
                                .get_mut(grid)
                                .unwrap()
                                .resize(width as _, height as _);
                        } else {
                            // let rect = gdk::Rectangle::new(0, 0, width as _, height as _);
                            log::warn!("Add grid {} to window {} at left top.", grid, rel.id);
                            window.add(grid, width as _, height as _, self.hldefs.clone());
                        };
                    }

                    RedrawEvent::WindowPosition {
                        grid,
                        start_row,
                        start_column,
                        width,
                        height,
                    } => {
                        // let (width_px, height_px) = (
                        //     width as f64 * self.charwidth,
                        //     height as f64 * self.lineheight,
                        // );
                        // let (start_width_px, start_height_px) = (
                        //     start_column as f64 * self.charwidth,
                        //     start_row as f64 * self.lineheight,
                        // );

                        let x = start_column as i32;
                        let y = start_row as i32;
                        let rect = gdk::Rectangle::new(x, y, width as i32, height as i32);
                        if !self.relationships.contains_key(&grid) {
                            self.relationships.insert(
                                grid,
                                Relation {
                                    id: 1,
                                    is_base: false,
                                },
                            );
                            if self.vwindows.get(1).is_none() {
                                let mut win = vimview::VimWindow::new(
                                    1,
                                    grid,
                                    rect,
                                    self.hldefs.clone(),
                                    self.font_description.clone(),
                                );
                                self.vwindows.insert(1, win);
                            };
                        }
                        let rel = self.relationships.get(&grid).unwrap();
                        // log::info!(
                        //     "grid {} pos to {}({})x{}({})",
                        //     grid,
                        //     rect.width(),
                        //     width,
                        //     rect.height(),
                        //     height
                        // );
                        let window = self.vwindows.get_mut(rel.id).unwrap();
                        let exists = window.get(grid).is_some();
                        if exists {
                            window
                                .get_mut(grid)
                                .unwrap()
                                .resize(width as _, height as _);
                        } else {
                            // let rect = gdk::Rectangle::new(0, 0, width as _, height as _);
                            log::warn!("Add grid {} to window {} at left top.", grid, rel.id);
                            window.add(grid, width as _, height as _, self.hldefs.clone());
                        };
                        window.set_pos(start_column, start_row);
                        // window.add(grid, width, height, self.hldefs.clone());
                        window
                            .get_mut(grid)
                            .unwrap()
                            .set_pos(start_column, start_row);
                        // self.vwindows.insert(
                        //     grid,
                        //     vimview::VimWindow::new(
                        //         0,
                        //         grid,
                        //         rect,
                        //         self.hldefs.clone(),
                        //         self.font_description.clone(),
                        //     ),
                        // );
                        log::info!(
                            "Ignored window {} position: row-start({}) col-start({}) width({}) height({})",
                            grid, start_row, start_column, width, height,
                        );
                    }
                    RedrawEvent::WindowViewport {
                        grid,
                        top_line,
                        bottom_line,
                        current_line,
                        current_column,
                        line_count,
                    } => {
                        /*
                        let rel = self.relationships.get(&grid).unwrap();
                        let mut window = self.vwindows.get_mut(rel.id).unwrap();
                        window
                            .get_mut(grid)
                            .unwrap()
                            .set_pos(current_column as _, top_line as _);
                        */
                        log::info!(
                            "Ignored window grid {} viewport: top({}) bottom({}) line-from({}) col-from({}) with {} lines",
                            grid, top_line, bottom_line, current_line, current_column, line_count,
                        );
                        // let win = vim_window::VimWindow::new(grid, gdk::Rectangle::new(start_row as _, start_column as _, width as _, height as _);
                        // self.vwindows.insert(grid, win);
                    }
                    RedrawEvent::WindowHide { grid } => {
                        log::info!("hide {}", grid);
                        if let Some(win) = self.relationships.get(&grid) {
                            if win.is_base {
                                self.vwindows.get_mut(win.id).unwrap().hide();
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
                        log::info!("close {}", grid);
                        if let Some(win) = self.relationships.get(&grid) {
                            if win.is_base {
                                self.vwindows.remove(grid);
                            } else {
                                self.vwindows.get_mut(win.id).unwrap().remove(grid);
                            }
                        }
                        self.relationships.remove(&grid);
                    }
                    RedrawEvent::Destroy { grid } => {
                        log::info!("destroy {}", grid);
                        if let Some(win) = self.relationships.get(&grid) {
                            if win.is_base {
                                self.vwindows.remove(grid);
                            } else {
                                self.vwindows.get_mut(win.id).unwrap().remove(grid);
                            }
                        }
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
                        connect_resize[_sender = sender.clone(), resized: Rc<atomic::AtomicBool> = Rc::clone(&resized)] => move |da, width, height| {
                            log::info!("resizing width: {}, height: {}", width, height);
                            resized.compare_exchange(false,
                                true,
                                atomic::Ordering::Acquire,
                                atomic::Ordering::Relaxed
                            ).ok();
                        },
                        add_tick_callback[sender = sender.clone(), resized = Rc::clone(&resized), font_metrics = model.font_metrics.clone()] => move |da, _clock| {
                            // calculate easing use clock
                            let val = resized.compare_exchange(true,
                                      false,
                                      atomic::Ordering::Acquire,
                                      atomic::Ordering::Relaxed);
                            if let Ok(true) = val {
                                let font_metrics = font_metrics.get();
                                log::info!("content height: {} widget height: {}", da.content_height(), da.height());
                                let rows = da.height() as f64 / font_metrics.lineheight;
                                let cols = da.width() as f64 / font_metrics.charwidth;
                                log::info!("rows: {} cols: {}", rows, cols);
                                sender.send(UiCommand::Resize{ width: cols as _, height: rows as _ }.into()).unwrap();
                            }
                            glib::source::Continue(true)
                        }
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

    additional_fields! {
        resized: Rc<atomic::AtomicBool>,
    }

    fn pre_init() {
        let resized = Rc::new(false.into());
    }

    fn post_init() {
        let pctx = vbox.pango_context();
        model.pctx.set(pctx.into()).ok();
        model.compute();
        log::info!("metrics after init: {:?}", model.font_metrics.get());
        // TODO: change window size (px) to match vim's viewport (cols and rows).
    }
}
