use relm4::{MessageHandler, Sender};
// use tokio::runtime::{Builder, Runtime};
// use tokio::sync::mpsc::unbounded_channel as unbound;

use crate::{
    app::AppMessage,
    bridge::{RedrawEvent, UiCommand},
    event_aggregator::EVENT_AGGREGATOR,
    loggingchan::LoggingTx,
    running_tracker::RUNNING_TRACKER,
};

pub struct VimMessager {}

impl MessageHandler<crate::app::AppModel> for VimMessager {
    type Msg = RedrawEvent;
    type Sender = LoggingTx<UiCommand>;

    fn init(app_model: &crate::app::AppModel, parent_sender: Sender<AppMessage>) -> Self {
        let mut rx = EVENT_AGGREGATOR.register_event::<RedrawEvent>();
        let sender = parent_sender.clone();
        let running_tracker = RUNNING_TRACKER.clone();
        app_model.rt.spawn(async move {
            loop {
                tokio::select! {
                    _ = running_tracker.wait_quit() => {
                        log::info!("messager quit.");
                        sender.send(AppMessage::Quit).unwrap();
                        // 保证最后一个退出, 避免其他task还在写,这里已经关闭,报错.
                        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                        break;
                    },
                    Some(event) = rx.recv() => {
                        log::trace!("RedrawEvent {:?}", event);
                        sender
                            .send(AppMessage::RedrawEvent(event))
                            .expect("Failed to send RedrawEvent to main thread");
                    },
                    else => {
                        log::info!("messager None RedrawEvent event received, quit.");
                        sender.send(AppMessage::Quit).unwrap();
                        break;
                    },
                }
            }
        });

        VimMessager {}
    }

    fn send(&self, message: RedrawEvent) {
        EVENT_AGGREGATOR.send::<RedrawEvent>(message);
    }

    fn sender(&self) -> Self::Sender {
        unimplemented!()
        // self.sender.clone()
    }
}
