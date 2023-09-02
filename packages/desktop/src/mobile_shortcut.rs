#![allow(unused)]

use super::*;
use wry::application::accelerator::Accelerator;
use wry::application::event_loop::EventLoopWindowTarget;

pub struct HotKey();

impl FromStr for HotKey {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(HotKey())
    }
}

pub struct ShortcutManager();

impl ShortcutManager {
    pub fn new<T>(target: &EventLoopWindowTarget<T>) -> Self {
        Self()
    }

    pub fn register(
        &mut self,
        accelerator: Accelerator,
    ) -> Result<GlobalShortcut, ShortcutManagerError> {
        Ok(GlobalShortcut())
    }

    pub fn unregister(&mut self, id: ShortcutId) -> Result<(), ShortcutManagerError> {
        Ok(())
    }

    pub fn unregister_all(&mut self) -> Result<(), ShortcutManagerError> {
        Ok(())
    }
}

use std::{error, fmt};

/// An error whose cause the `ShortcutManager` to fail.
#[non_exhaustive]
#[derive(Debug)]
pub enum ShortcutManagerError {
    AcceleratorAlreadyRegistered(Accelerator),
    AcceleratorNotRegistered(Accelerator),
    InvalidAccelerator(String),
}

impl error::Error for ShortcutManagerError {}
impl fmt::Display for ShortcutManagerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            ShortcutManagerError::AcceleratorAlreadyRegistered(e) => {
                f.pad(&format!("hotkey already registered: {:?}", e))
            }
            ShortcutManagerError::AcceleratorNotRegistered(e) => {
                f.pad(&format!("hotkey not registered: {:?}", e))
            }
            ShortcutManagerError::InvalidAccelerator(e) => e.fmt(f),
        }
    }
}

struct HotkeyError;

struct GlobalHotKeyEvent {
    id: u32,
}

pub(crate) type Code = dioxus::prelude::Code;

struct GlobalHotKeyManager {}
