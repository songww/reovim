use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt};
use relm4::{
    send, AppUpdate, Components, MessageHandler, Model, RelmApp, RelmMsgHandler, Sender,
    WidgetPlus, Widgets,
};
// use tokio::runtime::{Builder, Runtime};
use tokio::sync::mpsc::{
    channel, unbounded_channel as unbound, Sender as TokioSender, UnboundedSender,
};

use crate::{
    app::AppMessage,
    bridge::{self, RedrawEvent, UiCommand},
    channel::LoggingTx,
};

pub struct VimMessager {
    // rt: Runtime,
    // bridge: Bridge,
    sender: LoggingTx<UiCommand>,
}

impl MessageHandler<crate::app::AppModel> for VimMessager {
    type Msg = RedrawEvent;
    type Sender = LoggingTx<UiCommand>;

    fn init(app_model: &crate::app::AppModel, parent_sender: Sender<AppMessage>) -> Self {
        let (sender, mut rx) = unbound::<RedrawEvent>();
        let (ui_command_sender, ui_command_receiver) = unbound::<UiCommand>();

        app_model.rt.spawn(async move {
            while let Some(event) = rx.recv().await {
                parent_sender.send(AppMessage::RedrawEvent(event)).unwrap();
            }
        });

        app_model.rt.spawn(bridge::start_neovim_runtime(
            /* ui_command_sender */
            LoggingTx::attach(ui_command_sender.clone(), "chan-ui-commands".to_string()),
            ui_command_receiver,
            /* redraw_event_sender */
            LoggingTx::attach(sender, "chan-redraw-events".to_string()),
            /* running */ std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true)),
            /* opts */ app_model.opts.clone(),
        ));

        VimMessager {
            sender: LoggingTx::attach(ui_command_sender, "chan-ui-commands".to_string()),
        }
    }

    fn send(&self, message: Self::Msg) {
        log::error!("Ignored message: {:?}", message);
        unimplemented!();
        /*
        match msg {
            AppMsg::UiCommand(command) => {
                self.bridge.rt.block_on(||async {
                    self.sender.send(msg).unwrap();
                });
            }
            _ => {}
        }
        */
    }

    fn sender(&self) -> Self::Sender {
        self.sender.clone()
    }
}
