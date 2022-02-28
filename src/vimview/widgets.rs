use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic;

use gtk::prelude::*;
use parking_lot::RwLock;
use relm4::factory::positions::FixedPosition;
use relm4::*;

use crate::app::{self, Dragging};
use crate::bridge::{MouseAction, MouseButton, SerialCommand, UiCommand};
use crate::event_aggregator::EVENT_AGGREGATOR;
use crate::pos::Position;
use crate::rect::Rectangle;

use super::gridview::VimGridView;
use super::TextBuf;

type HighlightDefinitions = Rc<RwLock<crate::vimview::HighlightDefinitions>>;

pub struct VimGrid {
    win: u64,
    grid: u64,
    pos: FixedPosition,
    move_to: Cell<Option<FixedPosition>>,
    width: usize,
    height: usize,
    is_float: bool,
    focusable: bool,
    metrics: Rc<Cell<crate::metrics::Metrics>>,
    font_description: Rc<RefCell<pango::FontDescription>>,
    dragging: Rc<Cell<Option<Dragging>>>,

    textbuf: TextBuf,

    visible: bool,
}

impl VimGrid {
    pub fn new(
        id: u64,
        winid: u64,
        pos: Position,
        rect: Rectangle,
        hldefs: HighlightDefinitions,
        dragging: Rc<Cell<Option<Dragging>>>,
        metrics: Rc<Cell<crate::metrics::Metrics>>,
        font_description: Rc<RefCell<pango::FontDescription>>,
    ) -> VimGrid {
        let textbuf = TextBuf::new(rect.height, rect.width);
        textbuf.borrow().set_hldefs(hldefs.clone());
        textbuf.borrow().set_metrics(metrics.clone());
        VimGrid {
            win: winid,
            grid: id,
            pos: pos.into(),
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

    pub fn pos(&self) -> &FixedPosition {
        &self.pos
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
        log::debug!("scroll-region {} rows moved up.", rows);
        log::debug!(
            "Origin Region {:?} {}x{}",
            self.pos,
            self.width,
            self.height
        );
        self.textbuf().borrow_mut().up(rows);
    }

    // content go down, view go up, eat tail of rows.
    pub fn down(&mut self, rows: usize) {
        log::debug!("scroll-region {} rows moved down.", rows);
        log::debug!(
            "Origin Region {:?} {}x{}",
            self.pos,
            self.width,
            self.height
        );
        self.textbuf().borrow_mut().down(rows);
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
        self.textbuf().borrow().resize(height, width);
    }

    pub fn set_pos(&mut self, x: f64, y: f64) {
        self.pos = FixedPosition { x, y };
        self.move_to.replace(FixedPosition { x, y }.into());
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

#[derive(Debug)]
pub struct VimGridWidgets {
    view: VimGridView,
}

impl factory::FactoryPrototype for VimGrid {
    type Factory = crate::factory::FactoryMap<Self>;
    type Widgets = VimGridWidgets;
    type Root = VimGridView;
    type View = gtk::Fixed;
    type Msg = app::AppMessage;

    fn init_view(&self, grid: &u64, sender: Sender<app::AppMessage>) -> VimGridWidgets {
        let grid = *grid;
        view! {
            view = VimGridView::new(grid, self.width as _, self.height as _) {
                set_widget_name: &format!("vim-grid-{}-{}", self.win, grid),
                set_textbuf: self.textbuf.clone(),

                set_visible: self.visible,
                set_can_focus: true,
                set_focusable: true,
                set_focus_on_click: true,

                set_overflow: gtk::Overflow::Hidden,

                set_font_description: &self.font_description.borrow(),

                set_css_classes: &["vim-view-grid", &format!("vim-view-grid-{}", self.grid)],

                // inline_css: b"border: 1px solid @borders;",
            }
        }

        let click_listener = gtk::GestureClick::builder()
            .button(0)
            .exclusive(false)
            .touch_only(false)
            .n_points(1)
            .name("click-listener")
            .build();
        click_listener.connect_pressed(
            glib::clone!(@strong sender, @strong self.dragging as dragging, @strong self.metrics as metrics => move |c, n_press, x, y| {
                let metrics = metrics.get();
                let width = metrics.width();
                let height = metrics.height();
                let cols = x as f64 / width;
                let rows = y as f64 / height;
                log::trace!("grid {} mouse pressed {} times at {}x{} -> {}x{}", grid, n_press, x, y, cols, rows);
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
                        grid_id: grid,
                        position
                    })
                );
                log::trace!("grid {} release button {} current_button {} modifier {}", grid, c.button(), c.current_button(), modifier);
            }),
        );
        click_listener.connect_released(
            glib::clone!(@strong sender, @strong self.dragging as dragging, @strong self.metrics as metrics => move |c, n_press, x, y| {
                let metrics = metrics.get();
                let width = metrics.width();
                let height = metrics.height();
                let cols = x as f64 / width;
                let rows = y as f64 / height;
                log::trace!("grid {} mouse released {} times at {}x{} -> {}x{}", grid, n_press, x, y, cols, rows);
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
                        grid_id: grid,
                        position: (cols.floor() as u32, rows.floor() as u32)
                    })
                );
                log::trace!("grid {} release button {} current_button {} modifier {}", grid, c.button(), c.current_button(), modifier);
            }),
        );
        view.add_controller(&click_listener);

        let motion_listener = gtk::EventControllerMotion::new();
        let grid_id = grid;
        motion_listener.connect_enter(move |_, _, _| {
            app::GridActived.store(grid_id, atomic::Ordering::Relaxed);
        });
        motion_listener.connect_motion(glib::clone!(@strong sender, @strong self.dragging as dragging, @strong self.metrics as metrics => move |c, x, y| {
            log::trace!("cursor motion {} {}", x, y);
            if let Some(Dragging { btn, pos }) = dragging.get() {
                let metrics = metrics.get();
                let width = metrics.width();
                let height = metrics.height();
                let cols = x as f64 / width;
                let rows = y as f64 / height;
                let position = (cols.floor() as u32, rows.floor() as u32);
                log::trace!("Dragging {} from {:?} to {:?}", btn, pos, position);
                if pos != position {
                    EVENT_AGGREGATOR.send(
                        UiCommand::Serial(SerialCommand::Drag {
                            button: btn,
                            modifier: c.current_event_state(),
                            grid_id: grid,
                            position,
                        })
                    );
                    dragging.set(Dragging { btn, pos: position }.into());
                }
            }

        }));
        view.add_controller(&motion_listener);

        VimGridWidgets { view }
    }

    fn position(&self, _: &u64) -> FixedPosition {
        log::debug!("requesting position of grid {}", self.grid);
        FixedPosition {
            x: self.pos.x,
            y: self.pos.y,
        }
    }

    fn view(&self, index: &u64, widgets: &VimGridWidgets) {
        log::debug!(
            "vim grid {} update pos {:?} size {}x{}",
            index,
            self.pos,
            self.width,
            self.height
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

    fn root_widget(widgets: &VimGridWidgets) -> &VimGridView {
        &widgets.view
    }
}
