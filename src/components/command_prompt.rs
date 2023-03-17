use std::{cell::Cell, collections::LinkedList, rc::Rc, sync::RwLock};

use gtk::prelude::*;
use once_cell::sync::OnceCell;
use relm4::{
    prelude::*,
};
use tracing::{debug, info};

use crate::{
    bridge::{StyledContent},
    vimview::{HighlightDefinitions},
};

pub static PROMPT_BROKER: relm4::MessageBroker<CommandPromptMessage> = relm4::MessageBroker::new();

#[derive(Debug)]
struct CommandPrompt {
    level: u64,
    changed: Cell<bool>,
    name: String,
    text: String,
    position: u64,
    attrs: pango::AttrList,
    widget: OnceCell<gtk::Popover>,
}

impl CommandPrompt {
    fn new(level: u64, name: &str) -> CommandPrompt {
        CommandPrompt {
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
pub enum CommandPromptMessage {
    Show(StyledContent, u64, String, String, u64, u64),
    Hide,
    BlockHide,
}

#[derive(debug::Debug)]
pub struct VimCommandPrompt {
    hldefs: Rc<RwLock<HighlightDefinitions>>,
    prompts: LinkedList<CommandPrompt>,
    #[debug(skip)]
    removed: Cell<Option<Vec<gtk::Popover>>>,
}

//impl Model for VimCmdPrompts {
//    type Msg = VimCommandPromptEvent;
//    type Widgets = VimCmdPromptWidgets;
//    type Components = ();
//}

#[relm4::component(pub)]
impl Component for VimCommandPrompt {
    type CommandOutput = ();
    type Input = CommandPromptMessage;
    type Output = ();
    type Init = Rc<RwLock<HighlightDefinitions>>;
    fn init(
        hldefs: Rc<RwLock<HighlightDefinitions>>,
        root: &Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = VimCommandPrompt {
            hldefs,
            removed: Cell::new(None),
            prompts: LinkedList::new(),
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(
        &mut self,
        event: CommandPromptMessage,
        _sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        const U16MAX: f32 = u16::MAX as f32;
        match event {
            CommandPromptMessage::BlockHide => {
                todo!()
            }
            CommandPromptMessage::Hide => {
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
            CommandPromptMessage::Show(styled_content, position, start, prompt, indent, level) => {
                let indent = indent as usize;
                info!(
                    "cmd event level={} indent={} position={} start={} prompt={} {:?}",
                    level, indent, position, start, prompt, styled_content
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
                    self.prompts.push_back(CommandPrompt::new(level, name));
                    prompt_opt = self.prompts.back_mut();
                } else if let Some(after) = after {
                    let mut right = self.prompts.split_off(after);
                    self.prompts.push_back(CommandPrompt::new(level, name));
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

    view! {
        view = gtk::Fixed {
            set_visible: false,
            inline_css: "border: 0 solid #e5e7eb;",
        }
    }

    fn pre_view() {
        if let Some(removed) = model.removed.take() {
            for popover in removed.into_iter() {
                view.remove(&popover);
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
                popover.set_parent(view);
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
                        popover.set_parent(view);
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
                .insert_before(view, iter.peek().and_then(|p| p.widget.get()));
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
