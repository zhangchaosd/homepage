# bookmarkd read-only integration token (proposed contract)

The homepage shows pinned bookmarks by calling bookmarkd from the server side.
bookmarkd currently only accepts a passkey browser session, so it needs a small
addition: read-only bearer tokens for trusted integrations. This document is the
contract the homepage already implements (`src/widgets/bookmarkd.rs`).

## Request

```http
GET /api/v1/bookmarks?pinned=true&limit=12
Authorization: Bearer <token>
```

Response: the existing list shape — `{ "items": [Bookmark…], "total": n, "revision": n }`.
The homepage reads `id`, `url_raw`, `title`, `tags`, `pinned_position` and `icon`.

## Rules for bookmarkd

- Tokens are **read-only**: accepted on `GET /api/v1/bookmarks`, `GET /api/v1/folders`,
  `GET /api/v1/tags` and `GET /api/v1/library/revision` only. Any other route, and any
  non-GET method, answers `401` even with a valid token. No CSRF token is involved because
  no state changes.
- Store only a purpose-separated digest (e.g. `digest("integration-token", token)`, as for
  sessions); compare in constant time; never log the token.
- Tokens are created and revoked from the terminal, like setup grants:
  `bookmarkd token create <name>` prints the token once; `bookmarkd token list`,
  `bookmarkd token revoke <name>`.
- Tokens are separate from passkey sessions: they never set cookies and never satisfy
  "recent authentication".
- Keep the existing Origin rules for browser requests; a bearer request has no Origin.
- Rate limiting: share the existing per-peer limiter so a leaked token cannot be used to
  scrape at high speed.

## Homepage side

Put the token in the homepage's `secrets.toml`:

```toml
[tokens]
bookmarkd = "<token>"
```
