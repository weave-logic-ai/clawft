# ADR: Social Media Integration via Skills (Not Compiled Crates)

**Status**: Accepted
**Date**: 2026-02-25
**Decision**: Use SKILL.md-based skills for social media platform integrations
instead of compiled Rust crates.

## Context

We need Twitter/X and LinkedIn integration for bookmark processing, content
categorization, and content creation/posting. A broken `crates/clawft-twitter/`
stub existed that used non-existent `Plugin`/`PluginContext` traits (the actual
traits are `Tool`/`ToolContext` in `clawft-plugin`). It was not registered in
the workspace `Cargo.toml` and could not compile.

The question: do we need compiled Rust crates per social media platform, or
are skills sufficient?

## Decision

**Skills are sufficient. No new compiled crates per platform.**

### Rationale

1. **HTTP-only APIs**: Twitter and LinkedIn are REST APIs over HTTP. The
   existing `rest_request` tool from `clawft-plugin-oauth2` handles all
   authenticated HTTP calls with automatic Bearer token injection.

2. **OAuth2 already solved**: `clawft-plugin-oauth2` provides
   `oauth2_authorize`, `oauth2_callback`, `oauth2_refresh`, and `rest_request`
   tools. Any platform that uses OAuth2 works out of the box with a `Custom`
   provider preset.

3. **LLM-native categorization**: Content categorization (classifying bookmarks
   into tech/news/personal/etc.) is LLM reasoning work. A skill's instructions
   ARE the prompt -- no compiled code needed.

4. **CI cost avoidance**: Each compiled crate adds build time to both `native`
   and `browser` targets. Social media integrations are user-space concerns
   that don't belong in the compiled core.

5. **Hot-reload and editability**: Skills auto-discover from disk and hot-reload
   when changed. Users can customize social media workflows without rebuilding.

6. **Generalization**: The pattern extends to any OAuth2 platform (Bluesky,
   Mastodon, GitHub, etc.) with zero compiled code.

## What Was Deleted

### `crates/clawft-twitter/`

- **Why**: Used `Plugin` and `PluginContext` traits that don't exist in
  `clawft-plugin`. The actual trait is `Tool` with `ToolContext`.
- **Why not fix it**: Even with correct traits, a compiled crate adds CI cost
  and coupling for pure HTTP+LLM work that skills handle better.
- **Not in workspace**: Was never added to `Cargo.toml` `[workspace.members]`.

### `skills/twitter-bookmarks/`

- **Why**: One-line shell stub (`run.sh`) referencing non-existent plugin
  commands. The bookmarks action is folded into the unified `skills/twitter/`
  skill.

## What Was Created

| Skill | Purpose |
|-------|---------|
| `skills/social-auth/SKILL.md` | Shared OAuth2 management across platforms |
| `skills/twitter/SKILL.md` | Twitter/X: bookmarks, categorize, draft, post, search |
| `skills/linkedin/SKILL.md` | LinkedIn: saved posts, draft, post, profile |

## OAuth2 Token Flow

```
User config (~/.clawft/config.json)
  |
  +-- oauth2.providers.twitter:
  |     client_id, client_secret_ref, auth_url, token_url, scopes
  |
  v
/social-auth twitter authorize
  |
  +-- Calls oauth2_authorize -> returns browser URL
  +-- User authorizes in browser -> callback
  +-- Calls oauth2_callback -> exchanges code for tokens
  |
  v
Token stored at ~/.clawft/tokens/twitter.json
  |
  v
/twitter bookmarks
  |
  +-- Calls rest_request(GET, "https://api.x.com/2/users/me/bookmarks")
  +-- rest_request auto-injects Bearer token from token store
  +-- If 401 -> calls oauth2_refresh -> retries
  |
  v
Results stored at ~/.clawft/workspace/social/twitter/bookmarks/YYYY-MM-DD.json
```

## Extending to New Platforms

To add a new social media platform (e.g., Bluesky, Mastodon):

1. Add OAuth2 provider config to `~/.clawft/config.json` (or guide user).
2. Create `skills/<platform>/SKILL.md` following the twitter/linkedin pattern.
3. Optionally extend `skills/social-auth/SKILL.md` supported platforms table.
4. No Rust code changes. No rebuild. No CI impact.

## Exit Criteria

- [x] `crates/clawft-twitter/` deleted
- [x] `skills/twitter-bookmarks/` deleted (merged into `skills/twitter/`)
- [x] `skills/social-auth/SKILL.md` created and parseable
- [x] `skills/twitter/SKILL.md` created and parseable
- [x] `skills/linkedin/SKILL.md` created and parseable
- [ ] `cargo test --workspace` still passes (no Rust changes)
- [ ] `cargo check --workspace` still passes
- [ ] Skill discovery test finds new skills
