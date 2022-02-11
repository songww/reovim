use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::RwLock;

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
    use std::cell::Cell;

    use gtk::prelude::*;
    use gtk::subclass::prelude::*;

    // Object holding the state
    #[derive(Debug)]
    pub struct VimWindowView {
        win: Cell<u64>,
        width: Cell<u64>,
        height: Cell<u64>,
        default_grid: Cell<u64>,
    }

    impl Default for VimWindowView {
        fn default() -> Self {
            VimWindowView {
                win: 0.into(),
                width: 0.into(),
                height: 0.into(),
                default_grid: 0.into(),
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
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
        }

        fn properties() -> &'static [glib::ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecUInt64::new(
                        "win",
                        "window-id",
                        "win",
                        0,
                        u64::MAX,
                        0,
                        glib::ParamFlags::READWRITE,
                    ),
                    glib::ParamSpecUInt64::new(
                        "width",
                        "cols",
                        "width",
                        0,
                        u64::MAX,
                        0,
                        glib::ParamFlags::READWRITE,
                    ),
                    glib::ParamSpecUInt64::new(
                        "height",
                        "rows",
                        "height",
                        0,
                        u64::MAX,
                        0,
                        glib::ParamFlags::READWRITE,
                    ),
                    glib::ParamSpecUInt64::new(
                        "default-grid",
                        "default-grid-id",
                        "default-grid",
                        0,
                        u64::MAX,
                        0,
                        glib::ParamFlags::READWRITE,
                    ),
                ]
            });

            PROPERTIES.as_ref()
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "win" => self.win.get().to_value(),
                "width" => self.width.get().to_value(),
                "height" => self.height.get().to_value(),
                "default-grid" => self.default_grid.get().to_value(),
                _ => unimplemented!(),
            }
        }

        fn set_property(
            &self,
            _obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "win" => {
                    self.win.replace(value.get().unwrap());
                }
                "default-grid" => {
                    self.default_grid.replace(value.get::<u64>().unwrap());
                }
                "width" => {
                    self.width.replace(value.get::<u64>().unwrap());
                }
                "height" => {
                    self.height.replace(value.get::<u64>().unwrap());
                }
                _ => unimplemented!(),
            }
        }
    }

    // Trait shared by all widgets
    impl WidgetImpl for VimWindowView {
        fn snapshot(&self, widget: &Self::Type, snapshot: &gtk::Snapshot) {
            self.parent_snapshot(widget, snapshot);
        }

        fn measure(
            &self,
            widget: &Self::Type,
            orientation: gtk::Orientation,
            for_size: i32,
        ) -> (i32, i32, i32, i32) {
            let size_required = self.size_required(widget);
            log::error!(
                "measuring VimWindowView orientation {} for_size {} size_required {:?}",
                orientation,
                for_size,
                size_required
            );
            match orientation {
                gtk::Orientation::Vertical => (size_required.1, size_required.1, -1, -1),
                gtk::Orientation::Horizontal => (size_required.0, size_required.0, -1, -1),
                _ => self.parent_measure(widget, orientation, for_size),
            }
        }
    }

    impl FixedImpl for VimWindowView {}

    impl VimWindowView {
        fn size_required(&self, widget: &<Self as ObjectSubclass>::Type) -> (i32, i32) {
            let font_desc = widget.pango_context().font_description().unwrap();
            log::info!("font desc {}", font_desc.to_str());
            let metrics = widget.pango_context().metrics(None, None).unwrap();
            let metrics_ = widget
                .pango_context()
                .metrics(Some(&font_desc), None)
                .unwrap();
            assert_eq!(metrics_.height(), metrics.height());
            assert_eq!(
                metrics_.approximate_digit_width(),
                metrics.approximate_digit_width()
            );
            let lineheight = metrics.height() as f64 / pango::SCALE as f64;
            let charwidth = metrics.approximate_digit_width() as f64 / pango::SCALE as f64;
            (
                (self.width.get() as f64 * charwidth) as i32,
                (self.height.get() as f64 * lineheight) as i32,
            )
        }
    }
}

glib::wrapper! {
    pub struct VimWindowView(ObjectSubclass<imp::VimWindowView>)
        @extends gtk::Widget, gtk::Fixed,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl VimWindowView {
    pub fn new(window: u64, grid: u64, width: u64, height: u64) -> VimWindowView {
        // glib::Object::new(&[("window", &window), ("grid", &grid), ("rect", &rect)])
        glib::Object::new(&[
            ("win", &window),
            ("default-grid", &grid),
            ("width", &width),
            ("height", &height),
        ])
        .expect("Failed to create `VimWindowView`.")
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

// #[derive(Debug)]
pub struct VimGrid {
    win: u64,
    grid: u64,
    move_to: Cell<Option<FixedPosition>>,
    pos: FixedPosition,
    width: u64,
    height: u64,
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
            view = VimGridView::new(*grid, self.width, self.height) {
                set_widget_name: &format!("vim-grid-{}-{}", self.win, grid),
                set_hldefs: self.hldefs.clone(),
                set_textbuf:self.textbuf.as_ref().clone(),

                set_visible: self.visible,

                set_overflow: gtk::Overflow::Hidden,

                inline_css: b"border: 1px solid green;",
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

    fn position(&self, grid: &u64) -> FixedPosition {
        log::debug!("requesting grid position {}", grid);
        FixedPosition {
            x: self.pos.x,
            y: self.pos.y,
        }
    }

    fn view(&self, index: &u64, widgets: &VimGridWidgets) {
        log::debug!(
            "vim grid update {} pos {:?} size {}x{}",
            index,
            self.pos,
            self.width,
            self.height
        );
        let grid = &widgets.view;

        grid.set_visible(self.visible);
        grid.set_property("width", &self.width);
        grid.set_property("height", &self.height);

        if let Some(pos) = self.move_to.take() {
            gtk::prelude::FixedExt::move_(
                &grid.parent().unwrap().downcast::<gtk::Fixed>().unwrap(),
                grid,
                pos.x,
                pos.y,
            );
        }
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
        self.visible = true;
    }

    pub fn resize(&mut self, width: u64, height: u64) {
        self.width = width;
        self.height = height;
        self.textbuf.borrow().resize(height as _, width as _);
    }

    pub fn set_pos(&mut self, x: f64, y: f64) {
        self.pos = FixedPosition { x, y };
        self.move_to.replace(FixedPosition { x, y }.into());
    }

    /*
    /// (width, height)
    fn size_required(&self, cols: u64, rows: u64) -> (u64, u64) {
        (
            (cols as f64 * self.charwidth) as u64,
            (rows as f64 * self.lineheight) as u64,
        )
    }
    */
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
            view = VimWindowView::new(self.id, *grid, self.width as _, self.height as _) {
                set_visible: self.visible,
                set_widget_name: &format!("vim-window-{}-{}", self.id, grid),

                set_overflow: gtk::Overflow::Hidden,

                inline_css: b"border: 1px solid blue;",
            }
        }
        relm4::factory::Factory::generate(&self.grids, &view, sender.clone());

        self.sender.set(sender.clone()).ok();

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
        log::debug!("requesting window position {}", _grid);
        FixedPosition {
            x: self.pos.x,
            y: self.pos.y,
        }
    }

    fn view(&self, index: &u64, widgets: &VimWindowWidgets) {
        // log::warn!("vim window update {} {:?}", index, self.rect);
        let view = &widgets.view;

        view.set_visible(self.visible);
        view.set_property("width", self.width);
        view.set_property("height", self.height);

        if let Some(pos) = self.move_to.take() {
            gtk::prelude::FixedExt::move_(
                &view.parent().unwrap().downcast::<gtk::Fixed>().unwrap(),
                view,
                pos.x,
                pos.y,
            );
        }

        relm4::factory::Factory::generate(&self.grids, &view, self.sender.get().unwrap().clone());

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
    pub grid: u64,
    width: u64,
    height: u64,
    move_to: Cell<Option<FixedPosition>>,
    pos: FixedPosition,
    hldefs: HighlightDefinitions,
    font_description: Rc<RefCell<pango::FontDescription>>,

    visible: bool,
    queued_draw: bool,

    sender: OnceCell<Sender<app::AppMessage>>,

    grids: crate::factory::FactoryMap<VimGrid>,
}

impl std::fmt::Debug for VimWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VimWindow")
            .field("id", &self.id)
            .field("grid", &self.grid)
            .field("size", &(self.width, self.width))
            .field("position", &self.pos)
            .finish()
    }
}

impl VimWindow {
    pub fn new(
        id: u64,
        grid: u64,
        pos: FixedPosition,
        size: (u64, u64),
        hldefs: HighlightDefinitions,
        font_description: Rc<RefCell<pango::FontDescription>>,
    ) -> VimWindow {
        VimWindow {
            id,
            grid,
            hldefs,
            move_to: None.into(),
            pos,
            width: size.0,
            height: size.1,

            visible: true,
            queued_draw: false,

            font_description,

            sender: OnceCell::new(),

            grids: crate::factory::FactoryMap::new(),
        }
    }

    pub fn grids(&self) -> &crate::factory::FactoryMap<VimGrid> {
        &self.grids
    }

    pub fn set_pos(&mut self, x: f64, y: f64) {
        self.pos = FixedPosition { x, y };
        self.move_to.replace(FixedPosition { x, y }.into());
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

    pub fn remove(&mut self, grid: u64) -> Option<VimGrid> {
        self.grids.remove(grid)
    }

    pub fn add(&mut self, grid: u64, width: u64, height: u64, hldefs: HighlightDefinitions) {
        // let view = VimGridView::new(grid, width, height);
        log::info!("creating grid {} {}x{}", grid, width, height);
        let textbuf = TextBuf::new(height as _, width as _);
        let vimgrid = VimGrid {
            win: self.id,
            grid,
            pos: FixedPosition { x: 0., y: 0. },
            width,
            height,
            move_to: None.into(),
            hldefs,
            textbuf,
            visible: true,
            font_description: self.font_description.clone(),
        };
        self.grids.insert(grid, vimgrid);
        log::error!(
            "Add grid {} to {}, girds {:?}",
            grid,
            self.id,
            self.grids.iter().map(|(k, _)| *k).collect::<Vec<_>>()
        );
    }

    pub fn resize(&mut self, width: u64, height: u64) {
        self.width = width;
        self.height = height;
    }
}
