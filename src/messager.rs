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
        app_model.rt.spawn(async move {
            while let Some(event) = rx.recv().await {
                log::debug!("RedrawEvent");
                sender.send(AppMessage::RedrawEvent(event)).unwrap();
                tokio::task::yield_now().await;
            }
        });

        app_model.rt.spawn(async move {
            while RUNNING_TRACKER.is_running() {
                tokio::time::sleep(std::time::Duration::from_millis(20)).await
            }
            parent_sender.send(AppMessage::Quit).unwrap();
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
