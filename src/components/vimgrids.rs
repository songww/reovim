use std::cell::{Cell, RefCell};
use std::convert::identity;
use std::fmt::Debug;
use std::rc::Rc;
use std::sync::{atomic, RwLock};

use gtk::prelude::*;
use relm4::factory::Position;
use relm4::prelude::*;
use rustc_hash::FxHashMap as HashMap;
use tracing::{debug, info, trace, warn};

use crate::app::{self, AppMessage, Dragging};
use crate::bridge::{MouseAction, MouseButton, SerialCommand, UiCommand};
use crate::event_aggregator::EVENT_AGGREGATOR;
use crate::grapheme::{Coord, Pos, Rectangle};

use crate::vimview::BinGrid;
use crate::vimview::TextBuf;

type HighlightDefinitions = Rc<RwLock<crate::vimview::HighlightDefinitions>>;

#[derive(Clone, Debug)]
pub struct VimGrid {
    win: u64,
    grid: u64,
    pos: Pos,
    coord: Coord,
    move_to: Cell<Option<Pos>>,
    width: usize,
    height: usize,
    is_float: bool,
    focusable: bool,
    metrics: Rc<Cell<crate::metrics::Metrics>>,
    font_description: Rc<RefCell<pango::FontDescription>>,
    dragging: Rc<Cell<Option<Dragging>>>,

    textbuf: TextBuf,

    visible: bool,
    // animation: Option<adw::TimedAnimation>,
}

impl VimGrid {
    pub fn new(
        id: u64,
        winid: u64,
        coord: Coord,
        rect: Rectangle,
        hldefs: HighlightDefinitions,
        dragging: Rc<Cell<Option<Dragging>>>,
        metrics: Rc<Cell<crate::metrics::Metrics>>,
        font_description: Rc<RefCell<pango::FontDescription>>,
    ) -> VimGrid {
        let textbuf = TextBuf::new(rect.height, rect.width);
        textbuf.borrow().set_hldefs(hldefs.clone());
        textbuf.borrow().set_metrics(metrics.clone());
        let m = metrics.get();
        VimGrid {
            win: winid,
            grid: id,
            pos: (coord.col as f64 * m.width(), coord.row as f64 * m.height()).into(),
            coord,
            width: rect.width as _,
            height: rect.height as _,
            move_to: None.into(),
            dragging,
            is_float: false,
            focusable: true,
            metrics,
            textbuf,
            visible: true,
            font_description,
            // animation: None,
        }
    }

    pub fn id(&self) -> u64 {
        self.grid
    }

    pub fn textbuf(&self) -> &TextBuf {
        &self.textbuf
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn coord(&self) -> &Coord {
        &self.coord
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn show(&mut self) {
        self.visible = true;
    }

    pub fn clear(&self) {
        self.textbuf().borrow().clear();
    }

    pub fn reset_cache(&mut self) {
        self.textbuf().borrow().reset_cache();
    }

    // content go up, view go down, eat head of rows.
    pub fn up(
        &mut self,
        // top: usize,
        // bottom: usize,
        // left: usize,
        // right: usize,
        rows: usize,
        // cols: usize,
    ) {
        debug!("scroll-region {} rows moved up.", rows);
        debug!(
            "Origin Region {:?} {}x{}",
            self.coord, self.width, self.height
        );
        self.textbuf().borrow_mut().up(rows);
    }

    // content go down, view go up, eat tail of rows.
    pub fn down(&mut self, rows: usize) {
        debug!("scroll-region {} rows moved down.", rows);
        debug!(
            "Origin Region {:?} {}x{}",
            self.coord, self.width, self.height
        );
        self.textbuf().borrow_mut().down(rows);
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        self.visible = true;

        self.width = width;
        self.height = height;
        self.textbuf().borrow().resize(height, width);
    }

    pub fn set_coord(&mut self, col: f64, row: f64) {
        self.visible = true;

        let metrics = self.metrics.get();
        let pos: Pos = (col * metrics.width(), row * metrics.height()).into();
        self.pos = pos;
        self.coord = Coord { col, row };
        self.move_to.replace(pos.into());
    }

    pub fn set_is_float(&mut self, is_float: bool) {
        self.visible = is_float;
        self.is_float = is_float;
    }

    pub fn set_focusable(&mut self, focusable: bool) {
        self.focusable = focusable;
    }

    pub fn set_pango_context(&self, pctx: Rc<pango::Context>) {
        self.textbuf().borrow().set_pango_context(pctx);
    }
}

impl Position<crate::widgets::board::BoardPosition> for VimGrid {
    fn position(&self, _index: usize) -> crate::widgets::board::BoardPosition {
        crate::widgets::board::BoardPosition {
            x: self.pos.x,
            y: self.pos.y,
        }
    }
}

#[derive(Debug)]
pub enum Event {
    ResetCache,
    Show, //
    Clear,
    WindowClose, //
    WindowHide,  //
    Scroll {
        top: u64,
        bottom: u64,
        left: u64,
        right: u64,
        rows: i64,
        columns: i64,
    },
    Resize {
        width: usize,
        height: usize,
    },
    Float {
        focusable: bool,
    },
}

#[derive(Debug)]
pub enum GridEvent {
    Flush,
    ResetCache,
    Show(u64), //
    Clear(u64),
    Destroy(u64),     //
    WindowClose(u64), //
    WindowHide(u64),  //
    Scroll {
        grid: u64,
        top: u64,
        bottom: u64,
        left: u64,
        right: u64,
        rows: i64,
        columns: i64,
    },
    Resize {
        grid: u64,
        width: u64,
        height: u64,
    },
    Add {
        grid: u64,
        win: u64,
        coord: Coord,
        rectangle: Rectangle,
        hldefs: HighlightDefinitions,
        dragging: Rc<Cell<Option<Dragging>>>,
    },
    /// Should shown
    AtPosition {
        grid: u64,
        coord: Coord,
        rectangle: Rectangle,
    },
    FloatPosition {
        grid: u64,
        coord: Coord,
        focusable: bool,
    },
}

#[relm4::component(pub)]
impl SimpleComponent for VimGrid {
    // type Root = BinGrid;
    type Widgets = VimGridWidgets;

    type Input = Event;
    type Output = AppMessage;

    type Init = (
        u64,
        u64,
        Coord,
        Rectangle,
        HighlightDefinitions,
        Rc<Cell<Option<Dragging>>>,
        Rc<Cell<crate::metrics::Metrics>>,
        Rc<RefCell<pango::FontDescription>>,
    );

    view! {
        view = BinGrid {
            // model.grid, model.width as _, model.height as _) {
            set_gid: model.grid,
            set_width: model.width as _,
            set_height: model.height as _,
            set_textbuf: model.textbuf.clone(),

            set_widget_name: &format!("vim-grid-{}-{}", model.win, model.grid),

            #[watch]
            set_visible: model.visible,
            set_can_focus: true,
            set_focusable: true,
            set_focus_on_click: true,

            set_overflow: gtk::Overflow::Hidden,

            set_font_description: &model.font_description.borrow(),

            set_css_classes: &["vim-view-grid", &format!("vim-view-grid-{}", model.grid)],
        }
    }

    fn init(
        (id, winid, coord, rect, hldefs, dragging, metrics, font_description): Self::Init,
        view: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let click_listener = gtk::GestureClick::builder()
            .button(0)
            .exclusive(false)
            .touch_only(false)
            .n_points(1)
            .name("click-listener")
            .build();
        let gridid = id;
        click_listener.connect_pressed(
            glib::clone!(@strong sender, @weak dragging, @weak metrics => move |c, n_press, x, y| {
                sender.output(AppMessage::ShowPointer).unwrap();
                let metrics = metrics.get();
                let width = metrics.width();
                let height = metrics.height();
                let cols = x as f64 / width;
                let rows = y as f64 / height;
                trace!(grid = gridid, "mouse pressed {} times at {}x{} -> {}x{}", n_press, x, y, cols, rows);
                let position = (cols.floor() as u32, rows.floor() as u32);
                let modifier = c.current_event_state().to_string();
                let btn = match c.current_button() {
                    1 => MouseButton::Left,
                    2 => MouseButton::Middle,
                    3 => MouseButton::Right,
                    _ => { return; }
                };
                dragging.set(Dragging{ btn, pos: position}.into());
                EVENT_AGGREGATOR.send(
                    UiCommand::Serial(SerialCommand::MouseButton {
                        action: MouseAction::Press,
                        button: btn,
                        modifier: c.current_event_state(),
                        grid_id: gridid,
                        position
                    })
                );
                trace!(grid = id, "release button {} current_button {} modifier {}", c.button(), c.current_button(), modifier);
            }),
        );
        click_listener.connect_released(
            glib::clone!(@strong sender, @weak dragging, @weak metrics => move |c, n_press, x, y| {
                sender.output(app::AppMessage::ShowPointer).unwrap();
                let metrics = metrics.get();
                let width = metrics.width();
                let height = metrics.height();
                let cols = x as f64 / width;
                let rows = y as f64 / height;
                trace!("grid {} mouse released {} times at {}x{} -> {}x{}", gridid, n_press, x, y, cols, rows);
                let modifier = c.current_event_state().to_string();
                dragging.set(None);
                let btn = match c.current_button() {
                    1 => MouseButton::Left,
                    2 => MouseButton::Middle,
                    3 => MouseButton::Right,
                    _ => { return; }
                };
                EVENT_AGGREGATOR.send(
                    UiCommand::Serial(SerialCommand::MouseButton {
                        action: MouseAction::Release,
                        button: btn,
                        modifier: c.current_event_state(),
                        grid_id: gridid,
                        position: (cols.floor() as u32, rows.floor() as u32)
                    })
                );
                trace!("grid {} release button {} current_button {} modifier {}", gridid, c.button(), c.current_button(), modifier);
            }),
        );
        view.add_controller(click_listener);

        let motion_listener = gtk::EventControllerMotion::new();
        let grid_id = id;
        motion_listener.connect_enter(move |_, _, _| {
            app::GridActived.store(grid_id, atomic::Ordering::Relaxed);
        });
        motion_listener.connect_motion(
            glib::clone!(@strong sender, @weak dragging, @weak metrics => move |c, x, y| {
                sender.output(app::AppMessage::ShowPointer).unwrap();
                trace!(gridid, "cursor motion {} {}", x, y);
                if let Some(Dragging { btn, pos }) = dragging.get() {
                    let metrics = metrics.get();
                    let width = metrics.width();
                    let height = metrics.height();
                    let cols = x as f64 / width;
                    let rows = y as f64 / height;
                    let position = (cols.floor() as u32, rows.floor() as u32);
                    trace!("Dragging {} from {:?} to {:?}", btn, pos, position);
                    if pos != position {
                        EVENT_AGGREGATOR.send(
                            UiCommand::Serial(SerialCommand::Drag {
                                button: btn,
                                modifier: c.current_event_state(),
                                grid_id: gridid,
                                position,
                            })
                        );
                        dragging.set(Dragging { btn, pos: position }.into());
                    }
                }
                // for mouse auto hide
                // if motion show one second.

            }),
        );
        view.add_controller(motion_listener);

        let model = VimGrid::new(
            id,
            winid,
            coord,
            rect,
            hldefs,
            dragging,
            metrics,
            font_description,
        );
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    // fn position(&self, _: &u64) -> FixedPosition {
    //     debug!("requesting position of grid {}", self.grid);
    //     self.pos.into()
    // }

    fn post_view() {
        debug!(
            grid = model.grid,
            "vim grid update pos {:?} size {}x{}", model.pos, model.width, model.height
        );

        view.set_font_description(&model.font_description.borrow());

        let p_width = view.property::<u64>("width") as usize;
        let p_height = view.property::<u64>("height") as usize;
        if model.width != p_width || model.height != p_height {
            view.resize(model.width as _, model.height as _);
        }

        view.set_float(model.is_float);
        view.set_focusable(model.focusable);

        // if let Some(pos) = model.move_to.take() {
        //     gtk::prelude::FixedExt::move_(
        //         &view.parent().unwrap().downcast::<gtk::Fixed>().unwrap(),
        //         view,
        //         pos.x,
        //         pos.y,
        //     );
        // }

        view.queue_allocate();
        view.queue_resize();
    }

    fn update(&mut self, event: Event, sender: ComponentSender<Self>) {
        match event {
            Event::Show => self.visible = true,
            Event::Clear => self.clear(),
            Event::Resize { width, height } => self.resize(width, height),
            Event::ResetCache => self.reset_cache(),
            Event::WindowHide => self.visible = false,
            Event::WindowClose => self.visible = false,
            Event::Scroll {
                top,
                bottom,
                left,
                right,
                rows,
                columns,
            } => {
                // FIXME:
            }
            Event::Float { focusable } => {
                self.set_is_float(true);
                self.set_focusable(focusable);
            }
        }
    }
}

#[derive(Debug)]
pub struct VimGridsWidgets {
    root: gtk::Fixed,
}

#[derive(Debug)]
pub struct VimGrids {
    grids: HashMap<u64, Controller<VimGrid>>,
    evnets: Vec<GridEvent>,

    widget: gtk::Fixed,
    pctx: pango::Context,
    metrics: Rc<Cell<crate::metrics::Metrics>>,
    font_description: Rc<RefCell<pango::FontDescription>>,
}

impl Component for VimGrids {
    type Input = GridEvent;
    type Output = AppMessage;
    type CommandOutput = ();

    type Widgets = VimGridsWidgets;

    type Root = gtk::Fixed;

    type Init = (
        pango::Context,
        Rc<Cell<crate::metrics::Metrics>>,
        Rc<RefCell<pango::FontDescription>>,
    );

    fn init_root() -> Self::Root {
        gtk::Fixed::builder()
            .name("vim-grids")
            .visible(true)
            .focusable(true)
            .focus_on_click(true)
            .build()
    }

    fn init(
        (pctx, metrics, font_description): Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let mut model = VimGrids {
            grids: HashMap::default(),
            evnets: Vec::new(),
            pctx,
            metrics,
            font_description,

            widget: <VimGrids as relm4::Component>::init_root(),
        };

        let widgets = VimGridsWidgets {
            root: model.widget.clone(),
        };

        ComponentParts { model, widgets }
    }

    fn update(&mut self, event: Self::Input, sender: ComponentSender<Self>, widget: &Self::Root) {
        warn!("event {:?}", event);
        match event {
            GridEvent::Flush => {
                //
            }
            GridEvent::Show(grid) => self
                .grids
                .get_mut(&grid)
                .unwrap()
                .sender()
                .send(Event::Show)
                .unwrap(),
            GridEvent::ResetCache => {
                self.grids
                    .iter()
                    .for_each(|(_, v)| v.sender().send(Event::ResetCache).unwrap());
            }
            GridEvent::Destroy(grid) => {
                let grid = self.grids.remove(&grid).unwrap();
                widget.remove(grid.widget())
            }
            GridEvent::Add {
                grid,
                win,
                coord,
                rectangle,
                hldefs,
                dragging,
            } => {
                let vgrid = VimGrid::builder()
                    .launch((
                        grid,
                        win,
                        coord.clone(),
                        rectangle,
                        hldefs,
                        dragging,
                        self.metrics.clone(),
                        self.font_description.clone(),
                    ))
                    .forward(sender.output_sender(), identity);
                let (x, y) = coord.to_physical(self.metrics.get());
                FixedExt::put(&self.widget, vgrid.widget(), x, y);
                self.grids.insert(grid, vgrid);
                debug!("after add {:?}", &self.grids);
            }
            GridEvent::Scroll {
                grid,
                top,
                bottom,
                left,
                right,
                rows,
                columns,
            } => {
                // FIXME:
            }
            GridEvent::FloatPosition {
                grid,
                coord,
                focusable,
            } => {
                let vgrid = self.grids.get(&grid).unwrap();
                vgrid.sender().send(Event::Float { focusable }).unwrap();
                let (x, y) = coord.to_physical(self.metrics.get());
                self.widget.move_(vgrid.widget(), x, y)
            }
            GridEvent::Resize {
                grid,
                width,
                height,
            } => {
                self.grids
                    .get(&grid)
                    .unwrap()
                    .sender()
                    .send(Event::Resize {
                        width: width as _,
                        height: height as _,
                    })
                    .unwrap();
            }
            GridEvent::WindowHide(grid) => {
                self.grids
                    .get(&grid)
                    .unwrap()
                    .sender()
                    .send(Event::WindowHide)
                    .unwrap();
            }
            GridEvent::WindowClose(grid) => {
                let vgrid = self.grids.get(&grid).unwrap();
                vgrid.sender().send(Event::WindowClose).unwrap();
                self.widget.remove(vgrid.widget())
            }
            GridEvent::AtPosition {
                grid,
                coord,
                rectangle,
            } => {
                let vgrid = self.grids.get(&grid).unwrap();
                vgrid
                    .sender()
                    .send(Event::Resize {
                        width: rectangle.width,
                        height: rectangle.height,
                    })
                    .unwrap();
                let (x, y) = coord.to_physical(self.metrics.get());
                self.widget.move_(vgrid.widget(), x, y)
            }
            GridEvent::Clear(grid) => {
                self.grids
                    .get(&grid)
                    .unwrap()
                    .sender()
                    .send(Event::Clear)
                    .unwrap();
            }
        }
        // FIXME: set pango_context for new created grid.
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        warn!("unhandled command output {:?}", message);
    }

    fn update_view(&self, widgets: &mut Self::Widgets, sender: ComponentSender<Self>) {
        warn!("update view");
    }
}

impl VimGrids {
    pub fn get(&self, k: u64) -> Option<std::cell::Ref<VimGrid>> {
        self.grids.get(&k).map(|v| v.model())
    }

    pub fn iter(&self) -> impl Iterator<Item = std::cell::Ref<VimGrid>> + '_ {
        self.grids.iter().map(|(_, v)| v.model())
    }
}
