/*
pub struct KeyEvent {
    keyval: gdk::keys::Key,
    state: gdk::ModifierType,
}

impl From<(gdk::keys::Key, gdk::ModifierType)> for KeyEvent {
    fn from((keyval, state): (gdk::keys::Key, gdk::ModifierType)) -> Self {
        Self { keyval, state }
    }
}*/

pub trait ToInput {
    fn to_input(&self) -> Option<String>;
}

fn map_keyname(keyname: &str) -> Option<&str> {
        // Originally sourced from python-gui.
        match keyname {
            "asciicircum" => Some("^"), // fix #137
            "slash" => Some("/"),
            "backslash" => Some("\\"),
            "dead_circumflex" => Some("^"),
            "at" => Some("@"),
            "numbersign" => Some("#"),
            "colon" => Some(":"),
            "dollar" => Some("$"),
            "percent" => Some("%"),
            "ampersand" => Some("&"),
            "asterisk" => Some("*"),
            "parenleft" => Some("("),
            "parenright" => Some(")"),
            "underscore" => Some("_"),
            "plus" => Some("+"),
            "minus" => Some("-"),
            "bracketleft" => Some("["),
            "bracketright" => Some("]"),
            "braceleft" => Some("{"),
            "braceright" => Some("}"),
            "dead_diaeresis" => Some("\""),
            "dead_acute" => Some("\'"),
            "less" => Some("<"),
            "greater" => Some(">"),
            "comma" => Some(","),
            "period" => Some("."),
            "BackSpace" => Some("BS"),
            "Insert" => Some("Insert"),
            "Return" => Some("CR"),
            "Escape" => Some("Esc"),
            "Delete" => Some("Del"),
            "Page_Up" => Some("PageUp"),
            "Page_Down" => Some("PageDown"),
            "Enter" => Some("CR"),
            "ISO_Left_Tab" => Some("Tab"),
            "Tab" => Some("Tab"),
            "Up" => Some("Up"),
            "Down" => Some("Down"),
            "Left" => Some("Left"),
            "Right" => Some("Right"),
            "Home" => Some("Home"),
            "End" => Some("End"),
            "F1" => Some("F1"),
            "F2" => Some("F2"),
            "F3" => Some("F3"),
            "F4" => Some("F4"),
            "F5" => Some("F5"),
            "F6" => Some("F6"),
            "F7" => Some("F7"),
            "F8" => Some("F8"),
            "F9" => Some("F9"),
            "F10" => Some("F10"),
            "F11" => Some("F11"),
            "F12" => Some("F12"),
            _ => None,
        }
}

impl ToInput for (&gdk::Key, &gdk::ModifierType) {
    fn to_input(&self) -> Option<String> {
        let mut input = String::with_capacity(8);

        let keyname = self.0.name()?;

        let modifiers = self.1;
        if modifiers.contains(gdk::ModifierType::SHIFT_MASK) {
            input.push_str("S-");
        }
        if modifiers.contains(gdk::ModifierType::CONTROL_MASK) {
            input.push_str("C-");
        }
        if modifiers.contains(gdk::ModifierType::ALT_MASK) {
            input.push_str("A-");
        }
        if modifiers.contains(gdk::ModifierType::SUPER_MASK) {
            input.push_str("M-");
        }

        if keyname.chars().count() > 1 {
            input.push_str(map_keyname(&keyname)?);
        } else {
            input.push(self.0.to_unicode()?);
        }

        Some(format!("<{}>", input))
    }
}
