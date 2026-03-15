use d1v_api::Client;
use httpmock::MockServer;

/// Creates a test client pointing to the given mock server.
pub fn test_client(server: &MockServer) -> Client {
    Client::new(server.base_url()).unwrap()
}
