#[macro_use]
extern crate derive_new;
#[macro_use]
extern crate derivative;

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

    relm.run_with_args(&[title]);
}
