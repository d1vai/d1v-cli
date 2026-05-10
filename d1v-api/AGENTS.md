# d1v-api

## Response handling

- The JSON response body's `code` field is the API business code, separate from HTTP status; success is `code: 0` and `code: 200` is never success.
- Verify the runtime contract before changing the client when examples show `"code": 200`, and keep regression coverage that rejects it.

## Auth wrapping

- `.no_auth()` is opt-in per endpoint and must match `security` on that exact OpenAPI operation; never infer auth from sibling routes sharing a path prefix.
