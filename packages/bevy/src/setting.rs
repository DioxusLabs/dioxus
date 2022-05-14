use bevy::utils::Duration;

#[derive(Debug)]
pub struct DioxusSettings {
    pub focused_mode: UpdateMode,
    pub unfocused_mode: UpdateMode,
}

impl DioxusSettings {
    pub fn game() -> Self {
        DioxusSettings {
            focused_mode: UpdateMode::Continuous,
            unfocused_mode: UpdateMode::Continuous,
        }
    }

    pub fn update_mode(&self, focused: bool) -> &UpdateMode {
        match focused {
            true => &self.focused_mode,
            false => &self.unfocused_mode,
        }
    }
}

impl Default for DioxusSettings {
    fn default() -> Self {
        DioxusSettings {
            focused_mode: UpdateMode::Reactive {
                max_wait: Duration::from_secs(5),
            },
            unfocused_mode: UpdateMode::ReactiveLowPower {
                max_wait: Duration::from_secs(60),
            },
        }
    }
}

#[derive(Debug)]
pub enum UpdateMode {
    Continuous,
    Reactive { max_wait: Duration },
    ReactiveLowPower { max_wait: Duration },
}
