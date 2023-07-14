#![allow(unused)]

use super::*;
use wry::application::accelerator::Accelerator;
use wry::application::event_loop::EventLoopWindowTarget;

pub struct GlobalShortcut();
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
