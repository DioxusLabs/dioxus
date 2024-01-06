#![allow(unused)]

use super::*;
use std::str::FromStr;
use tao::event_loop::EventLoopWindowTarget;

use dioxus_html::input_data::keyboard_types::Modifiers;

#[derive(Clone, Debug)]
pub struct Accelerator;

#[derive(Clone, Copy)]
pub struct HotKey;

impl HotKey {
    pub fn new(mods: Option<Modifiers>, key: Code) -> Self {
        Self
    }

    pub fn id(&self) -> u32 {
        0
    }
}

impl FromStr for HotKey {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(HotKey)
    }
}

pub struct GlobalHotKeyManager();

impl GlobalHotKeyManager {
    pub fn new() -> Result<Self, HotkeyError> {
        Ok(Self())
    }

    pub fn register(&self, accelerator: HotKey) -> Result<HotKey, HotkeyError> {
        Ok(HotKey)
    }

    pub fn unregister(&self, id: HotKey) -> Result<(), HotkeyError> {
        Ok(())
    }

    pub fn unregister_all(&self, _: &[HotKey]) -> Result<(), HotkeyError> {
        Ok(())
    }
}

use std::{error, fmt};

/// An error whose cause the `ShortcutManager` to fail.
#[non_exhaustive]
#[derive(Debug)]
pub enum HotkeyError {
    AcceleratorAlreadyRegistered(Accelerator),
    AcceleratorNotRegistered(Accelerator),
    HotKeyParseError(String),
}

impl error::Error for HotkeyError {}
impl fmt::Display for HotkeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            HotkeyError::AcceleratorAlreadyRegistered(e) => {
                f.pad(&format!("hotkey already registered: {:?}", e))
            }
            HotkeyError::AcceleratorNotRegistered(e) => {
                f.pad(&format!("hotkey not registered: {:?}", e))
            }
            HotkeyError::HotKeyParseError(e) => e.fmt(f),
        }
    }
}

pub struct GlobalHotKeyEvent {
    pub id: u32,
}

impl GlobalHotKeyEvent {
    pub fn receiver() -> crossbeam_channel::Receiver<GlobalHotKeyEvent> {
        crossbeam_channel::unbounded().1
    }
}

pub(crate) type Code = dioxus_html::input_data::keyboard_types::Code;
