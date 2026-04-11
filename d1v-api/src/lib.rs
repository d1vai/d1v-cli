pub mod api;
pub mod client;
pub mod error;
pub mod jwt;
#[cfg(feature = "record")]
pub mod record;
pub mod response;
pub mod user_agent;
mod validate;

pub use crate::client::{Client, ClientBuilder, RequestBuilder};
pub use crate::error::{
    ApiCode, Error, HttpStatusError, Location, ServerValidationError, ValidationDetail,
};
pub use crate::jwt::Token;
pub use crate::response::Response;
pub use crate::user_agent::UserAgent;

pub use crate::api::user::{UpdateUser, User};
pub use crate::validate::{
    Code, CodeError, Email, EmailError, UrlError, Validate, ValidationError,
};

#[cfg(feature = "record")]
pub use crate::record::{set_recorder, Recorder, RecorderGuard, SetRecorderError};

/// Default base URL for D1V API.
pub const DEFAULT_BASE_URL: &str = "https://api.d1v.ai";
