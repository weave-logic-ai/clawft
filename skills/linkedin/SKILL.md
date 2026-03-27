---
name: linkedin
description: Interact with LinkedIn -- fetch saved posts, draft articles and updates, publish content, and view profile info.
version: 1.0.0
variables:
  - action
  - topic
allowed-tools:
  - Bash
  - Read
  - Write
  - Glob
user-invocable: true
argument-hint: "<action> [options] (e.g., draft \"topic\", post, profile)"
---

# LinkedIn Operations

You interact with the LinkedIn API v2 through clawft's OAuth2 plugin. All
authenticated requests use `weft tool rest_request --provider linkedin` which
auto-injects the Bearer token from `~/.clawft/tokens/linkedin.json`.

## Prerequisites

The user must have authorized LinkedIn via `/social-auth linkedin authorize`
before using any action. If a request returns 401, call
`weft tool oauth2_refresh --provider linkedin` and retry once.

## Available Actions

### saved -- Fetch Saved Posts

Fetch the authenticated user's saved/bookmarked posts.

```bash
weft tool rest_request --provider linkedin --method GET \
  --url "https://api.linkedin.com/v2/savedPosts?q=member&count=50"
```

Store results to:
```
~/.clawft/workspace/social/linkedin/saved/YYYY-MM-DD.json
```

Note: LinkedIn's saved posts API may have limited availability. If the endpoint
returns 403, inform the user that this feature requires specific API permissions
and suggest checking their LinkedIn app's authorized scopes.

### draft -- Compose Content

Draft a LinkedIn post or article on a given topic.

**Short update** (default): Up to 3000 characters. Professional tone.
- Include a hook in the first line (appears in feed preview).
- Use line breaks for readability.
- Suggest 3-5 relevant hashtags.
- Optionally include a call-to-action.

**Article** (use `--article` flag or when topic requires long-form):
- Title + body format.
- Structured with headers, bullet points, and clear sections.
- 500-2000 words recommended.
- Include a summary/TL;DR at the top.

Save drafts to:
```
~/.clawft/workspace/social/linkedin/drafts/<slug>.json
```

Draft format:
```json
{
  "topic": "original topic",
  "type": "update|article",
  "title": "Article title (articles only)",
  "body": "Full post content",
  "hashtags": ["#hashtag1", "#hashtag2"],
  "visibility": "PUBLIC",
  "created_at": "ISO timestamp",
  "status": "draft"
}
```

Present the draft to the user for review. LinkedIn content should be more
professional and detailed than Twitter content.

### post -- Publish Content

Post a draft or compose and send immediately.

1. If a draft slug is provided, read from drafts directory.
2. Show the user exactly what will be posted.
3. **Always ask for explicit confirmation** before posting.

First, get the user's LinkedIn URN:
```bash
weft tool rest_request --provider linkedin --method GET \
  --url "https://api.linkedin.com/v2/me"
```

Then post an update:
```bash
weft tool rest_request --provider linkedin --method POST \
  --url "https://api.linkedin.com/v2/ugcPosts" \
  --body '{
    "author": "urn:li:person:<person_id>",
    "lifecycleState": "PUBLISHED",
    "specificContent": {
      "com.linkedin.ugc.ShareContent": {
        "shareCommentary": { "text": "Post content here" },
        "shareMediaCategory": "NONE"
      }
    },
    "visibility": { "com.linkedin.ugc.MemberNetworkVisibility": "PUBLIC" }
  }'
```

After posting, update the draft status to `"posted"` with the post URN.

### profile -- View Profile Info

Fetch the authenticated user's basic profile information.

```bash
weft tool rest_request --provider linkedin --method GET \
  --url "https://api.linkedin.com/v2/me?projection=(id,localizedFirstName,localizedLastName,localizedHeadline,vanityName)"
```

Display the profile info in a readable format.

## Provider Configuration

Example LinkedIn provider config for `~/.clawft/config.json`:

```json
{
  "oauth2": {
    "providers": {
      "linkedin": {
        "preset": "custom",
        "client_id": "YOUR_CLIENT_ID",
        "client_secret_ref": { "env_var": "LINKEDIN_CLIENT_SECRET" },
        "auth_url": "https://www.linkedin.com/oauth/v2/authorization",
        "token_url": "https://www.linkedin.com/oauth/v2/accessToken",
        "scopes": ["openid", "profile", "w_member_social", "r_liteprofile"],
        "redirect_uri": "http://localhost:8085/callback"
      }
    }
  }
}
```

## Token Expiry Handling

If any API call returns HTTP 401:
1. Call `weft tool oauth2_refresh --provider linkedin`.
2. Retry the original request once.
3. If it fails again, inform the user their token may be revoked and suggest
   re-authorizing with `/social-auth linkedin authorize`.

Note: LinkedIn access tokens typically expire after 60 days. Refresh tokens
last 365 days. Plan accordingly.

## Error Handling

- **401 Unauthorized**: Refresh token and retry (see above).
- **403 Forbidden**: Missing API scope or app permission. Report which scope
  is needed.
- **422 Unprocessable Entity**: Content validation failed (e.g., too long,
  invalid media). Report the specific error.
- **429 Rate Limited**: LinkedIn rate limits are per-app. Wait and retry.
  Report the retry-after duration.
- **5xx Server Error**: LinkedIn is experiencing issues. Suggest retrying later.

## Safety Rules

- NEVER post without explicit user confirmation.
- NEVER store raw API tokens in workspace files -- only store content data.
- LinkedIn content should maintain professional tone. Warn the user if drafted
  content seems inappropriate for a professional network.
- Do not include sensitive personal information unless the user explicitly
  provides it.
