use dioxus::prelude::*;
use openidconnect::{core::CoreClient, ClientId};

#[derive(Props, Clone, Debug)]
pub struct ClientProps {
    pub client: CoreClient,
    pub client_id: ClientId,
}

impl PartialEq for ClientProps {
    fn eq(&self, other: &Self) -> bool {
        self.client_id == other.client_id
    }
}

impl ClientProps {
    pub fn new(client_id: ClientId, client: CoreClient) -> Self {
        ClientProps { client_id, client }
    }
}
