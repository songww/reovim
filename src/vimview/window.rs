use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::{atomic, RwLock};

use gdk::Rectangle;
use glib::subclass::prelude::*;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use once_cell::sync::OnceCell;
use relm4::factory::positions::FixedPosition;
use relm4::*;
use rustc_hash::FxHashMap as HashMap;

use crate::app;
use crate::bridge::UiCommand;
use crate::cloned;
use crate::color::{Color, ColorExt};

use super::grid::VimGridView;
use super::TextBuf;

type HighlightDefinitions = Rc<RwLock<crate::vimview::HighlightDefinitions>>;

mod imp {
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;

    // Object holding the state
    #[derive(Debug)]
    pub struct VimWindowView {
        grid: u64,
        window: u64,
        rect: gdk::Rectangle,
    }

    impl Default for VimWindowView {
        fn default() -> Self {
            VimWindowView {
                rect: gdk::Rectangle::new(0, 0, 0, 0),
                grid: 0,
                // grids: Vec::new(),
                window: 0,
            }
        }
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for VimWindowView {
        const NAME: &'static str = "VimWindow";
        type Type = super::VimWindowView;
        type ParentType = gtk::Fixed;
    }

    // Trait shared by all GObjects
    impl ObjectImpl for VimWindowView {
        // fn constructed(&self, obj: &Self::Type) {
        //     self.parent_constructed(obj);
        //     obj.set_label(&self.number.get().to_string());
        // }
    }

    // Trait shared by all widgets
    impl WidgetImpl for VimWindowView {
        fn snapshot(&self, widget: &Self::Type, snapshot: &gtk::Snapshot) {
            // let rect = graphene::Rect::new(0., 0., 100., 100.);
            // let cr = snapshot.append_cairo(&rect);
            // let pctx = widget.pango_context();
            // let layout = self.doc.as_ref().unwrap().layout(pctx);
            // pangocairo::show_layout(&cr, &layout);
            log::info!(
                "font description of window {} {:?}",
                self.window,
                widget.pango_context().font_description().unwrap().to_str()
            );
            self.parent_snapshot(widget, snapshot);
        }
    }

    impl FixedImpl for VimWindowView {}
}

glib::wrapper! {
    pub struct VimWindowView(ObjectSubclass<imp::VimWindowView>)
        @extends gtk::Widget, gtk::Fixed,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl VimWindowView {
    pub fn new(window: u64, grid: u64, rect: Rectangle) -> VimWindowView {
        // glib::Object::new(&[("window", &window), ("grid", &grid), ("rect", &rect)])
        glib::Object::new(&[]).expect("Failed to create `VimWindowView`.")
    }

    fn imp(&self) -> &imp::VimWindowView {
        imp::VimWindowView::from_instance(self)
    }

    pub fn set_font_description(&self, desc: &pango::FontDescription) {
        self.pango_context().set_font_description(desc)
    }
}

#[derive(Debug)]
pub struct VimWindowWidgets {
    view: VimWindowView,
}

#[derive(Copy, Clone, Debug)]
enum Visible {
    No,
    Yes,
}

impl Default for Visible {
    fn default() -> Self {
        Visible::Yes
    }
}

#[derive(Debug)]
pub struct VimGrid {
    win: u64,
    grid: u64,
    linespace: u64,
    charwidth: f64,
    lineheight: f64,
    rect: gdk::Rectangle,
    hldefs: HighlightDefinitions,
    font_description: Rc<RefCell<pango::FontDescription>>,

    textbuf: TextBuf,

    visible: bool,
}

#[derive(Debug)]
pub struct VimGridWidgets {
    view: VimGridView,
}

impl factory::FactoryPrototype for VimGrid {
    type Factory = crate::factory::FactoryMap<Self>;
    type Widgets = VimGridWidgets;
    type Root = VimGridView;
    type View = VimWindowView;
    type Msg = app::AppMessage;

    fn init_view(&self, grid: &u64, _sender: Sender<app::AppMessage>) -> VimGridWidgets {
        view! {
            view = VimGridView::new(*grid, self.rect.width() as _, self.rect.height() as _) {
                set_widget_name: &format!("grid-{}/{}", self.win, grid),
                set_hldefs: self.hldefs.clone(),
                set_textbuf:self.textbuf.as_ref().clone(),

                set_visible: watch!(self.visible),

                // set_size_request: watch!(self.rect.width() as _, self.rect.height() as _),
            }
        }

        let font_description = self.font_description.clone();
        view.add_tick_callback(move |view, _| {
            if let Some(desc) = view.pango_context().font_description() {
                let font_desc = unsafe { &*font_description.as_ref().as_ptr() }.clone();
                if desc != font_desc {
                    view.pango_context().set_font_description(&font_desc);
                }
            }
            glib::Continue(true)
        });

        VimGridWidgets { view }
    }

    fn position(&self, _grid: &u64) -> FixedPosition {
        log::info!("requesting grid position {}", _grid);
        FixedPosition {
            x: self.rect.x() as f64,
            y: self.rect.y() as f64,
        }
    }

    fn view(&self, index: &u64, widgets: &VimGridWidgets) {
        log::warn!("vim grid update {} {:?}", index, self.rect);
        let grid = &widgets.view;
        gtk::prelude::FixedExt::move_(
            &grid.parent().unwrap().downcast::<gtk::Fixed>().unwrap(),
            grid,
            self.rect.x() as _,
            self.rect.y() as _,
        );
        grid.queue_draw();
        grid.queue_resize();
        grid.queue_allocate();
    }

    fn root_widget(widgets: &VimGridWidgets) -> &VimGridView {
        &widgets.view
    }
}

impl VimGrid {
    pub fn textbuf(&self) -> &TextBuf {
        &self.textbuf
    }

    pub fn hide(&mut self) {
        // self.visible.set(Some(Visible::No));
        self.visible = false;
    }

    pub fn show(&mut self) {
        // self.visible.set(Some(Visible::Yes));
        self.visible = true;
    }

    pub fn resize(&mut self, width: u64, height: u64) {
        self.rect.set_width(width as _);
        self.rect.set_height(height as _);
        self.textbuf.borrow().resize(width as _, height as _);
    }

    pub fn set_pos(&mut self, x: u64, y: u64) {
        self.rect.set_x(x as i32);
        self.rect.set_y(y as i32);
    }

    /// (width, height)
    fn size_required(&self, cols: u64, rows: u64) -> (u64, u64) {
        (
            (cols as f64 * self.charwidth) as u64,
            (rows as f64 * self.lineheight) as u64,
        )
    }
}

impl<Widget> relm4::factory::FactoryView<Widget> for VimWindowView
where
    Widget: glib::IsA<gtk::Widget>,
{
    type Position = FixedPosition;
    type Root = Widget;

    fn add(&self, widget: &Widget, position: &FixedPosition) -> Widget {
        gtk::prelude::FixedExt::put(self, widget, position.x, position.y);
        widget.clone()
    }

    fn remove(&self, widget: &Widget) {
        gtk::prelude::FixedExt::remove(self, widget);
    }
}

impl factory::FactoryPrototype for VimWindow {
    type Factory = crate::factory::FactoryMap<Self>;
    type Widgets = VimWindowWidgets;
    type Root = VimWindowView;
    type View = gtk::Fixed;
    type Msg = app::AppMessage;

    fn init_view(&self, grid: &u64, sender: Sender<app::AppMessage>) -> VimWindowWidgets {
        view! {
            view = VimWindowView::new(self.id, *grid, self.rect) {
                set_visible: watch!(self.visible),
                set_widget_name: &format!("win-{}-{}", self.id, grid),

                factory!(self.grids),
            }
        }
        // relm4::factory::Factory::generate(&self.grids, &view, sender.clone());

        // let grid_id = self.grid;
        let on_click = gtk::GestureClick::new();
        on_click.set_name("vim-window-onclick-listener");
        on_click.connect_pressed(cloned!(sender, grid => move |c, n_press, x, y| {
            log::debug!("{:?} pressed {} times at {}x{}", c.name(), n_press, x, y);
            sender.send(
                UiCommand::MouseButton {
                    action: "press".to_string(),
                    grid_id: grid,
                    position: (x as u32, y as u32)
                }.into()
            ).expect("Failed to send mouse press event");
        }));
        on_click.connect_released(cloned!(sender, grid => move |c, n_press, x, y| {
            log::debug!("{:?} released {} times at {}x{}", c.name(), n_press, x, y);
            sender.send(
                UiCommand::MouseButton {
                    action: "release".to_string(),
                    grid_id: grid,
                    position: (x as u32, y as u32)
                }.into()
            ).expect("Failed to send mouse event");
        }));
        // on_click.connect_stopped(cloned!(sender => move |c| {
        //     log::debug!("Click({}) stopped", c.name().unwrap());
        //     // sender.send(AppMsg::Message(format!("Click({}) stopped", c.name().unwrap()))).unwrap();
        // }));
        // on_click.connect_unpaired_release(cloned!(sender => move |c, n_press, x, y, events| {
        //     // sender.send(AppMsg::Message(format!("Click({:?}) unpaired release {} times at {}x{} {:?}", c.group(), n_press, x, y, events))).unwrap();
        //     log::debug!("Click({:?}) unpaired release {} times at {}x{} {:?}", c.group(), n_press, x, y, events);
        // }));
        view.add_controller(&on_click);

        let on_scroll = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::all());
        on_scroll.set_name("scroller");
        on_scroll.connect_decelerate(cloned!(sender => move |_, x, y| {
            println!("scroll decelerate x: {}, y: {}", x, y);
            // UiCommand::Scroll { direction:  }
            // sender.send(AppMsg::Message(format!("scroll decelerate x: {}, y: {}", x, y))).unwrap();
        }));
        on_scroll.connect_scroll(cloned!(sender => move |_, x, y| {
            println!("scroll x: {}, y: {}", x, y);
            // sender.send(AppMsg::Message(format!("scroll x: {}, y: {}", x, y))).unwrap();
            // sender.send(UiCommand::Scroll { grid_id: grid_id,  }).unwrap();
            gtk::Inhibit(false)
        }));
        on_scroll.connect_scroll_begin(cloned!(sender => move |_| {
            println!("scroll begin");
            // sender.send(AppMsg::Message(format!("scroll begin"))).unwrap();
        }));
        on_scroll.connect_scroll_end(cloned!(sender => move |_| {
            println!("scroll end");
            // sender.send(AppMsg::Message(format!("scroll end"))).unwrap();
        }));
        view.add_controller(&on_scroll);

        let font_description = self.font_description.clone();
        view.add_tick_callback(move |view, _| {
            if let Some(desc) = view.pango_context().font_description() {
                let font_desc = unsafe { &*font_description.as_ref().as_ptr() }.clone();
                if desc != font_desc {
                    view.pango_context().set_font_description(&font_desc);
                }
            }
            glib::Continue(true)
        });

        VimWindowWidgets { view }
    }

    fn position(&self, _grid: &u64) -> FixedPosition {
        log::info!("requesting window position {}", _grid);
        FixedPosition {
            x: self.rect.x() as f64,
            y: self.rect.y() as f64,
        }
    }

    fn view(&self, index: &u64, widgets: &VimWindowWidgets) {
        // log::warn!("vim window update {} {:?}", index, self.rect);
        let view = &widgets.view;

        gtk::prelude::FixedExt::move_(
            &view.parent().unwrap().downcast::<gtk::Fixed>().unwrap(),
            view,
            self.rect.x() as _,
            self.rect.y() as _,
        );
        view.queue_draw();
        view.queue_resize();
        view.queue_allocate();
    }

    fn root_widget(widgets: &VimWindowWidgets) -> &VimWindowView {
        &widgets.view
    }
}

// #[derive(Clone)]
pub struct VimWindow {
    id: u64,
    grid: u64,
    linespace: u64,
    charwidth: f64,
    lineheight: f64,
    rect: gdk::Rectangle,
    hldefs: HighlightDefinitions,
    font_description: Rc<RefCell<pango::FontDescription>>,

    visible: bool,
    queued_draw: bool,

    grids: crate::factory::FactoryMap<VimGrid>,
}

impl std::fmt::Debug for VimWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VimWindow")
            .field("id", &self.id)
            .field("grid", &self.grid)
            .field("rect", &self.rect)
            .finish()
    }
}

impl VimWindow {
    pub fn new(
        id: u64,
        grid: u64,
        rect: gdk::Rectangle,
        hldefs: HighlightDefinitions,
        font_description: Rc<RefCell<pango::FontDescription>>,
    ) -> VimWindow {
        VimWindow {
            id,
            grid,
            rect,
            hldefs,
            linespace: 0,
            charwidth: 0.,
            lineheight: 0.,

            visible: true,
            queued_draw: false,

            font_description,

            grids: crate::factory::FactoryMap::new(),
        }
    }

    pub fn set_pos(&mut self, x: u64, y: u64) {
        self.rect.set_x(x as i32);
        self.rect.set_y(y as i32);
    }

    pub fn get(&self, grid: u64) -> Option<&VimGrid> {
        self.grids.get(grid)
    }

    pub fn get_mut(&mut self, grid: u64) -> Option<&mut VimGrid> {
        self.grids.get_mut(grid)
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn show(&mut self) {
        self.visible = true;
    }

    pub fn clear(&mut self) {
        self.grids
            .iter_mut()
            .for_each(|(_, grid)| grid.textbuf().borrow().clear());
    }

    pub fn queue_draw(&mut self) {
        self.queued_draw = true;
    }

    pub fn remove(&mut self, grid: u64) {
        self.grids.remove(grid);
    }

    pub fn add(&mut self, grid: u64, width: u64, height: u64, hldefs: HighlightDefinitions) {
        // let view = VimGridView::new(grid, width, height);
        let rect = gdk::Rectangle::new(0, 0, width as _, height as _);
        log::error!("creating grid {} {}x{}", grid, width, height);
        // FIXME: convert width to cols, height to rows.
        let textbuf = TextBuf::new(height as _, width as _);
        let vimgrid = VimGrid {
            win: self.id,
            grid,
            rect,
            hldefs,
            textbuf,
            visible: true,
            linespace: 0,
            charwidth: 0.,
            lineheight: 0.,
            font_description: self.font_description.clone(),
        };
        self.grids.insert(grid, vimgrid);
    }

    pub fn resize(&mut self, width: u64, height: u64) {
        self.rect.set_width(width as _);
        self.rect.set_height(height as _);
    }

    /// (width, height)
    fn size_required(&self, cols: u64, rows: u64) -> (u64, u64) {
        (
            (cols as f64 * self.charwidth) as u64,
            (rows as f64 * self.lineheight) as u64,
        )
    }
}
