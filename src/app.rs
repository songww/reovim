use std::cell::Cell;
use std::sync::RwLock;
use std::{rc::Rc, sync::atomic};

use gtk::prelude::{
    BoxExt, DrawingAreaExt, GtkWindowExt, OrientableExt, WidgetExt, WidgetExtManual,
};
use relm4::{send, AppUpdate, Model, RelmApp, Sender, WidgetPlus, Widgets};
use rustc_hash::FxHashMap;

use crate::vimview;
use crate::{
    bridge::{self, RedrawEvent, UiCommand},
    style, vim_window, Opts,
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

#[derive(Debug)]
struct Relation {
    id: u64,
    is_base: bool,
}

pub struct AppModel {
    pub opts: Opts,
    pub title: String,
    pub default_width: i32,
    pub default_height: i32,

    pub guifont: Option<String>,
    pub guifontset: Option<String>,
    pub guifontwide: Option<String>,
    pub linespace: Option<u64>,
    pub show_tab_line: Option<u64>,

    pub font_description: Cell<Option<pango::FontDescription>>,

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
            linespace: None,
            show_tab_line: None,

            font_description: Cell::new(None),

            hldefs: Rc::new(RwLock::new(vimview::HighlightDefinitions::new())),

            vwindows: crate::factory::FactoryMap::new(),
            relationships: FxHashMap::default(),

            opts,
        }
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
        log::info!("message at AppModel::update {:?}", message);
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
                // components.messager.send(event);
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
                            log::warn!("unhandled ambi_width {}", ambi_width);
                        }
                        bridge::GuiOption::ArabicShape(arabic_shape) => {
                            log::warn!("unhandled arabic-shape: {}", arabic_shape);
                        }
                        bridge::GuiOption::Emoji(emoji) => {
                            log::warn!("emoji: {}", emoji);
                        }
                        bridge::GuiOption::GuiFont(guifont) => {
                            let desc = pango::FontDescription::from_string(&guifont);
                            // for (_, win) in self.vwindows.iter() {
                            //     win.set_font_description(desc.clone());
                            // }
                            self.guifont.replace(guifont);
                            self.font_description.replace(desc.into());
                        }
                        bridge::GuiOption::GuiFontSet(guifontset) => {
                            self.guifontset.replace(guifontset);
                        }
                        bridge::GuiOption::GuiFontWide(guifontwide) => {
                            self.guifontwide.replace(guifontwide);
                        }
                        bridge::GuiOption::LineSpace(linespace) => {
                            self.linespace.replace(linespace);
                        }
                        bridge::GuiOption::ShowTabLine(show_tab_line) => {
                            self.show_tab_line.replace(show_tab_line);
                        }
                        bridge::GuiOption::TermGuiColors(term_gui_colors) => {
                            log::warn!("unhandled term gui colors: {}", term_gui_colors);
                        }
                        bridge::GuiOption::Pumblend(pumblend) => {
                            log::warn!("unhandled pumblend: {}", pumblend)
                        }
                        bridge::GuiOption::Unknown(name, value) => {
                            log::warn!("GuiOption({}: {:?}) not supported yet.", name, value)
                        }
                    },
                    RedrawEvent::DefaultColorsSet { colors } => {
                        self.hldefs.write().unwrap().set_defaults(colors);
                    }
                    RedrawEvent::HighlightAttributesDefine { id, style } => {
                        self.hldefs.write().unwrap().set(id, style);
                    }
                    RedrawEvent::Clear { grid } => {
                        if let Some(win) = self.relationships.get(&grid) {
                            self.vwindows
                                .get_mut(win.id)
                                .map(|w| w.get_mut(grid).clear())
                                .expect(&format!("grid {} not found.", grid));
                        };
                    }
                    RedrawEvent::GridLine {
                        grid,
                        row,
                        column_start,
                        cells,
                    } => {
                        if let Some(win) = self.relationships.get(&grid) {
                            // FIXME: check is base.
                            self.vwindows
                                .get_mut(win.id)
                                .unwrap()
                                .get_mut(grid)
                                .unwrap()
                                .set_line(row, column_start, cells);
                        };
                    }
                    RedrawEvent::Resize {
                        grid,
                        width,
                        height,
                    } => {
                        if let Some(win) = self.relationships.get(&grid) {
                            // FIXME: check is base.
                            let exists = self.vwindows.get(win.id).unwrap().get(grid).is_some();
                            if exists {
                                self.vwindows
                                    .get_mut(win.id)
                                    .unwrap()
                                    .get_mut(grid)
                                    .unwrap()
                                    .resize(width as _, height as _);
                            } else {
                                let rect = gdk::Rectangle::new(0, 0, width as _, height as _);
                                let window =
                                    vimview::VimWindow::new(0, grid, rect, Rc::clone(&self.hldefs));
                                self.vwindows.get_mut(win.id).unwrap().insert(grid, window);
                            };
                        };
                    }
                    RedrawEvent::WindowPosition {
                        grid,
                        start_row,
                        start_column,
                        width,
                        height,
                    } => {
                        // FIXME: row and column to device coord
                        let rect = gdk::Rectangle::new(
                            start_row as _,
                            start_column as _,
                            width as _,
                            height as _,
                        );
                        // self.vwindows.insert(
                        //     grid,
                        //     vimview::VimWindow::new(0, grid, rect, Rc::clone(&self.hldefs)),
                        // );
                        // FIXME: window position
                    }
                    RedrawEvent::WindowViewport {
                        grid,
                        top_line,
                        bottom_line,
                        current_line,
                        current_column,
                        line_count,
                    } => {
                        // let win = vim_window::VimWindow::new(grid, gdk::Rectangle::new(start_row as _, start_column as _, width as _, height as _);
                        // self.vwindows.insert(grid, win);
                    }
                    RedrawEvent::WindowHide { grid } => {
                        if let Some(win) = self.relationships.get(&grid) {
                            if win.is_base {
                                self.vwindows.get(win.id).unwrap().hide();
                            } else {
                                self.vwindows.get_mut(win.id).unwrap().get_mut(grid).hide();
                            }
                        }
                    }
                    RedrawEvent::WindowClose { grid } => {
                        if let Some(win) = self.relationships.get(&grid) {
                            if win.is_base {
                                self.vwindows.remove(grid);
                            } else {
                                self.vwindows.get_mut(win.id).unwrap().remove(grid);
                            }
                        }
                    }
                    RedrawEvent::Destroy { grid } => {
                        if let Some(win) = self.relationships.get(&grid) {
                            if win.is_base {
                                self.vwindows.remove(grid);
                            } else {
                                self.vwindows.get_mut(win.id).unwrap().remove(grid);
                            }
                        }
                    }
                    RedrawEvent::Flush => {
                        // FIXME: redraw all only this set.
                        self.vwindows.queue_draw();
                        // self.vwindows.iter().for_each(|(_k, win)| win.queue_draw());
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
                // set_margin_all: 5,

                // set_child: Add tabline

                append: overlay = &gtk::Overlay {
                    set_child: da = Some(&gtk::DrawingArea) {
                        set_hexpand: true,
                        connect_resize[_sender = sender.clone(), resized: Rc<atomic::AtomicBool> = Rc::clone(&resized)] => move |da, width, height| {
                            resized.compare_exchange(false,
                                      true,
                                      atomic::Ordering::Acquire,
                                      atomic::Ordering::Relaxed).ok();
                        },
                        add_tick_callback[sender = sender.clone(), resized = Rc::clone(&resized)] => move |da, _clock| {
                            // calculate easing use clock
                            let val = resized.compare_exchange(true,
                                      false,
                                      atomic::Ordering::Acquire,
                                      atomic::Ordering::Relaxed);
                            if let Ok(true) = val {
                                log::debug!("content height: {} widget height: {}", da.content_height(), da.height());
                                sender.send(UiCommand::Resize{ width: da.width() as _, height: da.height() as _ }.into()).unwrap();
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
            // connect_close_
        }
    }
    additional_fields! {
        resized: Rc<atomic::AtomicBool>,
    }
    fn pre_init() {
        let resized = Rc::new(false.into());
    }
}
