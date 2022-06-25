use adw::prelude::*;

use gtk::prelude::WidgetExt;
use relm4::*;

#[derive(Debug)]
enum AppMsg {
    Top,
    Center,
    Bottom,
    Line(usize),
    Inc(usize),
    Dec(usize),
}

struct AppModel {
    nr: usize,
}

impl Model for AppModel {
    type Msg = AppMsg;
    type Widgets = AppWidgets;
    type Components = ();
}

impl AppUpdate for AppModel {
    fn update(&mut self, msg: AppMsg, _components: &(), _sender: Sender<AppMsg>) -> bool {
        println!("AppMessage {:?}", msg);
        match msg {
            AppMsg::Top => {
                self.nr = 0;
            }
            AppMsg::Center => {
                unimplemented!()
            }
            AppMsg::Bottom => {
                unimplemented!()
            }
            AppMsg::Line(nr) => {
                self.nr = nr;
            }
            AppMsg::Inc(inc) => {
                self.nr += inc;
            }
            AppMsg::Dec(dec) => {
                self.nr = self.nr.saturating_sub(dec);
            }
        }
        true
    }
}

#[relm_macros::widget]
impl Widgets<AppModel, ()> for AppWidgets {
    view! {
        main_window = gtk::ApplicationWindow {
            set_default_height: 600,
            set_resizable: true,
            set_titlebar = Some(&adw::HeaderBar) {
                set_title_widget = Some(&adw::WindowTitle) {
                    set_title: "viewport",
                    set_subtitle: "viewport example",
                }
            },
            set_overflow: gtk::Overflow::Hidden,
            set_child: toolbox = Some(&gtk::Box) {
                set_orientation: gtk::Orientation::Vertical,
                set_overflow: gtk::Overflow::Hidden,
                append: actions = &gtk::ActionBar {
                    pack_start: totop = &adw::ButtonContent {
                        set_icon_name: "go-top-symbolic",
                        set_label: "top",
                    },
                    pack_start: tocenter = &adw::ButtonContent {
                        set_icon_name: "object-flip-vertical-symbolic",
                        set_label: "center",
                    },
                    pack_start: tobottom = &adw::ButtonContent {
                        set_icon_name: "go-bottom-symbolic",
                        set_label: "bottom",
                    },
                    pack_end: spin = &gtk::SpinButton {
                        set_range: args!(0., 700.),
                        set_numeric: true,
                        set_increments: args!(1., 10.),
                        connect_value_changed: glib::clone!(@strong sender => move |this| {
                            sender.send(AppMsg::Line(this.value() as usize)).unwrap();
                        })
                    }
                },

                append: clamp = &adw::Clamp {
                    set_overflow: gtk::Overflow::Hidden,
                    set_orientation: gtk::Orientation::Vertical,
                    // set_vadjustment: Some(&vadjustment),
                    // set_max_content_height: 400,
                    // set_min_content_height: 400,
                    set_maximum_size: 200,
                    set_tightening_threshold: 100,
                    // set_minimum_height: 400,
                    set_child: viewport = Some(&gtk::Viewport) {
                        set_overflow: gtk::Overflow::Hidden,

                        set_child: labels = Some(&gtk::Box) {
                            set_orientation: gtk::Orientation::Vertical,
                        }
                    }
                }
            }
        }
    }

    additional_fields! {
        vadjustment: gtk::Adjustment,
    }

    fn pre_init() {
        let vadjustment = gtk::Adjustment::default();
        vadjustment.set_value(0.);
        vadjustment.set_upper(1360.);
        vadjustment.set_page_size(400.);
        vadjustment.set_page_increment(200.);
    }

    fn post_init() {
        for nr in 0..20 {
            let label = gtk::Label::new(Some(&format!(
                "<span size='35pt' color='red'> line          ->           {}</span>",
                nr
            )));
            label.set_use_markup(true);
            labels.append(&label);
        }
        let controllers = clamp.observe_controllers();
        let ncontrollers = controllers.n_items() as usize;
        println!("total {} controllers", ncontrollers);
        // while let Some(obj) = controllers.item(0) {
        for n in 0..ncontrollers {
            println!("------------------------------> ");
            let obj = controllers.item(n as _).unwrap();
            let controller = obj.downcast_ref::<gtk::EventController>().unwrap();
            controller.set_propagation_phase(gtk::PropagationPhase::None);
            // clamp.remove_controller(controller);
        }
        // viewport.set_vadjustment(Some(&vadjustment));
        let controller = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
        controller.connect_scroll(glib::clone!(@strong sender => move |_, x, y| {
            println!("scrolling {} {}", x, y);
            // if y > 0. {
            //     sender.send(AppMsg::Inc(y as usize)).unwrap();
            // } else {
            //     sender.send(AppMsg::Dec(y.abs() as usize)).unwrap();
            // }
            gtk::Inhibit(false)
        }));
        // controller.set_propagation_phase(gtk::PropagationPhase::Target);
        main_window.add_controller(&controller);

        let to_top_clicked = gtk::GestureClick::new();
        to_top_clicked.connect_pressed(glib::clone!(@strong sender => move |_, _, _, _| {
            sender.send(AppMsg::Top).unwrap();
        }));
        totop.add_controller(&to_top_clicked);
        // glib::source::timeout_add(std::time::Duration::from_secs(1), {
        //     let sender = sender.clone();
        //     move || {
        //         sender.send(AppMsg::Top).unwrap();
        //         glib::Continue(true)
        //     }
        // });
        // let vadjustment = viewport.vadjustment().unwrap();
        // vadjustment.set_value(200.);
        // vadjustment.set_upper(200.);
        vadjustment.set_page_increment(100.);
    }

    fn pre_view() {
        // let vadjustment = self.viewport.vadjustment().unwrap();
        let vadjustment = &self.vadjustment;
        println!("vadjustment value: {}", vadjustment.value());
        println!("vadjustment upper: {}", vadjustment.upper());
        println!("vadjustment page: {}", vadjustment.page_size());
        println!("vadjustment page-incr: {}", vadjustment.page_increment());
        // println!(
        //     "sized-bin width: {}",
        //     self.sized_bin.property::<i32>("width")
        // );
        // println!(
        //     "sized-bin height: {}",
        //     self.sized_bin.property::<i32>("height")
        // );
        println!("nr {}", model.nr as f64);
        // vadjustment.set_upper(200.);
        // vadjustment.set_page_size(100.);
        // vadjustment.set_page_increment(50.);
        vadjustment.set_value(model.nr as f64);
        vadjustment.set_page_increment(100.);
        // self.viewport.set_vadjustment(Some(&self.vadjustment));
        self.clamp.queue_allocate();
        self.clamp.queue_resize();
        self.clamp.queue_draw();
    }
}

fn main() {
    let model = AppModel { nr: 0 };
    let relm = RelmApp::new(model);
    relm.run();
}
