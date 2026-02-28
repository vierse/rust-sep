# API

Errors are sent as a JSON string containing the reason, e.g. `"Unauthorized"`.

Many internal errors are simply `500 Internal Server Error`

## Core API
| Endpoint | Request | Response |
|---|---|---|
| `POST /api/shorten` | Shortens provided `url`, optionally using `alias` and `password` | On success `201 {"alias": "..."}` and sets an owner token, otherwise `400` indicates a validation error, `409` if alias already exists. |
| `GET /r/{alias}` | Resolve alias and redirect to target URL, unlock page, or collection page | On success `307` redirect,  `400` indicates a validation error. `404` alias not found, `410` if link expired. |
| `GET /r/{alias}/{idx}` | Redirect to item `idx` in a collection alias | On success `307` redirect,  `400` indicates a validation error. `404` alias not found, not a collection or out of bounds index. `410` if link expired. |
| `POST /api/unlock/{alias}` | Unlocks a password protected link with `password` | On success `200` and sets an unlock token, otherwise `400` indicates a validation error. `401` on wrong password, `404` alias not found, `410` if link expired. |
| `GET /api/info/{alias}` | Get link metadata. Includes metrics only when caller owns the link (active session or owner token) | On success `200 {"owned": bool, "protected": bool, "data": ...}` otherwise `400` indicates a validation error. `404` alias not found, `410` if link expired. |

## Collection API
| Endpoint | Request | Response |
|---|---|---|
| `POST /api/collection/create` | Create a link collection from `urls` (limited to 10), optionally with `alias` and `password` | On success `201 {"alias": "..."}` otherwise `400` indicates a validation error. `409` alias already exists or collection size limit reached. |
| `POST /api/collection/create/{alias}` | Convert an existing owned single link into a collection | On success `201 "/collection/{alias}"` otherwise `400` indicates a validation error or when the link is already a collection. `401` if it doesn't belong to caller, `404` alias not found, `410` if link expired. |
| `GET /api/collection/{alias}/list` | Get items in a collection | On success `200 {"alias": "...", "items": [...], "owned": bool}`. If locked, returns an unlock path `423 {"unlock": "/unlock/{alias}"}`. `400` indicates a validation error or when the link is not a collection. `404` alias not found, `410` if link expired. |
| `POST /api/collection/{alias}/add` | Add a `url` to an existing owned collection with an optional `title` | On success `201` otherwise `400` indicates a validation error. `401` if it doesn't belong to caller, `404` alias not found. `409` when collection size limit reached. `410` if link expired. |

## User API
| Endpoint | Request | Response |
|---|---|---|
| `GET /api/auth/me` | Authenticate current session cookie | On success `200 {"username": "..."}` otherwise `401` if session does not exist or is invalid. |
| `POST /api/auth/login` | Authenticate user using `username` and `password` | On success `200 {"username": "..."}` and sets a session cookie. `400` indicates a validation error. `401` indicates auth error. |
| `POST /api/auth/register` | Create user with `username` and `password` | On success `200 {"username": "..."}` and sets a session cookie. `400` indicates a validation error or username already existing. |
| `GET /api/user/list` | List links owned by authenticated user | On success `200 [{"alias": "...", "kind": "...", "protected": bool}, ...]` otherwise `401` indicates auth error. |
| `DELETE /api/user/link/{alias}` | Delete a user-owned link by alias | On success `204` otherwise `400` indicates a validation error. `401` auth error, `404` alias not found. |
| `POST /api/user/logout` | Logout current user | On success `204` and clears session cookie. `401` if session does not exist or is invalid. |