use std::{rc::Rc, sync::RwLock};

use adw::prelude::*;
use relm4::{
    factory::{FactoryComponent, FactoryVecDeque},
    prelude::*,
};
use tracing::{error, info};

use crate::{
    bridge::{MessageKind, StyledContent},
    vimview::{self, HighlightDefinitions},
};

#[derive(Debug)]
pub enum VimNotificationMessage {
    Show(MessageKind, StyledContent, bool),
    Mode(StyledContent),
    Ruler(StyledContent),
    Histories(Vec<(MessageKind, StyledContent)>),
    Clear,
    SetPosition(f64),
}

pub struct VimMessageWidgets {
    view: gtk::Frame,
}

#[derive(Debug)]
pub struct VimMessage {
    pub kind: MessageKind,
    pub content: Vec<(u64, String)>,
    pub hldefs: Rc<RwLock<vimview::HighlightDefinitions>>,
}

impl FactoryComponent for VimMessage {
    type Widgets = VimMessageWidgets;
    type Root = gtk::Box;
    type Input = ();
    type Output = ();
    type ParentWidget = relm4::gtk::Box;
    type ParentInput = VimNotificationMessage;
    type CommandOutput = ();
    type Init = (
        MessageKind,
        Vec<(u64, String)>,
        Rc<RwLock<vimview::HighlightDefinitions>>,
    );

    fn init_widgets(
        &mut self,
        _index: &DynamicIndex,
        root: &Self::Root,
        _returned_widget: &<Self::ParentWidget as relm4::factory::FactoryView>::ReturnedWidget,
        _sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let child = gtk::TextView::builder()
            .name("vim-message-text")
            .overflow(gtk::Overflow::Hidden)
            .cursor_visible(false)
            .editable(false)
            .build();
        let view = gtk::Frame::builder()
            .name("vim-message-frame")
            .focus_on_click(false)
            .focusable(false)
            .child(&child)
            .build();

        root.append(&view);

        VimMessageWidgets { view }
    }

    fn output_to_parent_input(_output: Self::Output) -> Option<Self::ParentInput> {
        None
    }

    fn init_model(
        (kind, content, hldefs): Self::Init,
        _index: &DynamicIndex,
        _sender: FactorySender<Self>,
    ) -> Self {
        VimMessage {
            kind,
            content,
            hldefs,
        }
    }

    fn init_root(&self) -> Self::Root {
        relm4::view! {
            root = gtk::Box {
                set_widget_name: "vim-messages",
                set_spacing: 10,
                set_orientation: gtk::Orientation::Vertical,
            }
        }

        root
    }

    // fn update(&mut self, message: Self::Input, sender: FactorySender<Self>) {
    fn update(&mut self, _message: Self::Input, _sender: FactorySender<Self>) {
        match self.kind {
            MessageKind::Echo => {
                let _message = String::new();
                //
                /*
                for (idx, text) in self.content.iter() {
                    let guard = self.hldefs.read();
                    let style = guard.get(*idx).unwrap();
                    let default_colors = guard.defaults().unwrap();
                    message.len() as u32;
                    let tag = gtk::TextTag::new(Some(&idx.to_string()));
                    if style.bold {
                        tag.set_weight(600);
                    }
                    if style.italic {
                        tag.set_style(pango::Style::Italic);
                    }
                    if style.underline {
                        tag.set_underline(pango::Underline::Single);
                        tag.set_underline_rgba(Some(&style.special(&default_colors)));
                    }
                    if style.undercurl {
                        tag.set_underline(pango::Underline::Error);
                        tag.set_underline_rgba(Some(&style.special(&default_colors)));
                    }
                    if style.strikethrough {
                        tag.set_strikethrough(true);
                        tag.set_strikethrough_rgba(Some(&style.special(&default_colors)))
                    }
                    tag.set_background_full_height(true);
                    tag.set_background_rgba(
                        style
                            .background()
                            .or_else(|| default_colors.background)
                            .as_ref(),
                    );
                    tag.set_foreground_rgba(Some(&style.foreground(default_colors)));
                    //
                    message.push_str(&text);
                }
                */
            }
            MessageKind::Error => {
                //
            }
            _ => {
                unimplemented!()
            }
        }
    }

    fn update_view(&self, _widgets: &mut Self::Widgets, _sender: FactorySender<Self>) {
        //
    }
}

#[derive(Debug)]
pub struct VimNotification {
    visible: bool,
    hldefs: Rc<RwLock<HighlightDefinitions>>,
    messages: FactoryVecDeque<VimMessage>,
}

#[relm4::component(pub)]
impl Component for VimNotification {
    type Init = Rc<RwLock<HighlightDefinitions>>;
    type Input = VimNotificationMessage;
    type Output = ();
    type CommandOutput = ();
    view! {
        #[root]
        view = gtk::Box {
            #[local_ref]
            messagebox -> gtk::Box {
                #[watch]
                set_visible: model.visible,
                set_widget_name: "vim-messages",
                set_spacing: 10,
                set_orientation: gtk::Orientation::Vertical,
            }
        }
    }

    fn init(
        hldefs: Rc<RwLock<HighlightDefinitions>>,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let messages = FactoryVecDeque::new(gtk::Box::default(), sender.input_sender());
        let model = VimNotification {
            hldefs,
            visible: false,
            messages,
        };

        let messagebox = model.messages.widget();

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>, _root: &Self::Root) {
        let mut guard = self.messages.guard();
        match message {
            VimNotificationMessage::Clear => {
                info!("Messages cleared.");
                guard.clear();
                self.visible = false;
            }
            VimNotificationMessage::Show(kind, content, replace_last) => {
                self.visible = true;
                if replace_last && !guard.is_empty() {
                    guard.pop_back();
                }
                guard.push_back((kind, content, self.hldefs.clone()));
            }
            VimNotificationMessage::Ruler(ruler) => {
                error!("Ruler not supported yet {:?}.", ruler);
            }
            VimNotificationMessage::Histories(entries) => {
                error!("History not supported yet {:?}", entries);
            }
            VimNotificationMessage::Mode(mode) => {
                info!("Current mode: {:?}", mode);
            }
            VimNotificationMessage::SetPosition(pos) => {
                unimplemented!("where to show {:?}", pos);
            }
        }
    }
}
