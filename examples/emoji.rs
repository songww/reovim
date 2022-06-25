// use unic_emoji_char as unic;
// use unicode_normalization::{is_nfc, is_nfd, is_nfkc, is_nfkd, UnicodeNormalization};
// use unicode_segmentation::UnicodeSegmentation;
// use xi_unicode::EmojiExt;
//
// fn main() {
//     println!("a is zwj {}", 'a'.is_zwj());
//     println!("a is emoji {}", 'a'.is_emoji());
//     println!("a is emoji modifier {}", 'a'.is_emoji_modifier());
//     println!("a is emoji modifier base {}", 'a'.is_emoji_modifier_base());
//     println!("a is tag spec char {}", 'a'.is_tag_spec_char());
//     println!("a is emoji cancel char {}", 'a'.is_emoji_cancel_tag());
//     println!(
//         "a is emoji combining enclosing keycap {}",
//         'a'.is_emoji_combining_enclosing_keycap()
//     );
//     println!(
//         "a is regional indicator symbol {}",
//         'a'.is_regional_indicator_symbol()
//     );
//     for c in "▶\u{fe0e}".chars() {
//         println!("{} is zwj {}", c, c.is_zwj());
//         println!("{} is emoji {}", c, c.is_emoji());
//         println!("{} is emoji modifier {}", c, c.is_emoji_modifier());
//         println!(
//             "{} is emoji modifier base {}",
//             c,
//             c.is_emoji_modifier_base()
//         );
//         println!("{} is tag spec char {}", c, c.is_tag_spec_char());
//         println!("{} is emoji cancel char {}", c, c.is_emoji_cancel_tag());
//         println!(
//             "{} is emoji combining enclosing keycap {}",
//             c,
//             'a'.is_emoji_combining_enclosing_keycap()
//         );
//         println!(
//             "{} is regional indicator symbol {}",
//             c,
//             c.is_regional_indicator_symbol()
//         );
//     }
//
//     println!("----------------------");
//     println!("a is nfc {}", is_nfc("a"));
//     println!("a is nfkc {}", is_nfkc("a"));
//     println!("a is nfd {}", is_nfd("a"));
//     println!("a is nfkd {}", is_nfkd("a"));
//     println!("'▶\u{fe0e}' is nfc {}", is_nfc("▶\u{fe0e}"));
//     println!("'▶\u{fe0e}' is nfkc {}", is_nfkc("▶\u{fe0e}"));
//     println!("'▶\u{fe0e}' is nfd {}", is_nfd("▶\u{fe0e}"));
//     println!("'▶\u{fe0e}' is nfkd {}", is_nfkd("▶\u{fe0e}"));
//
//     println!("----------------------");
//     println!(
//         "contains {}",
//         emoji::lookup_by_glyph::contains_glyph(&"▶\u{fe0e}".nfc().collect::<String>())
//     );
//     // println!(
//     //     "contains {:?}",
//     //     emoji::lookup_by_glyph::lookup(&"▶\u{fe0e}".nfc().collect::<String>()).unwrap()
//     // );
//     println!("{}", &"▶\u{fe0e}".nfc().collect::<String>());
//     println!(
//         "emojis get {:?}",
//         emojis::get(&"▶\u{fe0e}".nfc().collect::<String>())
//     );
//     println!("emojis get {:?}", emojis::get("▶\u{fe0e}"));
//     for c in "a▶\u{fe0e}".chars() {
//         println!("unic is emoji {:?}", unic::is_emoji(c));
//         println!("unic is emoji component {:?}", unic::is_emoji_component(c));
//         println!("unic is emoji modifier {:?}", unic::is_emoji_modifier(c));
//         println!(
//             "unic is emoji modifier base {:?}",
//             unic::is_emoji_modifier_base(c)
//         );
//         println!(
//             "unic is emoji presentation {:?}",
//             unic::is_emoji_presentation(c)
//         );
//     }
//
//     for graphemes in "a▶\u{fe0e}".graphemes(true) {
//         println!("graphemes is {:?}", graphemes);
//     }
//
//     println!("Find Emoji {:?}", emojito::find_emoji("▶\u{fe0e}"));
//     println!(
//         "Emoji {}",
//         regex::Regex::new(r"^\p{Emoji}+$")
//             .unwrap()
//             .is_match("▶\u{fe0e}")
//     );
//     // Emoji
//     println!(
//         "Emoji_Presentation {}",
//         regex::Regex::new(r"^\p{Emoji_Presentation}$")
//             .unwrap()
//             .is_match("a▶\u{fe0e}")
//     );
//     println!(
//         "Emoji_Modifier {}",
//         regex::Regex::new(r"\p{Emoji_Modifier}")
//             .unwrap()
//             .is_match("a▶\u{fe0e}")
//     );
//     println!(
//         "Emoji_Modifier_Base {}",
//         regex::Regex::new(r"\p{Emoji_Modifier_Base}")
//             .unwrap()
//             .is_match("a▶\u{fe0e}")
//     );
//     println!(
//         "Emoji_Component {}",
//         regex::Regex::new(r"\p{Emoji_Component}")
//             .unwrap()
//             .is_match("a▶\u{fe0e}")
//     );
//
//     use unicode_script::UnicodeScript;
//     println!("'中' script: {}", '中'.script());
//     println!("'a' script: {}", 'a'.script());
//     println!("'▶' script: {}", '▶'.script());
//     println!("'\u{fe0e}' script: {}", '\u{fe0e}'.script());
// }

fn main() {}
