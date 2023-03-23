&gtk::DrawingArea {
                        set_widget_name: "cursor",
                        set_visible: true,
                        set_hexpand: true,
                        set_vexpand: true,
                        set_can_focus: false,
                        set_sensitive: false,
                        set_focus_on_click: false,
                        set_css_classes: &["blink"],
                        set_draw_func[hldefs = model.hldefs.clone(),
                                      cursor = model.cursor.clone(),
                                      metrics = model.metrics.clone(),
                                      pctx = model.pctx.clone()] => move |da, cr, _, _| {
                            da.remove_css_class("blink");
                            da.set_opacity(1.);
                            let cursor = cursor.borrow();
                            let blinkon = cursor.blinkon().filter(|blinkon| *blinkon > 0);
                            let blinkoff = cursor.blinkoff().filter(|blinkoff| *blinkoff > 0);
                            let blinkwait = cursor.blinkwait().filter(|blinkwait| *blinkwait > 0);
                            if let (Some(blinkon), Some(blinkoff), Some(blinkwait)) = (blinkon, blinkoff, blinkwait) {
                                let css = format!(".blink {{
  animation-name: blinking;
  animation-delay: {}ms;
  animation-duration: {}ms;
  animation-iteration-count: infinite;
  animation-timing-function: steps(2, start);
}}

@keyframes blinking {{
  {}% {{ opacity: 0; }}
}}
",
                                    blinkwait,
                                    blinkon + blinkoff,
                                    blinkon * 100 / (blinkon + blinkoff)
                                );
                                let context = da.style_context();
                                let provider = gtk::CssProvider::new();
                                provider.load_from_data(css.as_bytes());
                                // FIXME: add once.
                                context.add_provider(&provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
                                debug!("css {} {}: \n{}", blinkon, blinkoff, &css);
                                da.add_css_class("blink");
                            }
                            let hldefs = hldefs.read();
                            let default_colors = hldefs.defaults().unwrap();
                            let bg = cursor.background(default_colors);
                            let fg = cursor.foreground(default_colors);
                            let cell = cursor.cell();
                            let metrics = metrics.get();
                            let (x, y, width, height)  = cursor.rectangle(metrics.width(), metrics.height());
                            debug!("drawing cursor at {}x{}.", x, y);
                            match cursor.shape {
                                CursorShape::Block => {
                                    use pango::AttrType;
                                    let attrs = pango::AttrList::new();
                                    cell.attrs.iter().filter_map(|attr| {
                                        match attr.type_() {
                                            AttrType::Family | AttrType::Style | AttrType::Weight | AttrType::Variant | AttrType::Underline | AttrType::Strikethrough | AttrType::Overline => {
                                                let mut attr = attr.clone();
                                                attr.set_start_index(0);
                                                attr.set_end_index(pango::ATTR_INDEX_TO_TEXT_END);
                                                Some(attr)
                                            },
                                            _ => None
                                        }
                                    }).for_each(|attr| attrs.insert(attr));
                                    debug!("cursor cell '{}' wide {}", cell.text, cursor.width);
                                    let itemized = &pango::itemize(&pctx, &cell.text, 0, cell.text.len() as _, &attrs, None)[0];
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
                                        debug!("cursor glyph width {}", width);
                                    }
                                    // 试试汉字
                                    cr.save().unwrap();
                                    cr.rectangle(x, y, width as f64, metrics.height());
                                    cr.set_source_rgba(bg.red() as f64, bg.green() as f64, bg.blue() as f64, bg.alpha() as f64);
                                    cr.fill().unwrap();
                                    cr.restore().unwrap();
                                    cr.set_source_rgba(fg.red() as f64, fg.green() as f64, fg.blue() as f64, fg.alpha() as f64);
                                    cr.move_to(x + geometry.width() as f64 / 2., y + metrics.ascent());
                                    pangocairo::show_glyph_string(cr, &itemized.analysis().font(), &mut glyph_string);
                                }
                                _ => {
                                    debug!("drawing cursor with {}x{}", width, height);
                                    cr.set_source_rgba(bg.red() as f64, bg.green() as f64, bg.blue() as f64, bg.alpha() as f64);
                                    cr.rectangle(x, y, width, height);
                                    cr.fill().unwrap();
                                }
                            }
                        }
                    }
