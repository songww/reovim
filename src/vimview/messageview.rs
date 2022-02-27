use std::{cell::Cell, rc::Rc, sync::RwLock};

use glib::subclass::prelude::*;
use gtk::prelude::*;
use once_cell::sync::OnceCell;
use relm4::{
    factory::{Factory, FactoryPrototype, FactoryVec, FactoryView},
    WidgetPlus,
};

use crate::{
    app::{AppMessage, AppModel},
    bridge::{MessageKind, RedrawEvent, StyledContent},
    metrics::Metrics,
};

use super::{HighlightDefinitions, VimGridView};

mod imp {
    use std::{cell::Cell, rc::Rc, slice::SliceIndex, sync::RwLock};

    use glib::{ffi::g_unichar_iswide, translate::from_glib};
    use gtk::{gdk::prelude::*, prelude::*, subclass::prelude::*};
    use once_cell::sync::OnceCell;

    use crate::{
        bridge::{GridLineCell, MessageKind, StyledContent},
        metrics::Metrics,
        vimview::{HighlightDefinitions, TextBuf, VimGridView},
    };

    // #[derive(Derivative)]
    #[derive(Debug)]
    pub struct VimMessageView {
        kind: Cell<MessageKind>,
        // styled_content: StyledContent,
        view: VimGridView,
        // hldefs: OnceCell<Rc<RwLock<HighlightDefinitions>>>,
        metrics: OnceCell<Rc<Cell<crate::metrics::Metrics>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for VimMessageView {
        const NAME: &'static str = "VimMessageView";
        type ParentType = gtk::Frame;
        type Type = super::VimMessageView;

        fn new() -> Self {
            let view = VimGridView::new(u64::MAX, 1, 1);
            Self {
                view,
                kind: Cell::new(MessageKind::Unknown),
                metrics: OnceCell::new(),
            }
        }
    }

    // Trait shared by all GObjects
    impl ObjectImpl for VimMessageView {
        fn constructed(&self, obj: &Self::Type) {
            obj.set_child(Some(&self.view));
            self.parent_constructed(obj);
        }

        fn properties() -> &'static [glib::ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecUInt64::new(
                        "id",
                        "grid-id",
                        "id",
                        1,
                        u64::MAX,
                        1,
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
                ]
            });

            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            _obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                //"id" => {
                //    self.id.replace(value.get::<u64>().unwrap());
                //}
                //"width" => {
                //    self.width.replace(value.get::<u64>().unwrap());
                //    self.textbuf()
                //        .resize(self.height.get() as _, self.width.get() as _);
                //}
                //"height" => {
                //    self.height.replace(value.get::<u64>().unwrap());
                //    self.textbuf()
                //        .resize(self.height.get() as _, self.width.get() as _);
                //}
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                //"id" => self.id.get().to_value(),
                //"width" => self.width.get().to_value(),
                //"height" => self.height.get().to_value(),
                _ => unimplemented!(),
            }
        }
    }

    // Trait shared by all widgets
    impl WidgetImpl for VimMessageView {
        fn snapshot(&self, widget: &Self::Type, snapshot: &gtk::Snapshot) {
            widget.snapshot_child(&self.view, snapshot);
            self.parent_snapshot(widget, snapshot);
        }

        fn measure(
            &self,
            _widget: &Self::Type,
            orientation: gtk::Orientation,
            for_size: i32,
        ) -> (i32, i32, i32, i32) {
            self.view.measure(orientation, for_size)
        }
    }

    impl FrameImpl for VimMessageView {
        //
    }

    impl VimMessageView {
        pub fn set_styled_context(&self, styled_content: StyledContent) {
            let (mut max_cols, mut cols, mut rows) = (1, 1, 0);
            let mut lines: Vec<Vec<GridLineCell>> = Vec::new();
            lines.push(Vec::new());
            for (style, text) in styled_content.iter() {
                for (no, line) in text.lines().enumerate() {
                    if no > 0 {
                        max_cols = max_cols.max(cols);
                        lines.push(Vec::with_capacity(max_cols));
                        rows += 1;
                        cols = 0;
                    }
                    for c in line.chars() {
                        let double_width: bool = unsafe { from_glib(g_unichar_iswide(c as u32)) };
                        lines[rows].push(GridLineCell {
                            text: String::from(c),
                            hldef: Some(*style),
                            repeat: None,
                            double_width,
                        });
                        cols += 1;
                        if double_width {
                            lines[rows].push(GridLineCell {
                                text: String::from(""),
                                hldef: Some(*style),
                                repeat: None,
                                double_width: false,
                            });
                            cols += 1;
                        }
                    }
                }
            }
            cols = max_cols.max(cols);
            rows = rows + 1;
            let textbuf = self.view.textbuf();
            textbuf.resize(rows, cols);
            for (no, cells) in lines.iter_mut().enumerate() {
                if cells.len() < cols {
                    cells.resize(
                        cols,
                        GridLineCell {
                            text: String::from(" "),
                            hldef: None,
                            repeat: None,
                            double_width: false,
                        },
                    );
                }
                textbuf.set_cells(no, 0, &cells);
            }
        }
        pub fn set_pango_context(&self, pctx: Rc<pango::Context>) {
            self.view.textbuf().set_pango_context(pctx);
        }
        pub fn set_hldefs(&self, hldefs: Rc<RwLock<HighlightDefinitions>>) {
            self.view.set_hldefs(hldefs)
        }
        pub fn set_metrics(&self, metrics: Rc<Cell<Metrics>>) {
            self.view.set_metrics(metrics.clone());
            self.metrics.set(metrics).unwrap();
        }
        pub fn set_kind(&self, kind: MessageKind) {
            self.kind.set(kind);
        }
    }
}

glib::wrapper! {
    pub struct VimMessageView(ObjectSubclass<imp::VimMessageView>)
        @extends gtk::Widget, gtk::Frame,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl VimMessageView {
    pub fn new(
        kind: MessageKind,
        styled_content: StyledContent,
        hldefs: Rc<RwLock<HighlightDefinitions>>,
        metrics: Rc<Cell<Metrics>>,
        pctx: Rc<pango::Context>,
    ) -> VimMessageView {
        let this: VimMessageView =
            glib::Object::new(&[]).expect("Failed to create `VimMessageView`.");
        let name = format!("vim-message-{}", kind);
        this.set_widget_name(&name);
        this.set_css_classes(&["vim-message", &name]);
        let imp = this.imp();
        imp.set_kind(kind);
        imp.set_hldefs(hldefs);
        imp.set_metrics(metrics);
        imp.set_pango_context(pctx);
        imp.set_styled_context(styled_content);
        this.set_halign(gtk::Align::End);
        this.set_valign(gtk::Align::Start);
        this.set_overflow(gtk::Overflow::Visible);
        this
    }
    fn imp(&self) -> &imp::VimMessageView {
        imp::VimMessageView::from_instance(self)
    }
}

pub struct VimMessage {
    kind: MessageKind,
    styled_content: StyledContent,
    hldefs: Rc<RwLock<HighlightDefinitions>>,
    metrics: Rc<Cell<Metrics>>,
    pctx: Rc<pango::Context>,
}

impl VimMessage {
    pub fn new(
        kind: MessageKind,
        styled_content: StyledContent,
        hldefs: Rc<RwLock<HighlightDefinitions>>,
        metrics: Rc<Cell<Metrics>>,
        pctx: Rc<pango::Context>,
    ) -> VimMessage {
        VimMessage {
            kind,
            styled_content,
            hldefs,
            metrics,
            pctx,
        }
    }

    pub fn kind(&self) -> MessageKind {
        self.kind
    }
}

#[derive(Debug)]
pub struct MessageViewWidgets {
    view: VimMessageView,
}

impl FactoryPrototype for VimMessage {
    type Factory = FactoryVec<Self>;
    type Widgets = MessageViewWidgets;
    type Root = VimMessageView;
    type View = gtk::Box;
    type Msg = AppMessage;
    fn init_view(
        &self,
        _key: &<Self::Factory as Factory<Self, Self::View>>::Key,
        _sender: relm4::Sender<AppMessage>,
    ) -> Self::Widgets {
        let guard = self.hldefs.read().unwrap();
        let colors = guard.defaults().unwrap();
        let metrics = self.metrics.get();
        let view = VimMessageView::new(
            self.kind,
            self.styled_content.clone(),
            self.hldefs.clone(),
            self.metrics.clone(),
            self.pctx.clone(),
        );
        view.set_margin_top(metrics.height() as _);
        view.set_margin_end(metrics.width() as _);
        let fg = colors.foreground.unwrap();
        if matches!(self.kind, MessageKind::Echo) {
        } else {
            //
        }
        let style = format!(
            "border: 1px solid {}; padding: {}px {}px; background: {};",
            fg.to_str(),
            metrics.height() / 2.,
            metrics.width(),
            colors.background.unwrap().to_str()
        );
        log::info!("inline css for message: {}", &style);
        view.inline_css(style.as_bytes());
        MessageViewWidgets { view }
    }

    fn position(&self, _: &usize) {}
    fn view(&self, _: &usize, widgets: &Self::Widgets) {
        // let guard = self.hldefs.read().unwrap();
        // let colors = guard.defaults().unwrap();
        // widgets.view.inline_css(
        //     format!("border 1px solid {}", colors.foreground.unwrap().to_str()).as_bytes(),
        // );
        widgets.view.show();
    }
    fn root_widget(widgets: &Self::Widgets) -> &Self::Root {
        &widgets.view
    }
}
