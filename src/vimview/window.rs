use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::RwLock;

use glib::subclass::prelude::*;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use relm4::factory::positions::FixedPosition;
use relm4::*;

use crate::app;
use crate::bridge::UiCommand;
use crate::cloned;

use super::grid::VimGridView;
use super::TextBuf;

type HighlightDefinitions = Rc<RwLock<crate::vimview::HighlightDefinitions>>;

/*
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
*/

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
    width: usize,
    height: usize,
    hldefs: HighlightDefinitions,
    metrics: Rc<Cell<app::FontMetrics>>,
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

                set_overflow: gtk::Overflow::Hidden,

                set_font_description: &self.font_description.borrow(),

                set_css_classes: &[&format!("vim-view-grid-{}", self.grid)],

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

        VimGridWidgets { view }
    }

    fn position(&self, _: &u64) -> FixedPosition {
        log::debug!("requesting grid position {}", self.grid);
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

        view.queue_allocate();
        view.queue_resize();
        // grid.queue_draw();

        log::info!(
            "font-description {}",
            self.font_description.borrow().to_str()
        );
    }

    fn root_widget(widgets: &VimGridWidgets) -> &VimGridView {
        &widgets.view
    }
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

/*
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

                set_font_description: &self.font_description.borrow(),

                // inline_css: b"border: 1px solid @borders;",
            }
        }
        relm4::factory::Factory::generate(&self.grids, &view, sender.clone());

        self.sender.set(sender.clone()).ok();

        // let grid_id = self.grid;

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
        view.set_font_description(&self.font_description.borrow());

        if let Some(pos) = self.move_to.take() {
            gtk::prelude::FixedExt::move_(
                &view.parent().unwrap().downcast::<gtk::Fixed>().unwrap(),
                view,
                pos.x,
                pos.y,
            );
        }

        relm4::factory::Factory::generate(&self.grids, &view, self.sender.get().unwrap().clone());

        // view.queue_draw();
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

    /*
    pub fn clear(&mut self) {
        self.grids
            .iter_mut()
            .for_each(|(_, grid)| grid.textbuf().borrow().clear());
    }
    */

    pub fn queue_draw(&mut self) {
        self.queued_draw = true;
    }

    pub fn remove(&mut self, grid: u64) -> Option<VimGrid> {
        self.grids.remove(grid)
    }

    pub fn add(&mut self, grid: u64, width: u64, height: u64) {
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
            hldefs: self.hldefs.clone(),
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
*/
