#![feature(new_uninit)]
#![feature(maybe_uninit_write_slice)]
#![feature(round_char_boundary)]
#[macro_use]
extern crate derive_new;
#[macro_use]
extern crate derivative;

use std::cell::Cell;

use clap::{IntoApp, Parser};

mod app;
mod bridge;
mod color;
mod components;
mod cursor;
mod event_aggregator;
mod factory;
mod keys;
mod loggingchan;
mod messager;
mod metrics;
mod pos;
mod rect;
mod running_tracker;
mod settings;
mod style;
mod vimview;

#[macro_export]
macro_rules! cloned {
    (@param _) => ( _ );
    (@param $x:ident) => ( $x );
    ($($n:ident),+ => move || $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move || $body
        }
    );
    ($($n:ident),+ => move |$($p:tt),+| $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move |$(cloned!(@param $p),)+| $body
        }
    );
}

enum ConnectionMode {
    Child,
    RemoteTcp(String),
}

#[derive(Parser, Clone, Debug, Default, PartialEq)]
pub struct Opts {
    /// Path to neovim binary
    #[clap(long = "nvim", value_name = "PATH")]
    nvim_path: Option<String>,

    /// Remote nvim via tcp
    #[clap(long = "remote", value_name = "HOST:PORT")]
    remote_tcp: Option<String>,

    // initial window width
    #[clap(long = "window-width", default_value_t = 800)]
    width: i32,
    // initial window height
    #[clap(long = "window-height", default_value_t = 600)]
    height: i32,

    /// A level of verbosity, and can be used multiple times
    #[clap(short, long, parse(from_occurrences))]
    verbose: i32,

    /// Arguments that are passed to nvim.
    #[clap(value_name = "ARGS", last = true)]
    nvim_args: Vec<String>,

    #[clap(skip)]
    title: String,

    #[clap(skip)]
    size: Option<(i64, i64)>,
}

impl Opts {
    fn connection_mode(&self) -> ConnectionMode {
        if let Some(ref remote) = self.remote_tcp {
            ConnectionMode::RemoteTcp(remote.to_owned())
        } else {
            ConnectionMode::Child
        }
    }
}

fn main() {
    let mut opts: Opts = Opts::parse();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log::trace!("command line options: {:?}", opts);
    let app = Opts::command().allow_missing_positional(true);
    let title = app.get_bin_name().unwrap_or("rv");
    opts.title = title.to_string();
    log::trace!("opts: {:?}", opts);
    let model = app::AppModel::new(opts);
    let relm = relm4::RelmApp::new(model);
    // app.connect_startup(f)
    // app.connect_window_added(|_, win| {
    //     WIN.set(Fragile::new(win.clone()))
    //         .expect("WIN was alredy set");
    //     log::info!("window add: {}", win);
    // });
    // app.set_flags(ApplicationFlags::NON_UNIQUE | ApplicationFlags::HANDLES_OPEN);
    // app.set_flags(ApplicationFlags::HANDLES_OPEN);
    // app.set_option_context_parameter_string(Some("Some Param"));
    // app.set_option_context_description(Some("Some Desc"));
    // app.set_option_context_summary(Some(&message));
    // app.connect_command_line(|_, cl| {print!("---- command line {:?}", cl.arguments()); 0});
    // app.connect_handle_local_options(move |_, opts| {
    //     opts.remove("nvim");
    //     for arg in opts_.nvim_args.iter() {
    //         opts.remove(arg);
    //     }
    //     for f in opts_.files.iter() {
    //         println!("Removing {}", f);
    //         opts.remove(f);
    //     }
    //     -1
    // });
    relm.run_with_args(&[title]);
}
