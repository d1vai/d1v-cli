pub mod api;
pub mod client;
pub mod error;
#[cfg(feature = "record")]
pub mod record;
pub mod response;
pub mod user_agent;

pub use crate::client::{Client, ClientBuilder, RequestBuilder};
pub use crate::error::{Error, HttpStatusError, Location, ValidationDetail, ValidationError};
pub use crate::response::Response;
pub use crate::user_agent::UserAgent;

#[cfg(feature = "record")]
pub use crate::record::{set_recorder, Recorder, RecorderGuard, SetRecorderError};

/// Default base URL for D1V API.
pub const DEFAULT_BASE_URL: &str = "https://api.d1v.ai";
