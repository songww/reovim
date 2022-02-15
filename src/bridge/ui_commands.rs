use std::ops::Deref;

use log::trace;

use nvim::Neovim;

use crate::{bridge::Tx, keys::ToInput};

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

impl ToString for MouseButton {
    fn to_string(&self) -> String {
        match self {
            MouseButton::Left => "left".to_string(),
            MouseButton::Right => "right".to_string(),
            MouseButton::Middle => "middle".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum UiCommand {
    Quit,
    Resize {
        width: u64,
        height: u64,
    },
    Keyboard(String),
    MouseButton {
        action: MouseAction,
        button: MouseButton,
        modifier: gdk::ModifierType,
        grid_id: u64,
        position: (u32, u32),
    },
    Scroll {
        direction: String,
        grid_id: u64,
        position: (u32, u32),
    },
    Drag {
        grid_id: u64,
        position: (u32, u32),
    },
    FileDrop(String),
    FocusLost,
    FocusGained,
}

impl UiCommand {
    pub async fn execute(self, nvim: &Neovim<Tx>) {
        match self {
            UiCommand::Quit => {
                nvim.command("qa!").await.ok();
            }
            UiCommand::Resize { width, height } => nvim
                .ui_try_resize(width.max(10) as i64, height.max(3) as i64)
                .await
                .expect(&format!(
                    "Resize failed, trying resize to {}x{}",
                    width.max(10) as i64,
                    height.max(3) as i64
                )),
            UiCommand::Keyboard(input_command) => {
                trace!("Keyboard Input Sent: {}", input_command);
                nvim.input(&input_command).await.expect("Input failed");
            }
            UiCommand::MouseButton {
                action,
                button,
                modifier,
                grid_id,
                position: (grid_x, grid_y),
            } => {
                nvim.input_mouse(
                    &button,
                    &action,
                    &modifier.to_input().unwrap(),
                    grid_id as i64,
                    grid_y as i64,
                    grid_x as i64,
                )
                .await
                .expect("Mouse Input Failed");
            }
            UiCommand::Scroll {
                direction,
                grid_id,
                position: (grid_x, grid_y),
            } => {
                nvim.input_mouse(
                    "wheel",
                    &direction,
                    "",
                    grid_id as i64,
                    grid_y as i64,
                    grid_x as i64,
                )
                .await
                .expect("Mouse Scroll Failed");
            }
            UiCommand::Drag {
                grid_id,
                position: (grid_x, grid_y),
            } => {
                nvim.input_mouse(
                    "left",
                    "drag",
                    "",
                    grid_id as i64,
                    grid_y as i64,
                    grid_x as i64,
                )
                .await
                .expect("Mouse Drag Failed");
            }
            UiCommand::FocusLost => nvim
                .command("if exists('#FocusLost') | doautocmd <nomodeline> FocusLost | endif")
                .await
                .expect("Focus Lost Failed"),
            UiCommand::FocusGained => nvim
                .command("if exists('#FocusGained') | doautocmd <nomodeline> FocusGained | endif")
                .await
                .expect("Focus Gained Failed"),
            UiCommand::FileDrop(path) => {
                nvim.command(format!("e {}", path).as_str()).await.ok();
            }
        }
    }
}
