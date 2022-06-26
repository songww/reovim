use std::cell::{Cell, RefCell};
use std::collections::LinkedList;
use std::rc::Rc;
use std::sync::{atomic, Arc};

use adw::prelude::BinExt;
use gtk::prelude::*;
use parking_lot::RwLock;
use relm4::factory::positions::FixedPosition;
use relm4::*;

use crate::app::{self, Dragging};
use crate::bridge::{MouseAction, MouseButton, SerialCommand, UiCommand};
use crate::event_aggregator::EVENT_AGGREGATOR;
use crate::grapheme::{Clamp, Coord, Pos, Rectangle};
use crate::text::FontMap;

use super::gridview::VimGridView;
use super::TextBuf;

type HighlightDefinitions = Rc<RwLock<crate::vimview::HighlightDefinitions>>;

type Nr = usize;

#[derive(Debug)]
struct OldClamps {
    range: Option<(Nr, Nr)>,
    actived: Option<Clamp>,
    clamps: LinkedList<Clamp>,
}

pub struct VimGrid {
    grid: u64,
    pos: Pos,
    coord: Coord,
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
    // animation: Option<adw::TimedAnimation>,
    clamp: Option<Clamp>,
    // preview view port.
    old_clamps: Arc<RwLock<OldClamps>>,
}

impl VimGrid {
    pub fn new(
        id: u64,
        coord: Coord,
        rect: Rectangle,
        hldefs: HighlightDefinitions,
        dragging: Rc<Cell<Option<Dragging>>>,
        metrics: Rc<Cell<crate::metrics::Metrics>>,
        font_description: Rc<RefCell<pango::FontDescription>>,
    ) -> VimGrid {
        let textbuf = TextBuf::new(rect.height, rect.width);
        textbuf.set_hldefs(hldefs);
        textbuf.set_metrics(metrics.clone());
        let m = metrics.get();
        VimGrid {
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
            clamp: None,
            old_clamps: Arc::new(RwLock::new(OldClamps {
                range: None,
                actived: None,
                clamps: LinkedList::new(),
            })),
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
        self.textbuf().clear();
    }

    pub fn reset_cache(&mut self) {
        self.textbuf().reset_cache();
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
            self.coord,
            self.width,
            self.height
        );
        self.textbuf().up(rows);
    }

    // content go down, view go up, eat tail of rows.
    pub fn down(&mut self, rows: usize) {
        log::debug!("scroll-region {} rows moved down.", rows);
        log::debug!(
            "Origin Region {:?} {}x{}",
            self.coord,
            self.width,
            self.height
        );
        self.textbuf().down(rows);
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
        self.textbuf().resize(height, width);
    }

    pub fn set_coord(&mut self, col: f64, row: f64) {
        let metrics = self.metrics.get();
        let pos: Pos = (col * metrics.width(), row * metrics.height()).into();
        let move_to: FixedPosition = pos.into();
        self.pos = pos;
        self.coord = Coord { col, row };
        self.move_to.replace(move_to.into());
    }

    pub fn set_is_float(&mut self, is_float: bool) {
        self.is_float = is_float;
    }

    pub fn set_focusable(&mut self, focusable: bool) {
        self.focusable = focusable;
    }

    pub fn set_pango_context(&self, pctx: Rc<pango::Context>) {
        self.textbuf().set_pango_context(pctx);
    }

    pub fn set_fontmap(&self, fontmap: Rc<FontMap>) {
        self.textbuf().set_fontmap(fontmap);
    }

    /// visible top-line - bottom-line
    pub fn set_viewport(&mut self, top: f64, bottom: f64) {
        if let Some(true) = self.clamp.map(|c| c.top() == top && c.bottom() == bottom) {
            // dose not changed.
            log::debug!(
                "viewport dose not set {}-{} {:?}",
                top,
                bottom,
                self.old_clamps
            );
            return;
        }
        self.textbuf().set_viewport(top, bottom);
        let mut old_clamps = self.old_clamps.write();

        if let Some(clamp) = self.clamp.replace(Clamp::new(top, bottom)) {
            old_clamps.clamps.push_back(clamp);
        }
        if old_clamps.clamps.len() > 5 {
            old_clamps.clamps.pop_front();
        }
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct VimGridWidgets {
    root: adw::Bin,
    view: VimGridView,
    #[derivative(Debug = "ignore")]
    smoother: Cell<Option<gtk::TickCallbackId>>,
}

impl factory::FactoryPrototype for VimGrid {
    type Factory = crate::factory::FactoryMap<Self>;
    type Widgets = VimGridWidgets;
    type Root = adw::Bin;
    type View = gtk::Fixed;
    type Msg = app::AppMessage;

    fn init_view(&self, grid: &u64, sender: Sender<app::AppMessage>) -> VimGridWidgets {
        let grid = *grid;
        view! {
            view = VimGridView::new(grid, self.width as _, self.height as _) {
                set_widget_name: &format!("vim-grid-{}", grid),
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

        let m = self.metrics.get();
        let rows = self
            .clamp
            .map(|clamp| clamp.bottom() - clamp.top())
            .unwrap_or_else(|| self.height() as f64)
            .min(self.height() as f64);
        let height_request = (rows * m.height()).max(1.);
        let vadjustment = gtk::Adjustment::default();
        vadjustment.set_page_size(height_request as f64);

        log::info!(
            "creating {} scrolled window max-height: {}",
            grid,
            height_request
        );
        let content_height = if height_request > 1. {
            height_request as i32
        } else {
            1
        };
        let win = gtk::ScrolledWindow::builder()
            .child(&view)
            .has_frame(false)
            .vadjustment(&vadjustment)
            .vscrollbar_policy(gtk::PolicyType::External)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .kinetic_scrolling(false)
            .max_content_height(content_height)
            .min_content_height(content_height - 1)
            .propagate_natural_width(true)
            .build();
        let bin = adw::Bin::new();
        bin.set_child(Some(&win));

        log::error!(
            "grid {} maximum_size ---------------------------> {}",
            self.grid,
            height_request,
        );
        {
            // Patch: disable scroll-controllers
            let controllers = win.observe_controllers();
            let n = controllers.n_items();
            let mut position = 0;
            while position < n {
                let object = controllers.item(position).unwrap();
                let controller = object.downcast_ref::<gtk::EventController>().unwrap();
                controller.set_propagation_phase(gtk::PropagationPhase::None);
                position += 1;
            }
        }
        // let target = adw::CallbackAnimationTarget::new(Some(Box::new(
        //     glib::clone!(@weak view, @weak vadjustment => move |value: f64| {
        //         log::warn!("callback smooth animation: {:.3}", value);
        //         vadjustment.set_value(value);
        //         view.queue_draw();
        //         view.queue_resize();
        //     }),
        // )));

        let click_listener = gtk::GestureClick::builder()
            .button(0)
            .exclusive(false)
            .touch_only(false)
            .n_points(1)
            .name("click-listener")
            .build();
        click_listener.connect_pressed(
            glib::clone!(@strong sender, @weak self.dragging as dragging, @weak self.metrics as metrics => move |c, n_press, x, y| {
                sender.send(app::AppMessage::ShowPointer).unwrap();
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
            glib::clone!(@strong sender, @weak self.dragging as dragging, @weak self.metrics as metrics => move |c, n_press, x, y| {
                sender.send(app::AppMessage::ShowPointer).unwrap();
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
        motion_listener.connect_motion(glib::clone!(@strong sender, @weak self.dragging as dragging, @weak self.metrics as metrics => move |c, x, y| {
            sender.send(app::AppMessage::ShowPointer).unwrap();
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
            // for mouse auto hide
            // if motion show one second.

        }));
        view.add_controller(&motion_listener);

        VimGridWidgets {
            root: bin,
            view,
            smoother: Cell::new(None),
        }
    }

    fn position(&self, _: &u64) -> FixedPosition {
        log::debug!("requesting position of grid {}", self.grid);
        self.pos.into()
    }

    fn view(&self, index: &u64, widgets: &VimGridWidgets) {
        log::info!(
            "vim grid {} update pos {:?} size {}x{}",
            index,
            self.pos,
            self.width,
            self.height
        );

        let view = &widgets.view;

        view.set_visible(self.visible);
        view.set_font_description(&self.font_description.borrow());

        self.smoothed(widgets);

        let p_width = view.property::<u64>("width") as usize;
        let p_height = view.property::<u64>("height") as usize;
        if self.width != p_width || self.height != p_height {
            let metrics = self.metrics.get();
            let height_request = self.height as f64 * metrics.height();
            log::info!("resizing scrolled window max-height: {}", height_request);
            widgets.root.child().map(|child| {
                let content_height = if height_request > 1. {
                    height_request as i32
                } else {
                    1
                };
                let win = child.downcast_ref::<gtk::ScrolledWindow>().unwrap();
                win.set_max_content_height(content_height);
                win.set_min_content_height(content_height - 1);
            });

            log::error!(
                "grid {} maximum_size ---------------------------> {}",
                self.grid,
                height_request
            );
            view.resize(self.width as _, self.height as _);
            view.queue_resize();
        }

        view.set_focusable(self.focusable);
        view.set_is_float(self.is_float);

        if let Some(pos) = self.move_to.take() {
            gtk::prelude::FixedExt::move_(
                widgets
                    .root
                    .parent()
                    .unwrap()
                    .downcast_ref::<gtk::Fixed>()
                    .unwrap(),
                &widgets.root,
                pos.x,
                pos.y,
            );
        }

        widgets.view.queue_allocate();
        widgets.view.queue_resize();
        widgets.view.queue_draw();
        widgets.root.queue_allocate();
        widgets.root.queue_resize();
        widgets.root.queue_draw();
    }

    fn root_widget(widgets: &VimGridWidgets) -> &adw::Bin {
        &widgets.root
    }
}

impl VimGrid {
    fn smoothed(&self, widgets: &VimGridWidgets) {
        let metrics = self.metrics.get();
        let old_clamps = self.old_clamps.read();
        if old_clamps.clamps.is_empty() {
            log::error!("grid {} empty clamps.", self.grid);
            // 还没有配置viewport
            return;
        }
        // last old clamp is actived, means no new clamps configured.
        // Animation already running.
        let clamp = *old_clamps.clamps.back().unwrap();
        if let Some(true) = old_clamps.actived.map(|ref c| *c == clamp) {
            log::error!("grid {} same clamps.", self.grid);
            // dose not changed, do nothing.
            return;
        }
        let textbuf = &self.textbuf;
        let lines = textbuf.lines();

        // let top = if let Some(ref actived) = old_clamps.actived {
        //     log::error!("current actived: {:?} clamps: {:?}", actived, old_clamps);
        //     let position = old_clamps
        //         .clamps
        //         .iter()
        //         .position(|clamp| clamp == actived)
        //         .unwrap();
        //     assert!(old_clamps.clamps.iter().take(position + 1).last().unwrap() == actived);
        //     let thetop = old_clamps
        //         .clamps
        //         .iter()
        //         .take(position + 1)
        //         .fold(f64::MAX, |b, c| if c.top() < b { c.top() } else { b });
        //     // 动画正在运行中, 此时需计算之前的位置?
        //     let value = vadjustment.value();
        //     let (from, to) = old_clamps.range.unwrap();

        //     // current nr scrolled to.
        //     let index = (value / metrics.height() - thetop).floor() as usize;
        //     if from < to {
        //         // assert!((index + from) <= to, "({} + {}) <= {}", index, from, to);
        //         (index + from).min(to) as f64
        //     } else {
        //         // from > to
        //         // assert!((index + to) <= from, "({} + {}) <= {}", index, to, from);
        //         (index + to).min(from) as f64
        //     }
        // } else {
        //     // animation dose not started yet.
        //     old_clamps.clamps.front().unwrap().top()
        // };
        // let vadjustment = widgets.viewport.vadjustment();

        let top = old_clamps.clamps.front().unwrap().top();

        let mut topidx = None;
        let mut ctopidx = None;
        // where from
        let topu = top.floor() as usize;
        // where to
        let ctopu = self
            .clamp
            .map(|clamp| clamp.top().floor() as usize)
            .unwrap();
        let mut nrs = Vec::with_capacity(self.height() * 2);
        for (relidx, line) in lines.iter().enumerate() {
            nrs.push(line.nr());
            if line.nr() == topu {
                topidx.replace(relidx);
            }
            if line.nr() == ctopu {
                ctopidx.replace(relidx);
            }
            if topidx.is_some() && ctopidx.is_some() {
                break;
            }
        }

        if topidx == ctopidx {
            return;
        }

        log::error!(
            "grid {} topu: {} ctopu {} topidx: {:?} ctopidx {:?}",
            self.grid,
            topu,
            ctopu,
            topidx,
            ctopidx
        );
        log::error!(
            "grid {} current clamp {:?} old-clamps {:?}",
            self.grid,
            self.clamp,
            old_clamps
        );
        log::error!("grid {} nrs: {:?}", self.grid, nrs);
        let topidx = topidx.unwrap_or(0);
        let ctopidx = ctopidx.unwrap();

        let value_from = topidx as f64 * metrics.height();
        let value_to = ctopidx as f64 * metrics.height();

        log::info!(
            "grid {} viewport {:?} animation from {} to {}",
            self.grid,
            self.clamp.unwrap(),
            value_from,
            value_to
        );

        if let Some(handle) = widgets.smoother.take() {
            handle.remove()
        };

        let frame_clock = widgets.root.frame_clock().unwrap();
        frame_clock.begin_updating();
        let startat = frame_clock.frame_time();
        let handle = widgets.root.add_tick_callback(glib::clone!(@strong self.textbuf as textbuf, @weak widgets.view as view, @weak self.old_clamps as old_clamps => @default-return glib::Continue(false), move |root, frame_clock| {
            // 40951.023317
            log::error!("frame_time {}", frame_clock.frame_time());
            let child = root.child().unwrap();
            let win = child.downcast_ref::<gtk::ScrolledWindow>().unwrap();
            let vadjustment = win.vadjustment();
            // TODO: current smooth duration is 150ms, should be configurable.
            let ratio = (frame_clock.frame_time() - startat) as f64 / 150000.;
            if ratio >= 1. {
                frame_clock.end_updating();
                vadjustment.set_value(value_to);

                std::thread::spawn({
                    let textbuf = textbuf.clone();
                    let old_clamps = old_clamps.clone();
                    move || {
                        textbuf.discard();
                        let mut old_clamps = old_clamps.write();
                        old_clamps.range.take();
                        old_clamps.clamps.clear();
                        old_clamps.actived.take();
                        log::error!("-------------------------------> animation done cleared.");
                    }
                });
                return glib::Continue(false);
            }

            let value = (value_to - value_from) * ratio + value_from;
            vadjustment.set_value(value);
            log::debug!("changed value to: {}", value);
            log::debug!("vadjustment-value: {}", vadjustment.value());
            log::debug!("vadjustment-lower: {}", vadjustment.lower());
            log::debug!("vadjustment-upper: {}", vadjustment.upper());
            log::debug!("vadjustment-page-size: {}", vadjustment.page_size());
            log::debug!("textbuf height: {}", view.height());
            win.queue_draw();
            glib::Continue(true)
        }));
        widgets.smoother.set(Some(handle));
        log::debug!(
            "grid {} -------------------------------> animation start",
            self.grid
        );
        drop(old_clamps);
        let mut old_clamps = self.old_clamps.write();
        old_clamps.actived.replace(clamp);
        old_clamps.range.replace((topu, ctopu));
    }
}

/*
fn smoothed(view: &adw::ClampScrollable, fc: &gdk::FrameClock) -> glib::Continue {
    adw::Easing::EaseOutQuart;
    glib::clone!(@strong sender, @strong self.textbuf as textbuf, @weak view, @weak self.old_clamps as old_clamps => move |_this| {
        // smooth scrolling done.
        std::thread::spawn({
            let textbuf = textbuf.clone();
            let old_clamps = old_clamps.clone();
            move || {
                textbuf.discard();
                let mut old_clamps = old_clamps.write();
                old_clamps.range.take();
                old_clamps.clamps.clear();
                old_clamps.actived.take();
                log::error!("-------------------------------> animation done cleared.");
            }
        }).join().unwrap();
        log::error!("-------------------------------> animation done");
        view.queue_draw();
        view.queue_resize();
        // let widget = this.widget();
        // let sw = widget.downcast_ref::<gtk::ScrolledWindow>().unwrap();
        // sw.vadjustment().set_value(0.);
    });
    glib::Continue(true);
}
*/
