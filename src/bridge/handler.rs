use std::sync::Arc;

use async_trait::async_trait;
use log::trace;
use nvim::{Handler, Neovim, Value};
use tokio::sync::Mutex;

use super::events::{parse_redraw_event, RedrawEvent};
use super::ui_commands::UiCommand;
use super::Tx;
use crate::channel::LoggingTx;

#[derive(Clone)]
pub struct NeovimHandler {
    ui_command_sender: Arc<Mutex<LoggingTx<UiCommand>>>,
    redraw_event_sender: Arc<Mutex<LoggingTx<RedrawEvent>>>,
}

impl NeovimHandler {
    pub fn new(
        ui_command_sender: LoggingTx<UiCommand>,
        redraw_event_sender: LoggingTx<RedrawEvent>,
    ) -> NeovimHandler {
        NeovimHandler {
            ui_command_sender: Arc::new(Mutex::new(ui_command_sender)),
            redraw_event_sender: Arc::new(Mutex::new(redraw_event_sender)),
        }
    }
}

#[async_trait]
impl Handler for NeovimHandler {
    type Writer = Tx;

    async fn handle_notify(&self, event_name: String, arguments: Vec<Value>, neovim: Neovim<Tx>) {
        trace!("Neovim notification: {:?}", &event_name);

        #[cfg(windows)]
        let ui_command_sender = self.ui_command_sender.clone();

        let redraw_event_sender = self.redraw_event_sender.clone();
        match event_name.as_ref() {
            "redraw" => {
                for events in arguments {
                    let parsed_events = parse_redraw_event(events, neovim.clone())
                        .expect("Could not parse event from neovim");

                    for parsed_event in parsed_events {
                        let redraw_event_sender = redraw_event_sender.lock().await;
                        redraw_event_sender.send(parsed_event).ok();
                    }
                }
            }
            "setting_changed" => {
                // SETTINGS.handle_changed_notification(arguments);
            }
            #[cfg(windows)]
            "neovide.register_right_click" => {
                let ui_command_sender = ui_command_sender.lock().await;
                ui_command_sender.send(UiCommand::RegisterRightClick).ok();
            }
            #[cfg(windows)]
            "neovide.unregister_right_click" => {
                let ui_command_sender = ui_command_sender.lock().await;
                ui_command_sender.send(UiCommand::UnregisterRightClick).ok();
            }
            _ => {}
        };
    }
}
