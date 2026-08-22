# D1V CLI — English translations

## Authentication
auth-email-prompt = Email:
auth-code-sent = Verification code sent to { $email }
auth-code-prompt = Verification code:
auth-password-prompt = Password:
auth-token-prompt = Token:
auth-token-empty = Token cannot be empty.
auth-login-success = Login successful!
auth-logout-success = Logged out.
auth-not-logged-in = Not logged in.
auth-status-logged-in = Logged in
auth-status-not-logged-in = Not logged in
auth-status-expired = Token expired
auth-status-label-user = user
auth-status-label-expires = expires in
warn-token-expiring = Token expires in { $duration }. Run `d1v auth login` to refresh.
auth-relogin-prompt = Token expired. Log in again?
auth-relogin-yes = Yes, log in again
auth-relogin-no = No, exit
auth-relogin-success = Re-authenticated successfully!
auth-method-prompt = Login method
auth-method-code = Verification code
auth-method-password = Password
auth-method-token = Authentication token
auth-method-api-key = API key
auth-api-key-prompt = API key:
auth-api-key-empty = API key cannot be empty.

## API keys
api-key-label-id = id
api-key-label-name = name
api-key-label-prefix = prefix
api-key-label-description = description
api-key-label-created = created
api-key-label-last-used = last used
api-key-empty-list = No API keys found.
api-key-not-found-id = API key with id { $id } not found.
api-key-not-found-name = API key "{ $name }" not found.
api-key-create-name-required = API key name is required. Pass a name or run interactively.
api-key-create-name-empty = API key name cannot be empty.
api-key-create-name-prompt = API key name:
api-key-create-desc-prompt = Description (optional):
api-key-create-success = Created API key "{ $name }" ({ $prefix }).
api-key-create-once-warning = ⚠ This key will NOT be shown again. Copy it now.
api-key-revoke-confirm-required = The --yes flag is required to revoke an API key non-interactively.
api-key-revoke-confirm-prompt = Revoke API key "{ $name }" ({ $prefix })?
api-key-revoke-confirm-yes = Yes, revoke it
api-key-revoke-confirm-no = No, keep it
api-key-revoke-success = Revoked API key "{ $name }".
api-key-save-prompt = Save this API key?
api-key-save-yes = Save as current credential
api-key-save-skip = Don't save, exit
api-key-save-saved = API key saved as current credential.

## Debug
debug-label-version = version:
debug-label-user-agent = user-agent:
debug-label-locale = locale:
debug-label-features = features:
debug-label-config = config:
debug-label-log-dir = log-dir:
debug-label-base-url = base-url:
debug-label-token = token:
debug-unknown = unknown
debug-features-none = none
debug-token-found = { $source }
debug-token-expires-in = expires in { $duration }
debug-token-expired = expired

## CLI Errors
error-not-logged-in = not logged in
hint-not-logged-in = Run `d1v auth login` to authenticate.
error-token-expired = token has expired
hint-token-expired = Run `d1v auth login` to re-authenticate.
error-network = network error
error-timeout = request timed out
error-connection-failed = could not connect to the server
hint-network = Check your internet connection and try again.
hint-timeout = The request timed out. Please try again later.
hint-connection = Check the server URL and your network connection. Run `d1v debug` to verify.
error-http-status = unexpected server response
error-invalid-response = invalid response data
error-invalid-url = invalid server URL
error-invalid-base-url = invalid server URL "{ $value }" from { $source }
hint-invalid-base-url-cli = Pass a valid URL or omit the flag.
hint-invalid-base-url-env = Unset `D1V_BASE_URL` or set it to a valid URL.
hint-invalid-base-url-config = Update `base_url` in ~/.d1v/config.toml.
error-server-validation = server validation failed

## API Error Codes
api-error-bad-request = bad request
api-error-bad-request-message = bad request ({ $message })
api-error-password-not-set = password not set
api-error-invalid-credentials = invalid email or password
api-error-email-required-before-password = bind an email before setting a password
api-error-invalid-code = invalid verification code
api-error-code-expired = verification code expired
api-error-code-invalid-or-expired = invalid or expired verification code
api-error-user-not-found = user not found
api-error-password-too-short = password is too short
api-error-email-in-use = email already in use
api-error-email-not-bound = email is not bound
api-error-invite-own-code = cannot accept your own invite code
api-error-invite-invalid = invalid invite code
api-error-invite-expired = invite code expired
api-error-invite-capacity = invite code capacity reached
api-error-invite-limit = invite limit reached for this code
api-error-invite-not-bound = invite code is not bound to an inviter
api-error-inviter-not-found = inviter not found
api-error-auth-required = authentication required
api-error-auth-required-message = authentication required ({ $message })
api-error-permission-denied = permission denied
api-error-permission-denied-message = permission denied ({ $message })
api-error-insufficient-privileges = requires a super-admin account
api-error-unknown = server error { $code } ({ $message })
api-error-unknown-code = server error { $code }

hint-config = Check your config file at ~/.d1v/config.toml.
hint-token-storage = Try running `d1v auth login` to re-authenticate.
canceled = Cancelled.
interrupted = Interrupted.

## Token
error-no-token-store = no writable token store available
error-keyring-unavailable = keyring is not available
error-keyring-load = failed to load from keyring
error-keyring-load-timeout = timed out while loading from keyring
error-keyring-save = failed to save to keyring
error-keyring-delete = failed to delete from keyring

## Config
error-no-home-dir = could not determine home directory
error-read-config = failed to read config file
error-write-config = failed to write config file
error-parse-config = failed to parse config file
error-serialize-config = failed to serialize config
error-invalid-config-value = invalid value for { $key }: { $value }
config-set-success = { $key } = { $value }
config-reset-success = Configuration reset to defaults.
config-edit-failed = failed to open config file

## Validation
validation-email-required = email address is required
validation-email-invalid = invalid email address
validation-code-required = verification code is required
validation-code-length = verification code must be 6 digits
validation-code-digit = verification code must contain only digits
validation-url-invalid = invalid URL

## User
user-info-updated = User info updated.
user-update-field-prompt = What to update
user-update-field-company-name = Company name
user-update-field-company-website = Company website
user-update-field-picture = Avatar URL
user-update-field-industry = Industry
user-update-field-referral-code = Referral code
user-label-id = id:
user-label-slug = slug:
user-label-email = email:
user-label-roles = roles:
user-label-company = company:
user-label-website = website:
user-label-industry = industry:
user-label-invite-code = invite code:

## Activity
activity-label-period = period:
activity-label-days = days:

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

## Confirm
confirm-yes = Yes
confirm-no = No

## Select
select-action-navigate = navigate
select-action-confirm = select
select-action-cancel = cancel
select-ctrl-c-hint = Press { $key } again to exit

## Duration
duration-days-hours = { $days }d { $hours }h
duration-hours-minutes = { $hours }h { $minutes }m
duration-minutes = { $minutes }m

## Upgrade
upgrade-up-to-date = ✅ Already on the latest version ({ $version }). No update needed.
upgrade-available = 💡 Update available: { $current } → { $latest }
upgrade-start = Upgrading d1v { $current } → { $latest }...
upgrade-downloading = Downloading d1v { $version }...
upgrade-download-complete = Download complete
upgrade-verifying = Verifying release checksum...
upgrade-installing = Installing update...
upgrade-success = 🎉 Successfully upgraded to d1v { $version }!
update-hint = 💡 New version available: { $current } → { $latest }  Run `d1v upgrade` to update.

## Uninstall
uninstall-success = 🗑️ Uninstall successful! Removed d1v from { $path }.

## Env
project-required = Project is required. Pass -p, set D1V_PROJECT_ID, or run in a d1v workspace.
env-key-not-found = Key "{ $key }" not found.
env-set-summary = { $created } created, { $updated } updated
env-import-summary = { $created } created, { $updated } updated, { $skipped } skipped (total: { $total })
env-export-saved = Exported to { $path }
env-import-stdin-required = Pipe .env content or use -i to specify a file.
env-sync-confirm-required = Pass --yes to confirm sync to Vercel.
env-label-key = Key
env-label-value = Value
env-label-description = Description
env-label-sensitive = Sensitive
env-label-message = Message
env-label-dev-project = Dev project
env-label-dev-env-count = Dev env count
env-label-dev-up-to-date = Dev up to date
env-label-prod-project = Prod project
env-label-prod-env-count = Prod env count
env-label-prod-up-to-date = Prod up to date
env-empty-list = No environment variables found.
env-yes = yes
env-no = no
