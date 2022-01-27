use std::cell::Cell;
use std::rc::Rc;
use std::sync::RwLock;

use gdk::Rectangle;
use glib::subclass::prelude::*;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use relm4::factory::positions::FixedPosition;
use relm4::*;

use crate::app;
use crate::bridge::UiCommand;
use crate::cloned;
use crate::color::{Color, ColorExt};

type HighlightDefinitions = Rc<RwLock<crate::vimview::HighlightDefinitions>>;

mod imp {
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;

    // Object holding the state
    #[derive(Debug)]
    pub struct VimWindowView {
        grid: u64,
        grids: Vec<u64>,
        window: u64,
        rect: gdk::Rectangle,
        // doc: Option<super::super::Document>,
    }

    impl Default for VimWindowView {
        fn default() -> Self {
            VimWindowView {
                rect: gdk::Rectangle::new(0, 0, 0, 0),
                ..Default::default()
            }
        }
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for VimWindowView {
        const NAME: &'static str = "VimWindow";
        type Type = super::VimWindowView;
        type ParentType = gtk::Widget;
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
                widget.pango_context().font_description()
            );
            self.parent_snapshot(widget, snapshot);
        }
    }

    impl VimWindowView {
        pub(super) fn set_font_description(&self, desc: &pango::FontDescription) {
            // self.pango_context().set_font_description(desc);
        }
    }
}

glib::wrapper! {
    pub struct VimWindowView(ObjectSubclass<imp::VimWindowView>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl VimWindowView {
    pub fn new(window: u64, grid: u64, rect: Rectangle) -> VimWindowView {
        glib::Object::new(&[("window", &window), ("grid", &grid), ("rect", &rect)])
            .expect("Failed to create `VimWindowView`.")
    }

    // pub fn render(&self, hldefs: &crate::app::HighlightDefinitions) {
    //     let cr = self.dh.borrow_mut().get_context().unwrap();
    //     cr.save().unwrap();
    //     cr.rectangle(
    //         self.rect.x() as f64,
    //         self.rect.y() as f64,
    //         self.rect.width() as f64,
    //         self.rect.height() as f64,
    //     );
    //     if let Some(defaults) = hldefs.defaults() {
    //         if let Some(background) = defaults.background {
    //             cr.set_source_rgba(
    //                 background.red() as _,
    //                 background.green() as _,
    //                 background.blue() as _,
    //                 background.alpha() as _,
    //             );
    //             cr.paint().unwrap();
    //     cr.restore().unwrap();
    //             println!("default background {}", background.to_hex());
    //         }
    //     }
    //     println!("rendering {}x{}", self.rect.width(), self.rect.height());
    //     cr.fill().unwrap();
    //     cr.restore().unwrap();
    //     cr.save().unwrap();
    //     cr.restore().unwrap();
    // }

    // pub fn resize(&mut self, width: u64, height: u64) {
    // log::info!(
    //     "resize request: {}x{} -> {}x{}",
    //     self.rect.width(),
    //     self.rect.height(),
    //     width,
    //     height
    // );
    //     self.rect.set_width(width as i32);
    //     self.rect.set_height(height as i32);
    // }

    // pub fn set_fonts(&self, fonts: &pango::FontDescription) {
    // self.da.pango_context().set_font_description(fonts);
    // }

    // pub fn set_linespace(&mut self, linespace: u64) {
    //     self.linespace.replace(linespace);
    // }

    // pub fn grid(&self) -> u64 {
    //     self.grid
    // }

    // pub fn linespace(&self) -> Option<u64> {
    //     self.linespace
    // }

    // pub fn rect(&self) -> gdk::Rectangle {
    //     self.rect
    // }

    pub fn flush(self) {
        // self.should_flush = true;
    }
    // pub fn new(grid: u64, rect: gdk::Rectangle, hldefs: HighlightDefinitions) -> VimWindow {
    //     Self {
    //         id: 0,
    //         grid,
    //         rect,
    //         hldefs: Rc::clone(&hldefs),
    //         repaint: true,
    //         linespace: None,
    //         should_flush: true,
    //         win: widget::VimWindow::new(),
    //         // doc: Document::new(rect.height() as _, rect.width() as _, Rc::clone(&hldefs)),
    //     }
    // }
    fn imp(&self) -> &imp::VimWindowView {
        imp::VimWindowView::from_instance(self)
    }

    pub fn set_font_description(&self, desc: &pango::FontDescription) {
        self.imp().set_font_description(desc)
    }

    pub fn clear(&mut self) {
        //     self.doc.clear();
    }

    pub fn set_line(&mut self, row: u64, column: u64, cells: Vec<crate::bridge::GridLineCell>) {
        //     self.doc.set_line(row as _, column as _, cells)
    }

    pub fn should_flush(&mut self) {
        // self.should_flush = true;
    }

    pub fn resize(&mut self, width: i32, height: i32) {
        // self.rect.set_width(width);
        // self.rect.set_height(height);

        // self.doc.rows = height as _;
        // self.doc.columns = width as _;
        // self.doc.clear();
    }
}

#[derive(Debug)]
pub struct VimWindowWidgets {
    root: VimWindowView,
}

impl factory::FactoryPrototype for VimWindow {
    type Factory = crate::factory::FactoryMap<Self>;
    type Widgets = VimWindowWidgets;
    type Root = VimWindowView;
    type View = gtk::Fixed;
    type Msg = app::AppMessage;

    fn init_view(&self, grid: &u64, sender: Sender<app::AppMessage>) -> VimWindowWidgets {
        let w = VimWindowView::new(self.id, *grid, self.rect);
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
        w.add_controller(&on_click);

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
        w.add_controller(&on_scroll);

        VimWindowWidgets { root: w }
    }

    fn position(&self, _grid: &u64) -> FixedPosition {
        FixedPosition {
            x: self.rect.x() as f64,
            y: self.rect.y() as f64,
        }
    }

    fn view(&self, index: &u64, widgets: &VimWindowWidgets) {
        log::info!("vim window update {} {:?}", index, self.rect);
        let win = &widgets.root;
        // win.render(&self.hldefs);
    }

    fn root_widget(widgets: &VimWindowWidgets) -> &VimWindowView {
        &widgets.root
    }
}

#[derive(Clone)]
pub struct VimWindow {
    id: u64,
    grid: u64,
    rect: gdk::Rectangle,
    hldefs: HighlightDefinitions,
    // repaint: bool,
    // should_flush: bool,
    // linespace: Option<u64>,
    // view: VimWindowView,
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
    ) -> VimWindow {
        VimWindow {
            id,
            grid,
            rect,
            hldefs,
            // view: Cell::new(None),
        }
    }

    pub fn set_pos(&mut self, x: u64, y: u64) {
        self.rect.set_x(x as i32);
        self.rect.set_y(y as i32);
    }
}
