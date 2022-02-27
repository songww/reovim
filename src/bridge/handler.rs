use async_trait::async_trait;
use log::trace;
use nvim::{Handler, Neovim, Value};

//use crate::bridge::clipboard::{get_remote_clipboard, set_remote_clipboard};
#[cfg(windows)]
use crate::bridge::ui_commands::{ParallelCommand, UiCommand};
use crate::{
    bridge::{events::parse_redraw_event, TxWrapper},
    event_aggregator::EVENT_AGGREGATOR,
    running_tracker::*,
    settings::SETTINGS,
};

#[derive(Clone)]
pub struct NeovimHandler {}

impl NeovimHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl Handler for NeovimHandler {
    type Writer = TxWrapper;

    async fn handle_request(
        &self,
        event_name: String,
        _arguments: Vec<Value>,
        _neovim: Neovim<TxWrapper>,
    ) -> Result<Value, Value> {
        trace!("Neovim request: {:?}", &event_name);

        match event_name.as_ref() {
            "neovide.get_clipboard" => {
                // let endline_type = neovim
                //     .command_output("set ff")
                //     .await
                //     .ok()
                //     .and_then(|format| {
                //         let mut s = format.split('=');
                //         s.next();
                //         s.next().map(String::from)
                //     });

                // get_remote_clipboard(endline_type.as_deref())
                //     .map_err(|_| Value::from("cannot get remote clipboard content"))
                Err(Value::from("get remote clipboard ignored."))
            }
            _ => Ok(Value::from("rpcrequest not handled")),
        }
    }

    async fn handle_notify(
        &self,
        event_name: String,
        arguments: Vec<Value>,
        neovim: Neovim<TxWrapper>,
    ) {
        trace!("Neovim notification: {:?}", &event_name);

        match event_name.as_ref() {
            "redraw" => {
                for events in arguments {
                    let parsed_events = parse_redraw_event(events, neovim.clone())
                        .expect("Could not parse event from neovim");

                    log::error!(
                        "RedrawEvents: {:?}",
                        parsed_events
                            .iter()
                            .map(|event| {
                                let s = format!("{:?}", event);
                                s.split_once(" {").map(|s| s.0).unwrap_or(&s).to_string()
                            })
                            .collect::<Vec<_>>()
                    );
                    for parsed_event in parsed_events {
                        EVENT_AGGREGATOR.send(parsed_event);
                    }
                }
            }
            "setting_changed" => {
                SETTINGS.handle_changed_notification(arguments);
            }
            "neovide.quit" => {
                let error_code = arguments[0]
                    .as_i64()
                    .expect("Could not parse error code from neovim");
                RUNNING_TRACKER.quit_with_code(error_code as i32, "Quit from neovim");
            }
            #[cfg(windows)]
            "neovide.register_right_click" => {
                EVENT_AGGREGATOR.send(UiCommand::Parallel(ParallelCommand::RegisterRightClick));
            }
            #[cfg(windows)]
            "neovide.unregister_right_click" => {
                EVENT_AGGREGATOR.send(UiCommand::Parallel(ParallelCommand::UnregisterRightClick));
            }
            "neovide.set_clipboard" => {
                // set_remote_clipboard(arguments).ok();
                log::error!("set remote clipboard ignored.")
            }
            _ => {}
        }
    }
}
