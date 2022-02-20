use std::{rc::Rc, sync::RwLock};

use gtk::prelude::*;
use relm4::{
    factory::{FactoryPrototype, FactoryVec},
    ComponentUpdate, Model, Sender, WidgetPlus, Widgets,
};

use crate::{
    app::{AppMessage, AppModel},
    bridge::{MessageKind, StyledContent},
    vimview::{self, HighlightDefinitions, VimGridView},
};

#[derive(Debug)]
pub enum VimNotifactionEvent {
    Show(MessageKind, StyledContent, bool),
    Mode(StyledContent),
    Ruler(StyledContent),
    Histories(Vec<(MessageKind, StyledContent)>),
    Clear,
    SetPosition(f64),
}

// #[derive(Debug)]
pub struct VimNotifactions {
    visible: bool,
    hldefs: Rc<RwLock<HighlightDefinitions>>,
    messages: FactoryVec<VimMessage>,
}

#[derive(Debug)]
pub struct VimMessage {
    pub kind: MessageKind,
    pub content: Vec<(u64, String)>,
    pub hldefs: Rc<RwLock<vimview::HighlightDefinitions>>,
}

impl FactoryPrototype for VimMessage {
    type Factory = FactoryVec<Self>;
    type Widgets = VimNotifactionWidgets;
    type Root = gtk::Frame;
    type View = gtk::Box;
    type Msg = VimNotifactionEvent;

    fn init_view(
        &self,
        _key: &<Self::Factory as relm4::factory::Factory<Self, Self::View>>::Key,
        _sender: Sender<Self::Msg>,
    ) -> Self::Widgets {
        let view = gtk::Frame::new(None);
        let child = gtk::TextView::new();
        child.set_editable(false);
        child.set_cursor_visible(false);
        child.set_widget_name("vim-message-text");
        child.set_overflow(gtk::Overflow::Hidden);
        view.set_child(Some(&child));
        view.set_focusable(false);
        view.set_focus_on_click(false);
        view.set_widget_name("vim-message-frame");
        VimNotifactionWidgets { view }
    }

    fn view(
        &self,
        _: &<Self::Factory as relm4::factory::Factory<Self, Self::View>>::Key,
        widgets: &Self::Widgets,
    ) {
        match self.kind {
            MessageKind::Echo => {
                let mut message = String::new();
                //
                /*
                for (idx, text) in self.content.iter() {
                    let guard = self.hldefs.read().unwrap();
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

    fn position(
        &self,
        _: &<Self::Factory as relm4::factory::Factory<Self, Self::View>>::Key,
    ) -> <Self::View as relm4::factory::FactoryView<Self::Root>>::Position {
    }

    fn root_widget(widgets: &Self::Widgets) -> &Self::Root {
        &widgets.view
    }
}

impl Model for VimNotifactions {
    type Msg = VimNotifactionEvent;
    type Widgets = VimNotifactionsWidgets;
    type Components = ();
}

impl ComponentUpdate<AppModel> for VimNotifactions {
    fn init_model(parent_model: &AppModel) -> Self {
        VimNotifactions {
            visible: false,
            hldefs: parent_model.hldefs.clone(),
            messages: FactoryVec::new(),
        }
    }

    fn update(
        &mut self,
        event: VimNotifactionEvent,
        _components: &(),
        _sender: Sender<VimNotifactionEvent>,
        _parent_sender: Sender<AppMessage>,
    ) {
        match event {
            VimNotifactionEvent::Clear => {
                log::info!("Messages cleared.");
                self.messages.clear();
                self.visible = false;
            }
            VimNotifactionEvent::Show(kind, content, replace_last) => {
                self.visible = true;
                if replace_last && !self.messages.is_empty() {
                    self.messages.pop();
                }
                self.messages.push(VimMessage {
                    kind,
                    content,
                    hldefs: self.hldefs.clone(),
                })
            }
            VimNotifactionEvent::Ruler(ruler) => {
                log::error!("Ruler not supported yet {:?}.", ruler);
            }
            VimNotifactionEvent::Histories(entries) => {
                log::error!("History not supported yet {:?}", entries);
            }
            VimNotifactionEvent::Mode(mode) => {
                log::info!("Current mode: {:?}", mode);
            }
            VimNotifactionEvent::SetPosition(pos) => {
                unimplemented!("where to show {:?}", pos);
            }
        }
    }
}

#[derive(Debug)]
pub struct VimNotifactionWidgets {
    view: gtk::Frame,
}

#[relm_macros::widget(pub)]
impl Widgets<VimNotifactions, AppModel> for VimNotifactionsWidgets {
    view! {
        view = gtk::Box {
            set_visible: watch!(model.visible),
            set_widget_name: "vim-messages",
            set_spacing: 10,
            factory!(model.messages),
        }
        // gtk::TextView {
        //     set_visible: watch!(model.visible),
        //     set_cursor_visible: false,
        // }
    }
}

#[derive(Debug)]
pub enum VimCmdEvent {
    Showing(Vec<(u64, String)>),
}
#[derive(Debug)]
pub struct VimCmdPrompt {
    visible: bool,
    hldefs: Rc<RwLock<HighlightDefinitions>>,
}

impl Model for VimCmdPrompt {
    type Msg = VimCmdEvent;
    type Widgets = VimCmdPromptWidgets;
    type Components = ();
}

impl ComponentUpdate<AppModel> for VimCmdPrompt {
    fn init_model(parent_model: &AppModel) -> Self {
        VimCmdPrompt {
            visible: false,
            hldefs: parent_model.hldefs.clone(),
        }
    }

    fn update(
        &mut self,
        event: VimCmdEvent,
        _components: &(),
        _sender: Sender<VimCmdEvent>,
        _parent_sender: Sender<AppMessage>,
    ) {
        match event {
            VimCmdEvent::Showing(_) => {
                self.visible = true;
            }
        }
    }
}

#[relm_macros::widget(pub)]
impl Widgets<VimCmdPrompt, AppModel> for VimCmdPromptWidgets {
    view! {
        view = VimGridView {
            set_visible: watch!(model.visible),
            inline_css: b"boder: 0 solid #e5e7eb",
        }
    }
}
