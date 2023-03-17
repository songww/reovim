use std::cell::{Cell, RefCell};
use std::fmt::Debug;
use std::rc::Rc;
use std::sync::{atomic, RwLock};

use gtk::prelude::*;
use relm4::factory::{FactoryView, Position};
use relm4::prelude::*;
use tracing::{debug, trace};

use crate::app::{self, AppMessage, Dragging};
use crate::bridge::{MouseAction, MouseButton, SerialCommand, UiCommand};
use crate::event_aggregator::EVENT_AGGREGATOR;
use crate::grapheme::{Coord, Pos, Rectangle};


use super::gridview::VimGridView;
use super::TextBuf;

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

// impl Debug for VimGrid {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         f.debug_struct("VimGrid")
//             .field("win-id", self.win)
//             .field("grid-id", self.grid)
//             .field("pos", self.pos)
//             .field("coord", self.coord)
//             .field("move-to", &self.move_to)
//             .finish()
//     }
// }

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
        self.width = width;
        self.height = height;
        self.textbuf().borrow().resize(height, width);
    }

    pub fn set_coord(&mut self, col: f64, row: f64) {
        let metrics = self.metrics.get();
        let pos: Pos = (col * metrics.width(), row * metrics.height()).into();
        self.pos = pos;
        self.coord = Coord { col, row };
        self.move_to.replace(pos.into());
    }

    pub fn set_is_float(&mut self, is_float: bool) {
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

#[relm4::factory(pub)]
impl FactoryComponent for VimGrid {
    type Widgets = VimGridWidgets;
    type ParentWidget = crate::widgets::board::Board;
    type ParentInput = AppMessage;
    type CommandOutput = AppMessage;
    type Input = ();
    type Output = ();

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
        view = VimGridView::new(self.grid, self.width as _, self.height as _) {
            set_widget_name: &format!("vim-grid-{}-{}", self.win, self.grid),
            set_textbuf: self.textbuf.clone(),

            set_visible: self.visible,
            set_can_focus: true,
            set_focusable: true,
            set_focus_on_click: true,

            set_overflow: gtk::Overflow::Hidden,

            set_font_description: &self.font_description.borrow(),

            set_css_classes: &["vim-view-grid", &format!("vim-view-grid-{}", self.grid)],
        }
    }

    fn init_model(
        (id, winid, coord, rect, hldefs, dragging, metrics, font_description): Self::Init,
        _index: &DynamicIndex,
        _sender: FactorySender<Self>,
    ) -> Self {
        VimGrid::new(
            id,
            winid,
            coord,
            rect,
            hldefs,
            dragging,
            metrics,
            font_description,
        )
    }
    fn init_widgets(
        &mut self,
        _index: &DynamicIndex,
        view: &Self::Root,
        _returned_widget: &<Self::ParentWidget as FactoryView>::ReturnedWidget,
        sender: FactorySender<Self>,
    ) -> VimGridWidgets {
        let widgets = view_output!();

        let view = &widgets.view;

        let click_listener = gtk::GestureClick::builder()
            .button(0)
            .exclusive(false)
            .touch_only(false)
            .n_points(1)
            .name("click-listener")
            .build();
        let gridid = self.grid;
        click_listener.connect_pressed(
            glib::clone!(@strong sender, @weak self.dragging as dragging, @weak self.metrics as metrics => move |c, n_press, x, y| {
                sender.command_sender().send(app::AppMessage::ShowPointer).unwrap();
                let metrics = metrics.get();
                let width = metrics.width();
                let height = metrics.height();
                let cols = x as f64 / width;
                let rows = y as f64 / height;
                trace!("grid {} mouse pressed {} times at {}x{} -> {}x{}", gridid, n_press, x, y, cols, rows);
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
                trace!("grid {} release button {} current_button {} modifier {}", gridid, c.button(), c.current_button(), modifier);
            }),
        );
        click_listener.connect_released(
            glib::clone!(@strong sender, @weak self.dragging as dragging, @weak self.metrics as metrics => move |c, n_press, x, y| {
                sender.command_sender().send(app::AppMessage::ShowPointer);
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
        let grid_id = self.grid;
        motion_listener.connect_enter(move |_, _, _| {
            app::GridActived.store(grid_id, atomic::Ordering::Relaxed);
        });
        motion_listener.connect_motion(glib::clone!(@strong sender, @weak self.dragging as dragging, @weak self.metrics as metrics => move |c, x, y| {
            sender.command_sender().send(app::AppMessage::ShowPointer);
            trace!("cursor motion {} {}", x, y);
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

        }));
        view.add_controller(motion_listener);

        widgets
    }

    // fn position(&self, _: &u64) -> FixedPosition {
    //     debug!("requesting position of grid {}", self.grid);
    //     self.pos.into()
    // }

    fn post_view() {
        debug!(
            "vim grid {} update pos {:?} size {}x{}",
            self.grid, self.pos, self.width, self.height
        );
        let view = &widgets.view;

        view.set_visible(self.visible);
        view.set_font_description(&self.font_description.borrow());

        let p_width = view.property::<u64>("width") as usize;
        let p_height = view.property::<u64>("height") as usize;
        if self.width != p_width || self.height != p_height {
            view.resize(self.width as _, self.height as _);
        }

        view.set_focusable(self.focusable);
        view.set_is_float(self.is_float);

        if let Some(pos) = self.move_to.take() {
            gtk::prelude::FixedExt::move_(
                &view.parent().unwrap().downcast::<gtk::Fixed>().unwrap(),
                view,
                pos.x,
                pos.y,
            );
        }

        view.queue_allocate();
        view.queue_resize();
    }
}
