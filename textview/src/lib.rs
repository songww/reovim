#![feature(hash_set_entry)]
pub mod attr;
pub mod builtin_font;
pub mod color;
pub mod consts;
pub mod drawing;
pub mod font_metrics;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
