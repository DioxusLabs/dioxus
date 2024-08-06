use cargo_mobile2::{
    apple::{
        cli::{Command, Input},
        target::Target,
    },
    opts::NoiseLevel,
    target::TargetTrait,
    util::cli::{Exec, GlobalFlags, Profile, TextWrapper},
};
use clap::Parser;

#[derive(Clone, Debug, Parser)]
pub enum Apple {
    #[clap(name = "open", about = "Open project in Xcode")]
    Open,
    #[clap(name = "check", about = "Checks if code compiles for target(s)")]
    Check {
        #[clap(name = "targets", default_value = Target::DEFAULT_KEY)]
        targets: Vec<String>,
    },
    #[clap(name = "build", about = "Builds static libraries for target(s)")]
    Build {
        #[clap(name = "targets", default_value = Target::DEFAULT_KEY)]
        targets: Vec<String>,
    },

    #[clap(name = "run", about = "Deploys IPA to connected device")]
    Run,
}

impl From<Apple> for Command {
    fn from(value: Apple) -> Self {
        match value {
            Apple::Open => todo!(),
            Apple::Check { targets } => todo!(),
            Apple::Build { targets } => todo!(),
            Apple::Run => Command::Run {
                profile: Profile {
                    profile: cargo_mobile2::opts::Profile::Debug,
                },
            },
        }
    }
}

pub fn run(cmd: Apple) {
    Input::new(
        GlobalFlags {
            noise_level: NoiseLevel::LoudAndProud,
            non_interactive: false,
        },
        cmd.into(),
    )
    .exec(&TextWrapper::default())
    .unwrap();
}
