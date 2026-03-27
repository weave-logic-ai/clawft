---
name: social-auth
description: Manage OAuth2 authorization for social media platforms (Twitter/X, LinkedIn, etc.). Authorize, check status, refresh, and revoke tokens.
version: 1.0.0
variables:
  - platform
  - action
allowed-tools:
  - Bash
  - Read
  - Write
user-invocable: true
argument-hint: "<platform> <action> (e.g., twitter authorize)"
---

# Social Media OAuth2 Management

You manage OAuth2 tokens for social media platforms through clawft's OAuth2
plugin. You translate user requests into `weft tool` CLI commands targeting the
`oauth2_authorize`, `oauth2_callback`, `oauth2_refresh`, and `rest_request`
tools.

## Supported Platforms

| Platform | Provider Config Name | Token File |
|----------|---------------------|------------|
| Twitter/X | `twitter` | `~/.clawft/tokens/twitter.json` |
| LinkedIn | `linkedin` | `~/.clawft/tokens/linkedin.json` |

New platforms can be added by configuring an OAuth2 provider in
`~/.clawft/config.json` under `oauth2.providers.<name>`.

## Available Actions

### authorize -- Start OAuth2 Flow

Begin the authorization code flow for a platform. Returns a URL the user must
open in their browser.

```bash
weft tool oauth2_authorize --provider {{platform}}
```

After the user authorizes in the browser, they will be redirected to the
callback URL. Capture the `code` and `state` parameters:

```bash
weft tool oauth2_callback --provider {{platform}} --code "<code>" --state "<state>"
```

This exchanges the auth code for tokens and stores them at
`~/.clawft/tokens/{{platform}}.json`.

### status -- Check Token Status

Check whether a valid token exists for a platform.

```bash
# Read the token file directly
cat ~/.clawft/tokens/{{platform}}.json 2>/dev/null
```

Report:
- Whether tokens exist.
- Whether the access token is expired (compare `expires_at` to current epoch).
- Whether a refresh token is available.
- The granted scopes.

Do NOT display the actual token values to the user. Only report metadata.

### refresh -- Refresh Access Token

Refresh an expired access token using the stored refresh token.

```bash
weft tool oauth2_refresh --provider {{platform}}
```

If no refresh token is available, inform the user they need to re-authorize.

### revoke -- Revoke Tokens

Revoke all tokens for a platform. This makes an API call to the provider's
revocation endpoint (if supported) and then deletes the local token file.

For Twitter/X:
```bash
weft tool rest_request --provider twitter --method POST \
  --url "https://api.x.com/2/oauth2/revoke" \
  --body '{"token": "<access_token>", "client_id": "<client_id>"}'
```

Then remove the local token file:
```bash
rm ~/.clawft/tokens/{{platform}}.json
```

## Provider Configuration

Before authorizing, the user must have a provider configured in
`~/.clawft/config.json`. Example for Twitter:

```json
{
  "oauth2": {
    "providers": {
      "twitter": {
        "preset": "custom",
        "client_id": "YOUR_CLIENT_ID",
        "client_secret_ref": { "env_var": "TWITTER_CLIENT_SECRET" },
        "auth_url": "https://twitter.com/i/oauth2/authorize",
        "token_url": "https://api.x.com/2/oauth2/token",
        "scopes": ["tweet.read", "tweet.write", "users.read", "bookmark.read", "bookmark.write", "offline.access"],
        "redirect_uri": "http://localhost:8085/callback"
      }
    }
  }
}
```

If the provider config is missing, guide the user through creating it. Never
hardcode or ask for the `client_secret` directly -- it must be referenced via
an environment variable through `client_secret_ref`.

## Error Handling

- **No provider config**: Tell the user to add the provider to their config.
- **No tokens stored**: Suggest running `authorize`.
- **Token expired, no refresh token**: Suggest re-running `authorize`.
- **Refresh failed (invalid_grant)**: The refresh token was revoked. Re-authorize.
- **Network errors**: Report the error and suggest retrying.

## Safety Rules

- NEVER display raw access tokens or refresh tokens to the user.
- NEVER hardcode client secrets. Always use `client_secret_ref` with env vars.
- NEVER store tokens outside `~/.clawft/tokens/`.
- Always confirm before revoking tokens.
