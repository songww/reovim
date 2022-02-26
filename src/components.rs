use std::{cell::Cell, collections::LinkedList, rc::Rc, sync::RwLock};

use gtk::prelude::*;
use once_cell::sync::OnceCell;
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
    }
}

struct VimCommandPrompt {
    level: u64,
    changed: Cell<bool>,
    name: String,
    text: String,
    position: u64,
    attrs: pango::AttrList,
    widget: OnceCell<gtk::Popover>,
}

impl VimCommandPrompt {
    fn new(level: u64, name: &str) -> VimCommandPrompt {
        VimCommandPrompt {
            level,
            changed: true.into(),
            name: name.to_string(),
            position: 0,
            text: String::new(),
            attrs: pango::AttrList::new(),
            widget: OnceCell::new(),
        }
    }
}

#[derive(Debug)]
pub enum VimCmdEvent {
    Show(StyledContent, u64, String, String, u64, u64),
    Hide,
    BlockHide,
}

#[derive(Derivative)]
pub struct VimCmdPrompts {
    hldefs: Rc<RwLock<HighlightDefinitions>>,
    prompts: LinkedList<VimCommandPrompt>,
    #[derivative(Debug = "ignore")]
    removed: Cell<Option<Vec<gtk::Popover>>>,
}

impl Model for VimCmdPrompts {
    type Msg = VimCmdEvent;
    type Widgets = VimCmdPromptWidgets;
    type Components = ();
}

impl ComponentUpdate<AppModel> for VimCmdPrompts {
    fn init_model(parent_model: &AppModel) -> Self {
        VimCmdPrompts {
            hldefs: parent_model.hldefs.clone(),
            removed: Cell::new(None),
            prompts: LinkedList::new(),
        }
    }

    fn update(
        &mut self,
        event: VimCmdEvent,
        _components: &(),
        _sender: Sender<VimCmdEvent>,
        _parent_sender: Sender<AppMessage>,
    ) {
        const U16MAX: f32 = u16::MAX as f32;
        match event {
            VimCmdEvent::BlockHide => {
                todo!()
            }
            VimCmdEvent::Hide => {
                self.prompts
                    .pop_back()
                    .and_then(|mut top| top.widget.take())
                    .map(|popover| {
                        self.removed
                            .get_mut()
                            .get_or_insert(Vec::new())
                            .push(popover);
                    });
            }
            VimCmdEvent::Show(styled_content, position, start, prompt, indent, level) => {
                let indent = indent as usize;
                log::info!(
                    "cmd event level={} indent={} position={} start={} prompt={} {:?}",
                    level,
                    indent,
                    position,
                    start,
                    prompt,
                    styled_content
                );
                let (name, length) = if !start.is_empty() {
                    ("vim-input-command", start.len())
                } else {
                    ("vim-input-prompt", prompt.len())
                };
                let length = styled_content
                    .iter()
                    .fold(length + indent, |length, (_, text)| length + text.len());
                let mut text = String::with_capacity(length);
                text.push_str(if !start.is_empty() { &start } else { &prompt });
                text.push_str(&" ".repeat(indent));
                let mut prompt_opt = None;
                let mut after = None;
                for (idx, c) in self.prompts.iter_mut().enumerate() {
                    if c.level == level {
                        prompt_opt.replace(c);
                        break;
                    }
                    if c.level > level {
                        after.replace(idx);
                        break;
                    }
                }

                if prompt_opt.is_none() && after.is_none() {
                    self.prompts.push_back(VimCommandPrompt::new(level, name));
                    prompt_opt = self.prompts.back_mut();
                } else if let Some(after) = after {
                    let mut right = self.prompts.split_off(after);
                    self.prompts.push_back(VimCommandPrompt::new(level, name));
                    self.prompts.append(&mut right);
                    prompt_opt = self.prompts.iter_mut().find(|p| p.level == level);
                } else {
                    prompt_opt.as_ref().map(|prompt| {
                        prompt.changed.set(true);
                    });
                }
                let prompt = prompt_opt.unwrap();

                prompt.position = position;

                let hldefs = self.hldefs.read().unwrap();
                let defaults = hldefs.defaults().unwrap();
                let attrs = &prompt.attrs;
                for (hldef, s) in styled_content {
                    let start_index = text.len() as u32;
                    text.push_str(&s);
                    let end_index = text.len() as u32;
                    let style = hldefs.get(hldef).unwrap();

                    if style.italic {
                        let mut attr = pango::AttrInt::new_style(pango::Style::Italic);
                        attr.set_start_index(start_index);
                        attr.set_end_index(end_index);
                        attrs.insert(attr);
                    }
                    if style.bold {
                        let mut attr = pango::AttrInt::new_weight(pango::Weight::Semibold);
                        attr.set_start_index(start_index);
                        attr.set_end_index(end_index);
                        attrs.insert(attr);
                    }
                    if style.strikethrough {
                        let mut attr = pango::AttrInt::new_strikethrough(true);
                        attr.set_start_index(start_index);
                        attr.set_end_index(end_index);
                        attrs.insert(attr);
                    }
                    if style.underline {
                        let mut attr = pango::AttrInt::new_underline(pango::Underline::Single);
                        attr.set_start_index(start_index);
                        attr.set_end_index(end_index);
                        attrs.insert(attr);
                    }
                    if style.undercurl {
                        let mut attr = pango::AttrInt::new_underline(pango::Underline::Error);
                        attr.set_start_index(start_index);
                        attr.set_end_index(end_index);
                        attrs.insert(attr);
                    }
                    let fg = style.foreground(defaults);
                    let mut attr = pango::AttrColor::new_foreground(
                        (fg.red() * U16MAX).round() as u16,
                        (fg.green() * U16MAX).round() as u16,
                        (fg.blue() * U16MAX).round() as u16,
                    );
                    attr.set_start_index(start_index);
                    attr.set_end_index(end_index);
                    attrs.insert(attr);
                    if let Some(bg) = style.background().or(defaults.background) {
                        let mut attr = pango::AttrColor::new_background(
                            (bg.red() * U16MAX).round() as u16,
                            (bg.green() * U16MAX).round() as u16,
                            (bg.blue() * U16MAX).round() as u16,
                        );
                        attr.set_start_index(start_index);
                        attr.set_end_index(end_index);
                        attrs.insert(attr);
                    }
                    let special = style.special(defaults);
                    let mut attr = pango::AttrColor::new_underline_color(
                        (special.red() * U16MAX).round() as u16,
                        (special.green() * U16MAX).round() as u16,
                        (special.blue() * U16MAX).round() as u16,
                    );
                    attr.set_start_index(start_index);
                    attr.set_end_index(end_index);
                    attrs.insert(attr);
                }
                prompt.text = text;
                // label.inline_css(b"border: 0 solid #e5e7eb");
            }
        }
    }
}

#[relm_macros::widget(pub)]
impl Widgets<VimCmdPrompts, AppModel> for VimCmdPromptWidgets {
    view! {
        view = gtk::Fixed {
            set_visible: false,
            inline_css: b"border: 0 solid #e5e7eb;",
        }
    }

    fn pre_view() {
        if let Some(removed) = model.removed.take() {
            for popover in removed.into_iter() {
                self.view.remove(&popover);
            }
        }

        // FIXME: metrics needed.
        // caculate height for per prompt.
        // position of each prompt.
        // ----------------------
        // | level 1            |
        // | |--------------------|
        // --| level 2            |
        //   |                    |
        //   |--------------------|
        //
        if let Some(top) = model.prompts.back() {
            let popover = top.widget.get_or_init(|| {
                gtk::Popover::builder()
                    .autohide(false)
                    .has_arrow(false)
                    .visible(true)
                    // .pointing_to(&gtk::gdk::Rectangle::new(10, 10, 300, 30))
                    .vexpand(false)
                    .hexpand(false)
                    .valign(gtk::Align::Start)
                    .halign(gtk::Align::Center)
                    .position(gtk::PositionType::Bottom)
                    .visible(false)
                    .width_request(600)
                    .height_request(50)
                    .build()
            });
            // ensure root widget has at least one child.
            if popover.parent().is_none() {
                popover.set_parent(&self.view);
            }
            popover.show();
            popover.present();
            // popover.inline_css(b"background: blue;");
        }
        for prompt in model.prompts.iter() {
            if prompt.changed.get() {
                prompt.widget.get_or_init(|| {
                    let popover = gtk::Popover::builder()
                        .autohide(false)
                        .has_arrow(false)
                        .visible(true)
                        .vexpand(false)
                        .hexpand(false)
                        .valign(gtk::Align::Center)
                        .halign(gtk::Align::Start)
                        .position(gtk::PositionType::Bottom)
                        .build();
                    if popover.parent().is_none() {
                        popover.set_parent(&self.view);
                    }
                    popover.show();
                    popover.present();
                    popover
                });
            }
        }
        let mut iter = model.prompts.iter().peekable();
        while let Some(prompt) = iter.next() {
            unsafe { prompt.widget.get_unchecked() }
                .insert_before(&self.view, iter.peek().and_then(|p| p.widget.get()));
            if prompt.changed.replace(false) {
                let popover = unsafe { prompt.widget.get_unchecked() };
                if popover.child().is_none() {
                    popover.set_child(Some(
                        &gtk::Label::builder()
                            .selectable(false)
                            .valign(gtk::Align::Center)
                            .halign(gtk::Align::Start)
                            .visible(true)
                            .hexpand(true)
                            .vexpand(true)
                            .build(),
                    ));
                }
                let child = popover.child().unwrap();
                let label = child.downcast_ref::<gtk::Label>().unwrap();
                label.set_text(&prompt.text);
                label.set_attributes(Some(&prompt.attrs));
            }
        }
    }
}

/*
#[derive(Debug)]
pub enum CursorEvent {
    GoTo,
    Mode,
}

pub struct CursorModel {
    visible: bool,
    hldefs: Rc<RwLock<HighlightDefinitions>>,
}

impl Model for CursorModel {
    type Msg = CursorEvent;
    type Widgets = CursorWidgets;
    type Components = ();
}

impl ComponentUpdate<AppModel> for CursorModel {
    fn init_model(parent_model: &AppModel) -> Self {
        CursorModel {
            visible: false,
            hldefs: parent_model.hldefs.clone(),
        }
    }

    fn update(
        &mut self,
        event: CursorEvent,
        _components: &(),
        _sender: Sender<CursorEvent>,
        _parent_sender: Sender<AppMessage>,
    ) {
        match event {
            CursorEvent::GoTo => {
                self.visible = true;
            }
            CursorEvent::Mode => {}
        }
    }
}

#[relm_macros::widget(pub)]
impl Widgets<CursorModel, AppModel> for CursorWidgets {
    view! {
        da = gtk::DrawingArea {
            set_widget_name: "cursor-drawing-area",
            set_visible: false,
            set_hexpand: false,
            set_vexpand: false,
            set_focus_on_click: false,
            set_draw_func[hldefs = model.hldefs.clone(),
                          cursor = model.cursor.clone(),
                          metrics = model.metrics.clone(),
                          pctx = model.pctx.clone()] => move |_da, cr, _, _| {
                let hldefs = hldefs.read().unwrap();
                let default_colors = hldefs.defaults().unwrap();
                let cursor = cursor.borrow();
                let bg = cursor.background(default_colors);
                let fg = cursor.foreground(default_colors);
                let cell = cursor.cell();
                let metrics = metrics.get();
                let (width, height)  = cursor.size(metrics.width(), metrics.height());
                let (x, y) = cursor.pos();
                log::error!("drawing cursor at {}x{}.", x, y);
                match cursor.shape {
                    CursorShape::Block => {
                        use pango::AttrType;
                        let attrs = pango::AttrList::new();
                        cell.attrs.iter().filter_map(|attr| {
                            match attr.type_() {
                                AttrType::Family | AttrType::Style | AttrType::Weight | AttrType::Variant | AttrType::Underline | AttrType::Strikethrough | AttrType::Overline => {
                                    let mut attr = attr.clone();
                                    attr.set_start_index(0);
                                    attr.set_end_index(0);
                                    Some(attr)
                                }, _ => None
                            }
                        }).for_each(|attr| attrs.insert(attr));
                        let itemized = &pango::itemize(&pctx, &cell.text, 0, -1, &attrs, None)[0];
                        let mut glyph_string = pango::GlyphString::new();
                        pango::shape(&cell.text, itemized.analysis(), &mut glyph_string);
                        let glyphs = glyph_string.glyph_info_mut();
                        assert_eq!(glyphs.len(), 1);
                        let geometry = glyphs[0].geometry_mut();
                        let width = (metrics.width() * cursor.width).ceil() as i32;
                        if geometry.width() > 0 && geometry.width() != width {
                            let x_offset =geometry.x_offset() - (geometry.width() - width) / 2;
                            geometry.set_width(width);
                            geometry.set_x_offset(x_offset);
                        }
                        cr.set_source_rgba(bg.red() as f64, bg.green() as f64, bg.blue() as f64, bg.alpha() as f64);
                        cr.rectangle(x, y, width as f64, metrics.height());
                        cr.fill().unwrap();
                        cr.set_source_rgba(fg.red() as f64, fg.green() as f64, fg.blue() as f64, bg.alpha() as f64);
                        pangocairo::show_glyph_string(cr, &itemized.analysis().font(), &mut glyph_string);
                    }
                    _ => {
                        cr.set_source_rgba(fg.red() as f64, fg.green() as f64, fg.blue() as f64, bg.alpha() as f64);
                        cr.rectangle(x, y, width, height);
                        cr.fill().unwrap();
                    }
                }
            }
        }
    }
}
*/
