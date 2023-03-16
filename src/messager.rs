use relm4::{prelude::*, Worker};
use tracing::{info, trace};

use crate::{
    app::{self, AppMessage},
    bridge::{RedrawEvent, UiCommand},
    event_aggregator::EVENT_AGGREGATOR,
    running_tracker::RUNNING_TRACKER,
};

pub struct VimMessager {}

pub struct RuntimeRef<'a>(&'a tokio::runtime::Runtime);

impl Worker for VimMessager {
    type Init = RuntimeRef<'_>;
    type Input = RedrawEvent;
    type Output = AppMessage;

    fn init(RuntimeRef(rt): Self::Init, sender: ComponentSender<Self>) -> Self {
        let mut rx = EVENT_AGGREGATOR.register_event::<RedrawEvent>();
        let sender = sender.clone();
        let running_tracker = RUNNING_TRACKER.clone();
        rt.spawn(async move {
            loop {
                tokio::select! {
                    _ = running_tracker.wait_quit() => {
                        info!("messager quit.");
                        sender.output(AppMessage::Quit).unwrap();
                        // 保证最后一个退出, 避免其他task还在写,这里已经关闭,报错.
                        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                        break;
                    },
                    Some(event) = rx.recv() => {
                        trace!("RedrawEvent {:?}", event);
                        sender
                            .output(AppMessage::RedrawEvent(event))
                            .expect("Failed to send RedrawEvent to main thread");
                    },
                    else => {
                        info!("messager None RedrawEvent event received, quit.");
                        sender.output(AppMessage::Quit).unwrap();
                        break;
                    },
                }
            }
        });

        VimMessager {}
    }

    fn update(&mut self, message: RedrawEvent, _: ComponentSender<Self>) {
        EVENT_AGGREGATOR.send::<RedrawEvent>(message);
    }
}
