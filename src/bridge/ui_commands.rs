use std::ops::Deref;
use std::sync::Arc;

use nvim::{call_args, rpc::model::IntoVal, Neovim};
use tokio::sync::mpsc::unbounded_channel;

#[cfg(windows)]
use crate::windows_utils::{
    register_rightclick_directory, register_rightclick_file, unregister_rightclick,
};
use crate::{
    bridge::TxWrapper, event_aggregator::EVENT_AGGREGATOR, keys::ToInput,
    running_tracker::RUNNING_TRACKER,
};

#[derive(Clone, Copy, Debug)]
pub enum MouseAction {
    Drag,
    Press,
    Release,
    // For the wheel
    Up,
    Down,
    Left,
    Right,
}

impl AsRef<str> for MouseAction {
    fn as_ref(&self) -> &str {
        match self {
            MouseAction::Drag => "drag",
            MouseAction::Press => "press",
            MouseAction::Release => "release",

            MouseAction::Up => "up",
            MouseAction::Down => "down",
            MouseAction::Left => "left",
            MouseAction::Right => "right",
        }
    }
}

impl Deref for MouseAction {
    type Target = str;
    fn deref(&self) -> &str {
        match self {
            MouseAction::Drag => "drag",
            MouseAction::Press => "press",
            MouseAction::Release => "release",

            MouseAction::Up => "up",
            MouseAction::Down => "down",
            MouseAction::Left => "left",
            MouseAction::Right => "right",
        }
    }
}

impl std::fmt::Display for MouseAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self)
    }
}

#[derive(Copy, Clone, Debug)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

impl AsRef<str> for MouseButton {
    fn as_ref(&self) -> &str {
        match self {
            MouseButton::Left => "left",
            MouseButton::Right => "right",
            MouseButton::Middle => "middle",
        }
    }
}

impl Deref for MouseButton {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        match self {
            MouseButton::Left => "left",
            MouseButton::Right => "right",
            MouseButton::Middle => "middle",
        }
    }
}

impl std::fmt::Display for MouseButton {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        match self {
            MouseButton::Left => f.write_str("left"),
            MouseButton::Right => f.write_str("right"),
            MouseButton::Middle => f.write_str("middle"),
        }
    }
}

// Serial commands are any commands which must complete before the next value is sent. This
// includes keyboard and mouse input which would cause problems if sent out of order.
//
// When in doubt, use Parallel Commands.
#[derive(Clone, Debug)]
pub enum SerialCommand {
    Keyboard(String),
    MouseButton {
        action: MouseAction,
        button: MouseButton,
        modifier: gtk::gdk::ModifierType,
        grid_id: u64,
        position: (u32, u32),
    },
    Scroll {
        direction: String,
        grid_id: u64,
        position: (u32, u32),
        modifier: gtk::gdk::ModifierType,
    },
    Drag {
        button: MouseButton,
        grid_id: u64,
        position: (u32, u32),
        modifier: gtk::gdk::ModifierType,
    },
}

impl SerialCommand {
    async fn execute(self, nvim: &Neovim<TxWrapper>) {
        match self {
            SerialCommand::Keyboard(input_command) => {
                log::trace!("Keyboard Input Sent: {}", input_command);
                nvim.input(&input_command).await.expect("Input failed");
            }
            SerialCommand::MouseButton {
                action,
                button,
                modifier,
                grid_id,
                position: (grid_x, grid_y),
            } => {
                let action: &str = &action;
                let button: &str = &button;
                let modifier: &str = &modifier.to_input().unwrap();
                log::trace!(
                    "input mouse button='{}' action='{}' modifier='{}' {}<({}, {})>",
                    button,
                    action,
                    modifier,
                    grid_id,
                    grid_x,
                    grid_y
                );
                nvim.input_mouse(
                    button,
                    action,
                    "",
                    grid_id as i64,
                    grid_y as i64,
                    grid_x as i64,
                )
                .await
                .expect("Mouse Input Failed");
            }
            SerialCommand::Scroll {
                direction,
                grid_id,
                position: (grid_x, grid_y),
                modifier,
            } => {
                log::trace!(
                    "Mouse Wheel Sent: {} {}<{:?}> ({})",
                    direction,
                    grid_id,
                    (grid_x, grid_y),
                    AsRef::<str>::as_ref(&modifier.to_input().unwrap()),
                );
                nvim.input_mouse(
                    "wheel",
                    &direction,
                    &modifier.to_input().unwrap(),
                    grid_id as i64,
                    grid_y as i64,
                    grid_x as i64,
                )
                .await
                .expect("Mouse Scroll Failed");
            }
            SerialCommand::Drag {
                button,
                grid_id,
                position: (grid_x, grid_y),
                modifier,
            } => {
                nvim.input_mouse(
                    &button,
                    "drag",
                    &modifier.to_input().unwrap(),
                    grid_id as i64,
                    grid_y as i64,
                    grid_x as i64,
                )
                .await
                .expect("Mouse Drag Failed");
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum ParallelCommand {
    Quit,
    Resize {
        width: u64,
        height: u64,
    },
    FileDrop(String),
    FocusLost,
    FocusGained,
    DisplayAvailableFonts(Vec<String>),
    #[cfg(windows)]
    RegisterRightClick,
    #[cfg(windows)]
    UnregisterRightClick,
}

impl ParallelCommand {
    async fn execute(self, nvim: &Neovim<TxWrapper>) {
        match self {
            ParallelCommand::Quit => {
                nvim.command("qa!").await.ok();
            }
            ParallelCommand::Resize { width, height } => nvim
                .ui_try_resize(width.max(10) as i64, height.max(3) as i64)
                .await
                .expect("Resize failed"),
            ParallelCommand::FocusLost => nvim
                .command("if exists('#FocusLost') | doautocmd <nomodeline> FocusLost | endif")
                .await
                .expect("Focus Lost Failed"),
            ParallelCommand::FocusGained => nvim
                .command("if exists('#FocusGained') | doautocmd <nomodeline> FocusGained | endif")
                .await
                .expect("Focus Gained Failed"),
            ParallelCommand::FileDrop(path) => {
                nvim.command(format!("e {}", path).as_str()).await.ok();
            }
            ParallelCommand::DisplayAvailableFonts(fonts) => {
                let mut content: Vec<String> = vec![
                    "What follows are the font names available for guifont. You can try any of them with <CR> in normal mode.",
                    "",
                    "To switch to one of them, use one of them, type:",
                    "",
                    "    :set guifont=<font name>:h<font size>",
                    "",
                    "where <font name> is one of the following with spaces escaped",
                    "and <font size> is the desired font size. As an example:",
                    "",
                    "    :set guifont=Cascadia\\ Code\\ PL:h12",
                    "",
                    "You may specify multiple fonts for fallback purposes separated by commas like so:",
                    "",
                    "    :set guifont=Cascadia\\ Code\\ PL,Delugia\\ Nerd\\ Font:h12",
                    "",
                    "Make sure to add the above command when you're happy with it to your .vimrc file or similar config to make it permanent.",
                    "------------------------------",
                    "Available Fonts on this System",
                    "------------------------------",
                ].into_iter().map(|text| text.to_owned()).collect();
                content.extend(fonts);

                nvim.command("split").await.ok();
                nvim.command("noswapfile hide enew").await.ok();
                nvim.command("setlocal buftype=nofile").await.ok();
                nvim.command("setlocal bufhidden=hide").await.ok();
                nvim.command("\"setlocal nobuflisted").await.ok();
                nvim.command("\"lcd ~").await.ok();
                nvim.command("file scratch").await.ok();
                nvim.call(
                    "nvim_buf_set_lines",
                    call_args![0i64, 0i64, -1i64, false, content],
                )
                .await
                .ok();
                nvim.command(
                    "nnoremap <buffer> <CR> <cmd>lua vim.opt.guifont=vim.fn.getline('.')<CR>",
                )
                .await
                .ok();
            }
            #[cfg(windows)]
            ParallelCommand::RegisterRightClick => {
                if unregister_rightclick() {
                    let msg =
                        "Could not unregister previous menu item. Possibly already registered.";
                    nvim.err_writeln(msg).await.ok();
                    log::error!("{}", msg);
                }
                if !register_rightclick_directory() {
                    let msg = "Could not register directory context menu item. Possibly already registered.";
                    nvim.err_writeln(msg).await.ok();
                    log::error!("{}", msg);
                }
                if !register_rightclick_file() {
                    let msg =
                        "Could not register file context menu item. Possibly already registered.";
                    nvim.err_writeln(msg).await.ok();
                    log::error!("{}", msg);
                }
            }
            #[cfg(windows)]
            ParallelCommand::UnregisterRightClick => {
                if !unregister_rightclick() {
                    let msg = "Could not remove context menu items. Possibly already removed.";
                    nvim.err_writeln(msg).await.ok();
                    log::error!("{}", msg);
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum UiCommand {
    Serial(SerialCommand),
    Parallel(ParallelCommand),
}

impl From<SerialCommand> for UiCommand {
    fn from(serial: SerialCommand) -> Self {
        UiCommand::Serial(serial)
    }
}

impl From<ParallelCommand> for UiCommand {
    fn from(parallel: ParallelCommand) -> Self {
        UiCommand::Parallel(parallel)
    }
}

pub fn start_ui_command_handler(nvim: Arc<Neovim<TxWrapper>>) {
    let (serial_tx, mut serial_rx) = unbounded_channel::<SerialCommand>();
    let ui_command_nvim = nvim.clone();
    let running_tracker = RUNNING_TRACKER.clone();
    tokio::spawn(async move {
        let mut ui_command_receiver = EVENT_AGGREGATOR.register_event::<UiCommand>();
        loop {
            tokio::select! {
                _ = running_tracker.wait_quit() => {
                    log::info!("ui command executor quit.");
                    break;
                }
                Some(ui_command) = ui_command_receiver.recv() => {
                    match ui_command {
                        UiCommand::Serial(serial_command) => serial_tx
                            .send(serial_command)
                            .expect("Could not send serial ui command"),
                        UiCommand::Parallel(parallel_command) => {
                            let ui_command_nvim = ui_command_nvim.clone();
                            tokio::spawn(async move {
                                log::trace!("aggregated parallel ui-command");
                                parallel_command.execute(&ui_command_nvim).await;
                            });
                        }
                    }
                },
                else => {
                    running_tracker.quit("ui command channel failed");
                },
            }
        }
    });

    let running_tracker = RUNNING_TRACKER.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ =  running_tracker.wait_quit() => {
                    log::info!("serial ui command executor quit.");
                    break;
                },
                Some(serial_command) = serial_rx.recv() => {
                    log::trace!("aggregated serial ui-command");
                    serial_command.execute(&nvim).await;
                },
                else => {
                    running_tracker.quit("serial ui command channel failed");
                    break;
                },
            }
        }
        /*
        while RUNNING_TRACKER.is_running() {
            match serial_rx.recv().await {
                Some(serial_command) => {
                    log::trace!("aggregated serial ui-command");
                    serial_command.execute(&nvim).await;
                }
                None => {
                    RUNNING_TRACKER.quit("serial ui command channel failed");
                }
            }
        }
        */
    });
}
