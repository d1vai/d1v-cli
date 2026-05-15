pub mod api;
pub mod client;
mod encode;
pub mod error;
pub mod jwt;
pub mod locale;
mod multipart;
#[cfg(feature = "record")]
pub mod record;
pub mod response;
pub mod user_agent;
mod validate;

pub use crate::client::{Client, ClientBuilder, RequestBuilder};
pub use crate::error::{
    ApiCode, ApiError, BadRequestKind, Error, HttpStatusError, Location, ServerValidationError,
    ValidationDetail,
};
pub use crate::jwt::Token;
pub use crate::locale::{IntoLocale, Locale, ParseLocaleError};
pub use crate::response::Response;
pub use crate::user_agent::UserAgent;

pub use crate::api::user::{DailyCount, PromptDailyActivity, UpdateUser, User};
pub use crate::api::{
    db::{
        DbAffectedResponse, DbBranch, DbColumnSchema, DbCreateTableRequest, DbDataOptions,
        DbDeleteRowsRequest, DbForeignKeySchema, DbMessageResponse, DbRenameTableRequest,
        DbRowsOptions, DbSchemaOptions, DbSchemaResponse, DbTableColumnInput, DbTableSchema,
        DbUpdateRowsRequest, DbValuesRequest, MigrationApprovalRequest, MigrationApprovalResponse,
        MigrationAutoReviewResponse, MigrationExecuteRequest, MigrationExecuteResponse,
        MigrationHistoryResponse, MigrationPlanRequest, MigrationPlanResponse,
        MigrationValidateRequest, MigrationValidateResponse, ProjectTokenRequest,
        ProjectTokenResponse,
    },
    deployment::{
        DeploymentInfo, DeploymentListOptions, DeploymentListResponse, DeploymentLogsResponse,
        DeploymentResponse,
    },
    github_app::{
        GitHubAppConnectUrl, GitHubAppInstallation, GitHubAppRepository, GitHubAppStatus,
        GitHubImportAutoDeploy, GitHubImportRequest, GitHubImportResponse, GitHubProjectCliAccess,
        GitHubProjectGitCredential,
    },
    github_ops::{PullWorkspaceRequest, PullWorkspaceResponse},
    project::{
        CreateProject, CreateProjectResponse, CreateProjectWithIntegrations, LocalImportUploadFile,
        ProjectTemplateInfo, UpdateProject, UserProject,
    },
    session::{
        ChatHistoryEntry, ExecuteSessionRequest, ExecuteSessionResponse, HistoryOptions,
        ProjectSession,
    },
};
pub use crate::validate::{
    Code, CodeError, Email, EmailError, UrlError, Validate, ValidationError,
};

#[cfg(feature = "record")]
pub use crate::record::{Recorder, RecorderGuard, SetRecorderError, set_recorder};

/// Default base URL for D1V API.
pub const DEFAULT_BASE_URL: &str = "https://api.d1v.ai";
