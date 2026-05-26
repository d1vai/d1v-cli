use d1v_api::Client;
use httpmock::MockServer;

pub fn test_client(server: &MockServer) -> Client {
    Client::builder()
        .base_url(server.base_url())
        .build()
        .unwrap()
}

pub fn authed_client(server: &MockServer) -> Client {
    Client::builder()
        .base_url(server.base_url())
        .token("token123")
        .build()
        .unwrap()
}
