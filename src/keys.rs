use gtk::gdk;

pub trait ToInput {
    fn to_input(&self) -> Option<String>;
}

fn map_keyname(keyname: String) -> Option<&'static str> {
    // Originally sourced from python-gui.
    match keyname.as_ref() {
        "asciicircum" => "^".into(), // fix #137
        "slash" => "/".into(),
        "backslash" => "\\".into(),
        "dead_circumflex" => "^".into(),
        "at" => "@".into(),
        "numbersign" => "#".into(),
        "colon" => ":".into(),
        "dollar" => "$".into(),
        "percent" => "%".into(),
        "ampersand" => "&".into(),
        "asterisk" => "*".into(),
        "parenleft" => "(".into(),
        "parenright" => ")".into(),
        "underscore" => "_".into(),
        "plus" => "+".into(),
        "minus" => "-".into(),
        "bracketleft" => "[".into(),
        "bracketright" => "]".into(),
        "braceleft" => "{".into(),
        "braceright" => "}".into(),
        "dead_diaeresis" => "\"".into(),
        "dead_acute" => "\'".into(),
        "less" => "<".into(),
        "greater" => ">".into(),
        "comma" => ",".into(),
        "period" => ".".into(),
        "BackSpace" => "BS".into(),
        "Insert" => "Insert".into(),
        "Return" => "CR".into(),
        "Escape" => "Esc".into(),
        "Delete" => "Del".into(),
        "Page_Up" => "PageUp".into(),
        "Page_Down" => "PageDown".into(),
        "Enter" => "CR".into(),
        "ISO_Left_Tab" => "Tab".into(),
        "Tab" => "Tab".into(),
        "Up" => "Up".into(),
        "Down" => "Down".into(),
        "Left" => "Left".into(),
        "Right" => "Right".into(),
        "Home" => "Home".into(),
        "End" => "End".into(),
        "F1" => "F1".into(),
        "F2" => "F2".into(),
        "F3" => "F3".into(),
        "F4" => "F4".into(),
        "F5" => "F5".into(),
        "F6" => "F6".into(),
        "F7" => "F7".into(),
        "F8" => "F8".into(),
        "F9" => "F9".into(),
        "F10" => "F10".into(),
        "F11" => "F11".into(),
        "F12" => "F12".into(),
        _ => None,
    }
}

impl ToInput for gdk::ModifierType {
    fn to_input(&self) -> Option<String> {
        let mut input = String::with_capacity(8);

        if self.contains(gdk::ModifierType::SHIFT_MASK) {
            input.push_str("S-");
        }
        if self.contains(gdk::ModifierType::CONTROL_MASK) {
            input.push_str("C-");
        }
        if self.contains(gdk::ModifierType::ALT_MASK) {
            input.push_str("A-");
        }
        if self.contains(gdk::ModifierType::SUPER_MASK) {
            input.push_str("M-");
        }

        Some(input.to_string())
    }
}

impl ToInput for (gdk::Key, gdk::ModifierType) {
    fn to_input(&self) -> Option<String> {
        let modkey = self.1.to_input()?;
        let keyname = self.0.name().unwrap();

        if keyname.chars().count() > 1 {
            format!("<{}{}>", modkey, map_keyname(keyname.to_string())?).into()
        } else {
            let k = self.0.to_unicode().unwrap();
            if !self.1.is_empty() {
                format!("<{}{}>", modkey, k).into()
            } else {
                k.to_string().into()
            }
        }
    }
}
