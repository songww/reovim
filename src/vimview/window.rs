use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::{atomic, RwLock};

use glib::subclass::prelude::*;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use relm4::factory::positions::FixedPosition;
use relm4::*;

use crate::app;
use crate::bridge::UiCommand;

use super::grid::VimGridView;
use super::TextBuf;

type HighlightDefinitions = Rc<RwLock<crate::vimview::HighlightDefinitions>>;

// #[derive(Debug)]
pub struct VimGrid {
    win: u64,
    grid: u64,
    move_to: Cell<Option<FixedPosition>>,
    pos: FixedPosition,
    width: usize,
    height: usize,
    hldefs: HighlightDefinitions,
    flush: Rc<atomic::AtomicBool>,
    metrics: Rc<Cell<app::FontMetrics>>,
    font_description: Rc<RefCell<pango::FontDescription>>,

    textbuf: TextBuf,

    visible: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

impl Position {
    pub fn new(x: f64, y: f64) -> Position {
        Position { x, y }
    }
}

impl Into<FixedPosition> for Position {
    fn into(self) -> FixedPosition {
        FixedPosition {
            x: self.x,
            y: self.y,
        }
    }
}

impl From<(f64, f64)> for Position {
    fn from((x, y): (f64, f64)) -> Self {
        Position { x, y }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Rectangle {
    pub width: usize,
    pub height: usize,
}

impl Rectangle {
    fn new(width: usize, height: usize) -> Rectangle {
        Rectangle { width, height }
    }
}

impl From<(usize, usize)> for Rectangle {
    fn from((width, height): (usize, usize)) -> Self {
        Rectangle { width, height }
    }
}

impl From<(u64, u64)> for Rectangle {
    fn from((width, height): (u64, u64)) -> Self {
        Rectangle {
            width: width as usize,
            height: height as usize,
        }
    }
}

impl VimGrid {
    pub fn new(
        id: u64,
        winid: u64,
        pos: Position,
        rect: Rectangle,
        flush: Rc<atomic::AtomicBool>,
        hldefs: HighlightDefinitions,
        metrics: Rc<Cell<app::FontMetrics>>,
        font_description: Rc<RefCell<pango::FontDescription>>,
    ) -> VimGrid {
        let textbuf = TextBuf::new(rect.height, rect.width);
        VimGrid {
            win: winid,
            grid: id,
            pos: pos.into(),
            width: rect.width as _,
            height: rect.height as _,
            move_to: None.into(),
            hldefs: hldefs.clone(),
            flush,
            metrics,
            textbuf,
            visible: true,
            font_description,
        }
    }
    pub fn textbuf(&self) -> &TextBuf {
        &self.textbuf
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

    pub fn resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
        self.textbuf.borrow().resize(height, width);
    }

    pub fn set_pos(&mut self, x: f64, y: f64) {
        self.pos = FixedPosition { x, y };
        self.move_to.replace(FixedPosition { x, y }.into());
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
        view! {
            view = VimGridView::new(*grid, self.width as _, self.height as _) {
                set_widget_name: &format!("vim-grid-{}-{}", self.win, grid),
                set_hldefs: self.hldefs.clone(),
                set_textbuf:self.textbuf.clone(),
                set_font_metrics: self.metrics.clone(),

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
            glib::clone!(@strong sender, @strong grid, @strong self.metrics as metrics => move |_, n_press, x, y| {
                let metrics = metrics.get();
                let charwidth = metrics.charwidth();
                let lineheight = metrics.lineheight() + metrics.linespace();
                let cols = x as f64 / charwidth;
                let rows = y as f64 / lineheight;
                log::info!("grid {} mouse pressed {} times at {}x{} -> {}x{}", grid, n_press, x, y, cols, rows);
                sender.send(
                    UiCommand::MouseButton {
                        action: "press".to_string(),
                        grid_id: grid,
                        position: (cols.floor() as u32, rows.floor() as u32)
                    }.into()
                ).expect("Failed to send mouse press event");
            }),
        );
        click_listener.connect_released(
            glib::clone!(@strong sender, @strong grid, @strong self.metrics as metrics => move |_, n_press, x, y| {
                let metrics = metrics.get();
                let charwidth = metrics.charwidth();
                let lineheight = metrics.lineheight() + metrics.linespace();
                let cols = x as f64 / charwidth;
                let rows = y as f64 / lineheight;
                log::info!("grid {} mouse released {} times at {}x{} -> {}x{}", grid, n_press, x, y, cols, rows);
                sender.send(
                    UiCommand::MouseButton {
                        action: "release".to_string(),
                        grid_id: grid,
                        position: (cols.floor() as u32, rows.floor() as u32)
                    }.into()
                ).expect("Failed to send mouse event");
            }),
        );
        view.add_controller(&click_listener);

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

        if let Some(pos) = self.move_to.take() {
            gtk::prelude::FixedExt::move_(
                &view.parent().unwrap().downcast::<gtk::Fixed>().unwrap(),
                view,
                pos.x,
                pos.y,
            );
        }

        // if let Ok(true) = self.flush.compare_exchange(
        //     true,
        //     false,
        //     atomic::Ordering::Acquire,
        //     atomic::Ordering::Relaxed,
        // ) {
        view.queue_allocate();
        view.queue_resize();
        view.queue_draw();
        // }
    }

    fn root_widget(widgets: &VimGridWidgets) -> &VimGridView {
        &widgets.view
    }
}
