use glib::subclass::types::FromObject;
use gtk::prelude::FixedExt;
use gtk::traits::WidgetExt;

mod imp {
    use gtk::subclass::prelude::*;

    #[derive(Default)]
    pub struct Board;

    #[gtk::glib::object_subclass]
    impl ObjectSubclass for Board {
        const NAME: &'static str = "Board";
        type Type = super::Board;
        type ParentType = gtk::Fixed;
    }

    // Trait shared by all GObjects
    impl ObjectImpl for Board {}

    // Trait shared by all widgets
    impl WidgetImpl for Board {}

    // Trait shared by all buttons
    impl FixedImpl for Board {}
}

glib::wrapper! {
    pub struct Board(ObjectSubclass<imp::Board>)
        @extends gtk::Fixed, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Board {
    pub fn new() -> Board {
        glib::Object::builder().build()
    }

    fn imp(&self) -> &imp::Board {
        imp::Board::from_object(self)
    }
}

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}

impl relm4::RelmSetChildExt for Board {
    fn container_set_child(&self, widget: Option<&impl AsRef<gtk::Widget>>) {
        widget.map(|widget| self.put(widget.as_ref(), 0., 0.));
    }

    fn container_get_child(&self) -> Option<gtk::Widget> {
        self.first_child()
    }
}

impl relm4::ContainerChild for Board {
    type Child = gtk::Widget;
}

impl relm4::factory::FactoryView for Board {
    type Children = gtk::Widget;
    type ReturnedWidget = gtk::Widget;

    type Position = BoardPosition;

    fn factory_remove(&self, widget: &Self::ReturnedWidget) {
        self.remove(widget);
    }

    fn factory_append(
        &self,
        widget: impl AsRef<Self::Children>,
        position: &Self::Position,
    ) -> Self::ReturnedWidget {
        let widget = widget.as_ref();
        self.put(widget, position.x, position.y);
        widget.clone()
    }

    fn factory_prepend(
        &self,
        widget: impl AsRef<Self::Children>,
        position: &Self::Position,
    ) -> Self::ReturnedWidget {
        let widget = widget.as_ref();
        self.put(widget, position.x, position.y);
        widget.clone()
    }

    fn factory_move_after(&self, widget: &Self::ReturnedWidget, other: &Self::ReturnedWidget) {
        //
    }

    fn factory_move_start(&self, widget: &Self::ReturnedWidget) {
        //
    }

    fn factory_insert_after(
        &self,
        widget: impl AsRef<Self::Children>,
        position: &Self::Position,
        other: &Self::ReturnedWidget,
    ) -> Self::ReturnedWidget {
        let widget = widget.as_ref();
        self.put(widget, position.x, position.y);
        widget.clone()
    }

    fn factory_update_position(&self, widget: &Self::ReturnedWidget, position: &Self::Position) {
        self.move_(widget, position.x, position.y)
    }

    fn returned_widget_to_child(root_child: &Self::ReturnedWidget) -> Self::Children {
        root_child.clone()
    }
}

// impl Position<()> for Board {
//     fn position(&self, index: usize) -> () {}
// }

#[derive(Copy, Clone, Debug)]
pub struct BoardPosition {
    pub x: f64,
    pub y: f64,
    // id: u64,
}
