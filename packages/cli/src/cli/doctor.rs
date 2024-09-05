use clap::Parser;

#[derive(Clone, Debug, Parser)]
pub struct Doctor {}

impl Doctor {
    pub async fn run(self) -> anyhow::Result<()> {
        Ok(())
    }
}
