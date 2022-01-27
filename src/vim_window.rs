use std::cell::Cell;
use std::rc::Rc;
use std::sync::RwLock;

use gtk::prelude::*;
use relm4::factory::positions::FixedPosition;
use relm4::*;

use crate::app;
use crate::bridge::UiCommand;
use crate::cloned;
use crate::color::{Color, ColorExt};

mod widget {

    use glib::Object;

    glib::wrapper! {
        pub struct VimWindow(ObjectSubclass<imp::VimWindow>)
            @extends gtk::Widget,
            @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
    }

    impl VimWindow {
        pub fn new() -> VimWindow {
            Object::new(&[]).expect("Failed to create `VimWindow`.")
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

        // pub fn set_pos(&mut self, x: u64, y: u64) {
        //     self.rect.set_x(x as i32);
        //     self.rect.set_y(y as i32);
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

        pub fn clear(&self) {
            //
        }

        pub fn flush(self) {
            // self.should_flush = true;
        }
    }
    mod imp {
        use gtk::prelude::WidgetExt;
        use gtk::subclass::prelude::*;

        // Object holding the state
        #[derive(Debug, Default)]
        pub struct VimWindow {
            grid: u64,
            window: u64,
            // doc: Option<super::super::Document>,
        }

        // The central trait for subclassing a GObject
        #[glib::object_subclass]
        impl ObjectSubclass for VimWindow {
            const NAME: &'static str = "VimWindow";
            type Type = super::VimWindow;
            type ParentType = gtk::Widget;
        }

        // Trait shared by all GObjects
        impl ObjectImpl for VimWindow {
            // fn constructed(&self, obj: &Self::Type) {
            //     self.parent_constructed(obj);
            //     obj.set_label(&self.number.get().to_string());
            // }
        }

        // Trait shared by all widgets
        impl WidgetImpl for VimWindow {
            fn snapshot(&self, widget: &Self::Type, snapshot: &gtk::Snapshot) {
                // let rect = graphene::Rect::new(0., 0., 100., 100.);
                // let cr = snapshot.append_cairo(&rect);
                // let pctx = widget.pango_context();
                // let layout = self.doc.as_ref().unwrap().layout(pctx);
                // pangocairo::show_layout(&cr, &layout);
                self.parent_snapshot(widget, snapshot);
            }
        }

        // impl WidgetImplExt for VimWindow {}
    }
}

// #[derive(Clone)]
// struct Document {
//     rows: usize,
//     columns: usize,
//     hldefs: HighlightDefinitions,
//     cells: Box<[Box<[crate::bridge::GridLineCell]>]>,
// }
//
// impl Default for Document {
//     fn default() -> Self {
//         Self::new(1, 1, HighlightDefinitions::new())
//     }
// }
//
// impl std::fmt::Debug for Document {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         f.debug_struct("Document")
//             .field("rows", &self.rows)
//             .field("columns", &self.columns)
//             .finish()
//     }
// }
//
// impl Document {
//     fn new(rows: usize, columns: usize, hldefs: HighlightDefinitions) -> Self {
//         Self {
//             rows,
//             columns,
//             hldefs,
//             cells: Self::empty_cells(rows, columns),
//         }
//     }
//
//     fn empty_cells(rows: usize, columns: usize) -> Box<[Box<[crate::bridge::GridLineCell]>]> {
//         let column = vec![
//             crate::bridge::GridLineCell {
//                 text: " ".to_string(),
//                 highlight_id: None,
//                 repeat: None,
//                 double_width: false
//             };
//             columns
//         ]
//         .into_boxed_slice();
//         vec![column; rows].into_boxed_slice()
//     }
//
//     fn set_line(&mut self, row: usize, column: usize, cells: Vec<crate::bridge::GridLineCell>) {
//         let mut cells = cells;
//         let line = &mut self.cells[row];
//         println!(
//             "default length {}, from column {}, with {} cells",
//             self.columns,
//             column,
//             cells.len()
//         );
//         let column_to = column + cells.len();
//         line[column..column_to].clone_from_slice(&cells);
//     }
//
//     fn clear(&mut self) {
//         self.cells = Self::empty_cells(self.rows, self.columns);
//     }
//
//     fn layout(&self, pctx: pango::Context) -> pango::Layout {
//         let layout = pango::Layout::new(&pctx);
//         layout
//     }
// }

type HighlightDefinitions = Rc<RwLock<crate::vimview::HighlightDefinitions>>;

#[derive(Clone)]
pub struct VimWindow {
    id: u64,
    grid: u64,
    rect: gdk::Rectangle,
    hldefs: HighlightDefinitions,
    repaint: bool,
    should_flush: bool,
    linespace: Option<u64>,
    // doc: Document,
    win: widget::VimWindow,
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
    pub fn new(grid: u64, rect: gdk::Rectangle, hldefs: HighlightDefinitions) -> VimWindow {
        Self {
            id: 0,
            grid,
            rect,
            hldefs: Rc::clone(&hldefs),
            repaint: true,
            linespace: None,
            should_flush: true,
            win: widget::VimWindow::new(),
            // doc: Document::new(rect.height() as _, rect.width() as _, Rc::clone(&hldefs)),
        }
    }

    pub fn set_font_description(&self, desc: &pango::FontDescription) {
        self.win.pango_context().set_font_description(desc)
    }

    pub fn clear(&mut self) {
        //     self.doc.clear();
    }

    pub fn hide(&self) {
        self.win.hide();
    }

    pub fn set_line(&mut self, row: u64, column: u64, cells: Vec<crate::bridge::GridLineCell>) {
        //     self.doc.set_line(row as _, column as _, cells)
    }

    pub fn should_flush(&mut self) {
        self.should_flush = true;
    }

    pub fn queue_draw(&self) {
        self.win.queue_draw();
    }

    pub fn resize(&mut self, width: i32, height: i32) {
        self.rect.set_width(width);
        self.rect.set_height(height);

        // self.doc.rows = height as _;
        // self.doc.columns = width as _;
        // self.doc.clear();
    }
}

#[derive(Debug)]
pub struct VimWindowWidgets {
    root: widget::VimWindow,
}

impl factory::FactoryPrototype for VimWindow {
    type Factory = crate::factory::FactoryMap<Self>;
    type Widgets = VimWindowWidgets;
    type Root = widget::VimWindow;
    type View = gtk::Fixed;
    type Msg = app::AppMessage;

    fn init_view(&self, grid_id: &u64, sender: Sender<app::AppMessage>) -> VimWindowWidgets {
        let w = widget::VimWindow::new();
        // let grid_id = self.grid;
        let on_click = gtk::GestureClick::new();
        on_click.set_name("vim-window-onclick-listener");
        on_click.connect_pressed(cloned!(sender, grid_id => move |c, n_press, x, y| {
            log::debug!("{:?} pressed {} times at {}x{}", c.name(), n_press, x, y);
            sender.send(
                UiCommand::MouseButton {
                    action: "press".to_string(),
                    grid_id,
                    position: (x as u32, y as u32)
                }.into()
            ).expect("Failed to send mouse press event");
        }));
        on_click.connect_released(cloned!(sender, grid_id => move |c, n_press, x, y| {
            log::debug!("{:?} released {} times at {}x{}", c.name(), n_press, x, y);
            sender.send(
                UiCommand::MouseButton {
                    action: "release".to_string(),
                    grid_id,
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

    fn root_widget(widgets: &VimWindowWidgets) -> &widget::VimWindow {
        &widgets.root
    }
}
