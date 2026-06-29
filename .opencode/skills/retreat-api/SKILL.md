---
name: retreat-api
description: Work with the my_retreat_nest Rust/Axum/SeaORM retreat booking API
license: MIT
compatibility: opencode
---

## Project Layout

```
src/
├── main.rs              # Entry point (jemalloc, tokio runtime)
├── lib.rs               # App bootstrap, router assembly
├── env.rs               # Environment config loader
├── state.rs             # AppState (DB connection pool)
├── routes/              # HTTP handlers (controllers)
│   ├── mod.rs
│   ├── health.rs
│   ├── auth.rs
│   ├── users.rs
│   ├── categories.rs
│   ├── retreats.rs
│   ├── retreat_reviews.rs
│   ├── retreat_galleries.rs
│   ├── gallery_categories.rs
│   └── wishlists.rs
├── entities/            # SeaORM generated entities
├── entities_helper/     # Re-exports with cleaner type aliases
├── serializers/         # Request/Response DTOs
│   ├── mod.rs
│   ├── auth.rs | users.rs | categories.rs | retreats.rs
│   ├── retreat_reviews.rs | retreat_galleries.rs
│   ├── gallery_categories.rs | wishlists.rs
│   └── pagination.rs
└── utils/               # Shared utilities
    ├── jwt.rs           # Token generation & verification
    ├── password.rs      # Argon2 password hashing
    ├── response.rs      # CustomResponse builder
    ├── serializer.rs    # Custom deserializer helpers
    ├── storage.rs       # File upload/read/delete
    ├── macros.rs        # set_fields!, set_active_model_fields!, map_fields!
    ├── extractors/
    │   └── auth.rs      # AuthUser / AuthAdmin extractors
    └── middlewares/
        └── panic.rs     # Global panic handler
migration/               # SeaORM migration crate
uploads/                 # Gallery image storage
```

## Response Envelope

All endpoints return JSON in the format `{ data, message, meta }` using `CustomResponse`:

```rust
CustomResponse::<Data, Meta>::builder(data)
    .message("...")
    .status_code(StatusCode::CREATED)
    .meta(pagination_meta)
    .build()
```

Import from `crate::utils::response::CustomResponse`.

## Auth

- **AuthUser:** Extract from `Authorization: Bearer <access_token>`. Loads the full `UserModel` from DB.
- **AuthAdmin:** Same implementation — no admin role check yet.
- **JWT:** HS256 dual-token (access + refresh) with separate keys from env vars.
- Import from `crate::utils::extractors::auth::{AuthUser, AuthAdmin}`.

## Custom Macros

Defined in `src/utils/macros.rs`.

- `set_fields!` — For PATCH: conditionally sets `ActiveModel` fields from `Option<T>` serializer fields.
- `set_active_model_fields!` — For POST: creates `ActiveModel` with all fields wrapped in `Set(...)`.
- `map_fields!` — Transfers fields from a `Model` to a `Serializer` struct field-by-field.

## Pagination

Accept via `Query<Pagination>` (from `crate::serializers::pagination::Pagination`):

```
?page=1&page_size=10
```

Return `PaginationMeta` alongside data arrays in the `CustomResponse` meta field.

## File Uploads

- Use Axum `Multipart` extractor.
- Files stored at `{UPLOAD_DIR}/retreat/gallery/<uuid>.<ext>`.
- Old files are cleaned up on update and delete.
- Import helpers from `crate::utils::storage`.

## Validation

Derive `validator::Validate` on serializer structs. On validation failure, return error response via `to_error_response`.

## Error Handling

```rust
to_error_response(e, status)                    // Any Display error
to_error_response_with_message(msg, status)      // Pre-formatted message
```

`CatchPanicLayer` catches panics and returns a 500 error.

## Route Registration

Routes are registered in `src/lib.rs` using Axum's `Router`. Each module in `src/routes/` exports a `routes()` function that returns a `Router` scoped to its path prefix.

## Migrations

- Managed via `sea-orm-cli` in `migration/` crate.
- Regenerate entities after migration: `sea-orm-cli generate entity -o src/entities` (run from project root).
