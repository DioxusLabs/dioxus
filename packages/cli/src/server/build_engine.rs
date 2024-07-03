pub struct BuildEngine {}

impl BuildEngine {
    pub fn start() -> Self {
        Self {}
    }

    /// Wait for any new updates to the builder - either it completed or gave us a mesage etc
    pub async fn wait(&mut self) {
        todo!()
    }
}
