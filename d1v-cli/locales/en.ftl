# D1V CLI — English translations

## Authentication
auth-email-prompt = Email:
auth-email-invalid = please enter a valid email address
auth-code-sent = Verification code sent to { $email }
auth-code-prompt = Verification code:
auth-code-invalid = please enter a 6-digit code
auth-password-prompt = Password:
auth-token-prompt = Token:
auth-token-empty = Token cannot be empty.
auth-login-success = Login successful!
auth-logout-success = Logged out.
auth-not-logged-in = Not logged in.

## Debug
debug-label-version = version:
debug-label-user-agent = user-agent:
debug-label-config = config:
debug-label-base-url = base-url:
debug-label-token = token:
debug-unknown = unknown
debug-token-found = ✓ ({ $source })
debug-token-missing = ✗

## Output
error-label = Error:
hint-label = Hint:

## CLI Errors
error-not-logged-in = not logged in
hint-not-logged-in = Run `d1v auth login` to authenticate.
error-token-expired = token has expired
hint-token-expired = Run `d1v auth login` to re-authenticate.
error-network = network error
error-timeout = request timed out
hint-network = Check your internet connection and try again.
hint-timeout = The request timed out. Please try again later.
hint-config = Check your config file at ~/.d1v/config.toml.
hint-token-storage = Try running `d1v auth login` to re-authenticate.
cancelled = Cancelled.

## Token
error-no-token-store = no writable token store available
error-keyring-unavailable = keyring is not available
error-keyring-save = failed to save to keyring

## Config
error-no-home-dir = could not determine home directory
error-read-config = failed to read config file
error-write-config = failed to write config file
error-parse-config = failed to parse config file
error-serialize-config = failed to serialize config

## User
user-info-updated = User info updated.

## Password
password-new-prompt = New password:
password-confirm-prompt = Confirm password:
password-mismatch = Passwords do not match.
password-empty = Password cannot be empty.
password-set-success = Password set.
password-forgot-sent = Password reset email sent to { $email }.
password-reset-success = Password reset successful.

## Email
email-code-sent = Verification code sent to { $email }.
email-bind-success = Email bound successfully.
email-change-success = Email changed successfully.

## Invitation & Onboarding
invitation-accepted = Invitation accepted.
onboard-success = Onboarding marked as complete.
