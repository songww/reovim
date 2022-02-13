use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::{atomic, RwLock};

use glib::ObjectExt;
use gtk::prelude::{
    BoxExt, DrawingAreaExt, DrawingAreaExtManual, EventControllerExt, GtkWindowExt, OrientableExt,
    WidgetExt,
};
use once_cell::sync::OnceCell;
use pango::FontDescription;
use relm4::{send, AppUpdate, Model, RelmApp, Sender, WidgetPlus, Widgets};
use rustc_hash::FxHashMap;

use crate::vimview::{self, VimGrid};
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
pub struct GridWindow {
    // window number
    winid: u64,
    // default grid
    //default_grid: u64,
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

    pub fn charwidth(&self) -> f64 {
        self.charwidth
    }

    pub fn linespace(&self) -> f64 {
        self.linespace
    }

    pub fn lineheight(&self) -> f64 {
        self.lineheight
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

    pub flush: Rc<atomic::AtomicBool>,
    pub focused: Rc<atomic::AtomicU64>,
    pub background_changed: Rc<atomic::AtomicBool>,

    pub vgrids: crate::factory::FactoryMap<vimview::VimGrid>,
    // relations about grid with window.
    pub relationships: FxHashMap<u64, GridWindow>,

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

            font_metrics: Rc::new(FontMetrics::new().into()),

            font_description: Rc::new(RefCell::new(FontDescription::from_string("monospace 17"))),

            pctx: OnceCell::new(),

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

    pub fn compute(&self) {
        const SINGLE_WIDTH_CHARS: &'static str = concat!(
            "! \" # $ % & ' ( ) * + , - . / ",
            "0 1 2 3 4 5 6 7 8 9 ",
            ": ; < = > ? @ ",
            "A B C D E F G H I J K L M N O P Q R S T U V W X Y Z ",
            "[ \\ ] ^ _ ` ",
            "a b c d e f g h i j k l m n o p q r s t u v w x y z ",
            "{ | } ~ \n",
        );
        let desc = self.font_description.borrow();
        log::debug!(
            "font desc {} {} {} {}pt",
            desc.family().unwrap(),
            desc.weight(),
            desc.style(),
            desc.size() / pango::SCALE,
        );
        let pctx = self.pctx.get().unwrap();
        pctx.set_font_description(&desc);
        let layout = pango::Layout::new(pctx);
        let metrics = pctx.metrics(Some(&desc), None).unwrap();
        layout.set_text(SINGLE_WIDTH_CHARS);
        let lineheight = layout.line(1).unwrap().height();
        let mut font_metrics = self.font_metrics.get();
        font_metrics.lineheight = lineheight as f64 / pango::SCALE as f64;
        font_metrics.charwidth = metrics.approximate_digit_width() as f64 / pango::SCALE as f64;
        log::info!("line-height {:?}", font_metrics.lineheight);
        log::info!("char-width {:?}", font_metrics.charwidth);
        self.font_metrics.replace(font_metrics);
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

                                self.guifont.replace(guifont);
                                self.font_description.replace(desc);

                                self.compute();

                                self.flush.store(true, atomic::Ordering::Relaxed);
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
                            self.vgrids.insert(
                                grid,
                                VimGrid::new(
                                    grid,
                                    0,
                                    (0., 0.).into(),
                                    (width, height).into(),
                                    self.flush.clone(),
                                    self.hldefs.clone(),
                                    self.font_metrics.clone(),
                                    self.font_description.clone(),
                                ),
                            );
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

                        let font_metrics = self.font_metrics.get();
                        let x = (start_column) as f64 * font_metrics.charwidth;
                        let y =
                            (start_row) as f64 * (font_metrics.lineheight + font_metrics.linespace); //;

                        if self.vgrids.get(grid).is_none() {
                            // dose not exists, create
                            self.vgrids.insert(
                                grid,
                                VimGrid::new(
                                    grid,
                                    winid,
                                    (x.floor(), y.floor()).into(),
                                    (width, height).into(),
                                    self.flush.clone(),
                                    self.hldefs.clone(),
                                    self.font_metrics.clone(),
                                    self.font_description.clone(),
                                ),
                            );
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

                        // self.focused.store(grid, atomic::Ordering::Relaxed);

                        let (x, y) = self.rt.block_on(window.get_position()).unwrap();
                        let (x, y) = (x as f64, y as f64);
                        async fn allocated(
                            window: nvim::Window<crate::bridge::Tx>,
                        ) -> Result<(i64, i64), Box<nvim::error::CallError>>
                        {
                            Ok((window.get_width().await?, window.get_height().await?))
                        }
                        let (width, height): (i64, i64) =
                            self.rt.block_on(allocated(window)).unwrap();
                        let (width, height) = (width as usize, height as usize);

                        log::info!(
                            "window {} viewport grid {} viewport: top({}) bottom({}) highlight-line({}) highlight-column({}) with {} lines",
                             winid, grid, top_line, bottom_line, current_line, current_column, line_count,
                        );

                        let winid = winid as u64;

                        if self.vgrids.get(grid).is_none() {
                            // dose not exists, create
                            self.vgrids.insert(
                                grid,
                                VimGrid::new(
                                    grid,
                                    winid,
                                    (x, y).into(),
                                    (width, height).into(),
                                    self.flush.clone(),
                                    self.hldefs.clone(),
                                    self.font_metrics.clone(),
                                    self.font_description.clone(),
                                ),
                            );
                            self.relationships.insert(grid, GridWindow { winid });
                            log::info!(
                                "Add grid {} to window {} at {}x{}.",
                                grid,
                                winid,
                                height,
                                width
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
                        connect_resize[sender = sender.clone(), metrics = model.font_metrics.clone()] => move |da, width, height| {
                            log::info!("da resizing width: {}, height: {}", width, height);
                            let metrics = metrics.get();
                            let rows = da.height() as f64 / (metrics.lineheight + metrics.linespace); //  + metrics.linespace
                            let cols = da.width() as f64 / metrics.charwidth;
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
                    add_overlay: windows_container = &gtk::Fixed {
                        set_widget_name: "windows-container",
                        factory!(model.vgrids),
                    },
                    add_overlay: windows_float_container = &gtk::Fixed {
                        set_widget_name: "float-windows-container",
                        set_visible: false,
                    },
                    add_overlay: message_windows_container = &gtk::Fixed {
                        set_widget_name: "message-windows-container",
                        set_visible: false,
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
        model.compute();

        /*
        let click_listener = gtk::GestureClick::builder()
            .button(0)
            .exclusive(false)
            .touch_only(false)
            .n_points(1)
            .name("click-listener")
            .build();
        click_listener.connect_pressed(
            glib::clone!(@strong sender, @strong grid => move |c, n_press, x, y| {
                log::info!("{:?} pressed {} times at {}x{}", c.name(), n_press, x, y);
                sender.send(
                    UiCommand::MouseButton {
                        action: "press".to_string(),
                        grid_id: grid,
                        position: (x as u32, y as u32)
                    }.into()
                ).expect("Failed to send mouse press event");
            }),
        );
        click_listener.connect_released(
            glib::clone!(@strong sender, @strong grid => move |c, n_press, x, y| {
                log::info!("{:?} released {} times at {}x{}", c.name(), n_press, x, y);
                sender.send(
                    UiCommand::MouseButton {
                        action: "release".to_string(),
                        grid_id: grid,
                        position: (x as u32, y as u32)
                    }.into()
                ).expect("Failed to send mouse event");
            }),
        );
        windows_container.add_controller(&click_listener);
        let on_click = gtk::GestureClick::new();
        on_click.set_name("vim-window-onclick-listener");
        on_click.connect_pressed(glib::clone!(@strong sender => move |c, n_press, x, y| {
            log::debug!("{:?} pressed {} times at {}x{}", c.name(), n_press, x, y);
            sender.send(
                UiCommand::MouseButton {
                    action: "press".to_string(),
                    grid_id: grid,
                    position: (x as u32, y as u32)
                }.into()
            ).expect("Failed to send mouse press event");
        }));
        on_click.connect_released(glib::clone!(@strong sender => move |c, n_press, x, y| {
            log::debug!("{:?} released {} times at {}x{}", c.name(), n_press, x, y);
            sender.send(
                UiCommand::MouseButton {
                    action: "release".to_string(),
                    grid_id: grid,
                    position: (x as u32, y as u32)
                }.into()
            ).expect("Failed to send mouse event");
        }));
        overlay.add_controller(&on_click);
        */
        // on_click.connect_stopped(cloned!(sender => move |c| {
        //     log::debug!("Click({}) stopped", c.name().unwrap());
        //     // sender.send(AppMsg::Message(format!("Click({}) stopped", c.name().unwrap()))).unwrap();
        // }));
        // on_click.connect_unpaired_release(cloned!(sender => move |c, n_press, x, y, events| {
        //     // sender.send(AppMsg::Message(format!("Click({:?}) unpaired release {} times at {}x{} {:?}", c.group(), n_press, x, y, events))).unwrap();
        //     log::debug!("Click({:?}) unpaired release {} times at {}x{} {:?}", c.group(), n_press, x, y, events);
        // }));

        /*
        listener.connect_decelerate(
            glib::clone!(@strong sender, @strong model.focused as focused => move |_, x, y| {
                let grid_id = focused.load(atomic::Ordering::Relaxed);
                log::debug!("scroll decelerate x: {}, y: {}", x, y);
                // sender.send(AppMsg::Message(format!("scroll decelerate x: {}, y: {}", x, y))).unwrap();
            }),
        );
        listener.connect_scroll_begin(
            glib::clone!(@strong sender, @strong model.focused as focused => move |_| {
                let grid_id = focused.load(atomic::Ordering::Relaxed);
                log::debug!("scroll begin");
                // sender.send(AppMsg::Message(format!("scroll begin"))).unwrap();
            }),
        );
        listener.connect_scroll_end(
            glib::clone!(@strong sender, @strong model.focused as focused => move |_| {
                let grid_id = focused.load(atomic::Ordering::Relaxed);
                log::debug!("scroll end");
                // sender.send(AppMsg::Message(format!("scroll end"))).unwrap();
            }),
        );
        overlay.add_controller(&listener);
        */

        let listener = gtk::EventControllerScroll::builder()
            .flags(gtk::EventControllerScrollFlags::all())
            .name("scroller-listener")
            .build();
        listener.connect_scroll(glib::clone!(@strong sender => move |c, x, y| {
            let grid: u64 = if let Some(child) = c.widget().focus_child() {
                child.property("id")
            } else {
                return gtk::Inhibit(false)
            };
            let direction = if y > 0. {
                "down"
            } else {
                "up"
            };
            // let grid_id = focused.load(atomic::Ordering::Relaxed);
            let command = UiCommand::Scroll { direction: direction.into(), grid_id: grid, position: (0, 1) };
            sender.send(AppMessage::UiCommand(command)).unwrap();
            log::debug!("scrolling grid {} x: {}, y: {}", grid, x, y);
            gtk::Inhibit(false)
        }));
        overlay.add_controller(&listener);
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
    }
}
