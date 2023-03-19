use relm4::prelude::*;
use tracing::{info, trace};

use crate::{
    app::AppMessage,
    bridge::{RedrawEvent, UiCommand},
    event_aggregator::EVENT_AGGREGATOR,
    running_tracker::RUNNING_TRACKER,
};

#[derive(Debug)]
pub struct VimMessager {}

impl Component for VimMessager {
    type Init = ();
    type Input = UiCommand;
    type Output = AppMessage;
    type Root = ();
    type Widgets = ();
    type CommandOutput = AppMessage;

    fn init_root() -> Self::Root {
        ()
    }

    fn init(_: Self::Init, _: &Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let mut rx = EVENT_AGGREGATOR.register_event::<RedrawEvent>();
        // let sender = sender.clone();
        let running_tracker = RUNNING_TRACKER.clone();

        sender.command(|sender, _| async move {
            loop {
                tokio::select! {
                    _ = running_tracker.wait_quit() => {
                        info!("messager quit.");
                        sender.send(AppMessage::Quit).unwrap();
                        // 保证最后一个退出, 避免其他task还在写,这里已经关闭,报错.
                        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                        break;
                    },
                    Some(event) = rx.recv() => {
                        trace!("RedrawEvent {:?}", event);
                        sender
                            .send(AppMessage::RedrawEvent(event))
                            .expect("Failed to send RedrawEvent to main thread");
                    },
                    else => {
                        info!("messager None RedrawEvent event received, quit.");
                        sender.send(AppMessage::Quit).unwrap();
                        break;
                    },
                }
            }
        });

        ComponentParts {
            model: VimMessager {},
            widgets: (),
        }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>, _: &Self::Root) {
        EVENT_AGGREGATOR.send::<UiCommand>(message);
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        trace!("sending out {:?}", message);
        sender.output(message).unwrap();
    }
}
