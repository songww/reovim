#[macro_use]
extern crate derive_new;

use clap::{CommandFactory, Parser};
use relm4::RelmApp;

use crate::app::{App, AppMessage};

mod app;
mod bridge;
mod color;
mod components;
mod cursor;
mod event_aggregator;
mod factory;
mod grapheme;
mod keys;
mod loggingchan;
mod messager;
mod metrics;
mod running_tracker;
mod settings;
mod style;
mod vimview;
mod widgets;

enum ConnectionMode {
    Child,
    RemoteTcp(String),
}

#[derive(Parser, Clone, Debug, Default, PartialEq)]
pub struct Opts {
    /// Path to neovim binary
    #[clap(long = "nvim", env = "NVIM", value_name = "NVIM")]
    nvim_path: Option<String>,

    /// Remote nvim via tcp
    #[clap(long = "remote", env = "REMOTE", value_name = "HOST:PORT")]
    remote_tcp: Option<String>,

    // initial window width
    #[clap(long = "window-width", env = "WIDTH", default_value_t = 800)]
    width: i32,
    // initial window height
    #[clap(long = "window-height", env = "HEIGHT", default_value_t = 600)]
    height: i32,

    /// A level of log, see: https://docs.rs/env_logger/latest/env_logger/#enabling-logging
    #[clap(short, long, value_name = "RUST_LOG", action = clap::ArgAction::Count)]
    verbose: u8,

    /// files to open.
    #[clap(env = "FILES", value_name = "FILES")]
    files: Vec<String>,

    /// Arguments that are passed to nvim.
    #[clap(env = "ARGS", value_name = "ARGS", last = true)]
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
    use tracing::trace;
    use tracing_subscriber::{filter::LevelFilter, EnvFilter};

    let mut opts: Opts = Opts::parse();
    let levelfilter = match opts.verbose {
        0 => LevelFilter::OFF,
        1 => LevelFilter::ERROR,
        2 => LevelFilter::WARN,
        3 => LevelFilter::INFO,
        4 => LevelFilter::DEBUG,
        _ => LevelFilter::TRACE,
    };

    let subscriber = tracing_subscriber::fmt()
        .compact()
        .with_max_level(levelfilter);
    if let Ok(filter) = EnvFilter::try_from_default_env() {
        subscriber.with_env_filter(filter).init();
    } else {
        subscriber.init();
    }

    let app = Opts::command().allow_missing_positional(true);
    let title = app.get_bin_name().unwrap_or("rv");
    opts.title = title.to_string();

    trace!("opts: {:?}", opts);
    // let model = app::AppModel::new(opts);
    let relm =
        RelmApp::<AppMessage>::new("me.songww.editor.reovim").with_args(vec![title.to_string()]);

    relm.run::<App>(opts);
}
