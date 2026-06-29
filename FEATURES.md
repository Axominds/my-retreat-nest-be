# `my_retreat_nest` — Feature Summary

**RESTful API backend** built with **Rust + Axum + SeaORM + PostgreSQL** for a retreat/hotel booking and management platform.

---

## Tech Stack

| Category | Technology |
|---|---|
| Language | Rust (edition 2024) |
| Web Framework | Axum 0.8.7 |
| ORM | SeaORM 1.1 (PostgreSQL) |
| Auth | JWT (HS256) — access + refresh token pair |
| Password Hashing | Argon2id |
| File Upload | Multipart uploads to local filesystem |
| Validation | `validator` derive macros |

---

## Database Schema (8 Tables)

| Table | Purpose |
|---|---|
| `users` | User accounts (unique email, name, phone, Argon2-hashed password) |
| `categories` | Retreat categories (hotel, resort, farmhouse, etc.) |
| `retreats` | Core retreat entity (name, description, slug, location, budget, social links, publish status) |
| `retreat_users` | Staff/owner association (many-to-many: users ↔ retreats, with role & ownership flag) |
| `retreat_reviews` | User reviews (rating 0–5, text, unique per user+retreat) |
| `retreat_galleries` | Gallery images per retreat (file path, caption, order, category) |
| `gallery_categories` | Categories for gallery images |
| `wishlists` | User-saved retreats (unique per user+retreat) |

**Cross-cutting:** All tables include `created_at`/`updated_at` timestamps (auto-updated via PostgreSQL trigger); most have `created_by`/`updated_by` audit columns.

---

## Feature Modules & API Endpoints (35 routes)

### Health
| Method | Path | Auth | Description |
|---|---|---|---|
| GET | `/` | — | Server health check |

### Authentication (`/auth/`)
| Method | Path | Auth | Description |
|---|---|---|---|
| POST | `/auth/login/` | — | Login (email + password → JWT pair) |
| POST | `/auth/refresh/` | — | Exchange refresh token for new JWT pair |

### Users (`/users/`)
| Method | Path | Auth | Description |
|---|---|---|---|
| POST | `/users/` | — | Create user (signup) |
| GET | `/users/` | — | List users (paginated) |
| GET | `/users/{id}/` | — | Get user by ID |
| PATCH | `/users/{id}/` | AuthUser | Update user |
| DELETE | `/users/{id}/` | — | Delete user |

### Categories (`/categories/`)
| Method | Path | Auth | Description |
|---|---|---|---|
| POST | `/categories/` | — | Create category |
| GET | `/categories/` | — | List categories |
| GET | `/categories/{id}/` | — | Get category |
| PATCH | `/categories/{id}/` | — | Update category |
| DELETE | `/categories/{id}/` | — | Delete category |

### Retreats (`/retreats/`)
| Method | Path | Auth | Description |
|---|---|---|---|
| POST | `/retreats/` | — | Create retreat |
| GET | `/retreats/` | — | List retreats (paginated) |
| GET | `/retreats/{id}/` | — | Get retreat |
| PATCH | `/retreats/{id}/` | — | Update retreat |
| DELETE | `/retreats/{id}/` | — | Delete retreat |
| POST | `/retreats/{id}/users/` | — | Add staff user (creates user if not exists) |
| PATCH | `/retreats/{id}/users/{ruid}/` | — | Update staff role |
| DELETE | `/retreats/{id}/users/{ruid}/` | — | Remove staff from retreat |

### Reviews (`/retreats/{id}/reviews/`)
| Method | Path | Auth | Description |
|---|---|---|---|
| POST | `/retreats/{id}/reviews/` | AuthUser | Create review (one per user per retreat) |
| GET | `/retreats/{id}/reviews/` | — | List reviews (paginated) |
| PATCH | `/retreats/{id}/reviews/{rid}/` | AuthUser | Update own review |
| DELETE | `/retreats/{id}/reviews/{rid}/` | AuthUser | Delete own review |

### Gallery Categories (`/gallery-categories/`)
| Method | Path | Auth | Description |
|---|---|---|---|
| POST | `/gallery-categories/` | AuthAdmin | Create gallery category |
| GET | `/gallery-categories/` | — | List gallery categories |
| PATCH | `/gallery-categories/{id}/` | AuthAdmin | Update gallery category |
| DELETE | `/gallery-categories/{id}/` | AuthAdmin | Delete gallery category |

### Retreat Galleries (`/retreats/{id}/galleries/`)
| Method | Path | Auth | Description |
|---|---|---|---|
| POST | `/retreats/{id}/galleries/` | AuthUser | Upload gallery image (multipart) |
| GET | `/retreats/{id}/galleries/` | — | List gallery items (paginated) |
| PATCH | `/retreats/{id}/galleries/{gid}/` | AuthUser | Update gallery item |
| DELETE | `/retreats/{id}/galleries/{gid}/` | AuthUser | Delete gallery item (removes file from disk) |
| GET | `/retreats/{id}/galleries/{gid}/image/` | — | Serve gallery image file |

### Wishlists (`/users/wishlists/retreats/`)
| Method | Path | Auth | Description |
|---|---|---|---|
| POST | `/users/wishlists/retreats/{id}/` | AuthUser | Add retreat to wishlist (idempotent) |
| DELETE | `/users/wishlists/retreats/{id}/` | AuthUser | Remove from wishlist |
| GET | `/users/wishlists/retreats/` | AuthUser | List wishlist (paginated) |

---

## Architecture & Patterns

- **Layered design:** Routes (controllers) → SeaORM entities (data layer) → Serializers (DTOs)
- **Unified JSON responses:** `{ data, message, meta }` envelope via `CustomResponse` builder
- **JWT dual-token auth:** Access token in `Authorization` header + refresh endpoint; HS256-signed with separate keys
- **Custom macros:** `set_fields!`, `set_active_model_fields!`, `map_fields!` reduce field-mapping boilerplate
- **Axum extractors:** `AuthUser` and `AuthAdmin` for ergonomic auth injection into handlers
- **Standardized pagination:** `?page=1&page_size=10` → `PaginationMeta` in every list response
- **File storage:** UUID-named files in `uploads/retreat/gallery/`; old file cleanup on update/delete
- **Audit trail:** `created_by`/`updated_by` FK columns on most tables
- **Error handling:** Unified `to_error_response()` + `CatchPanicLayer` for panic recovery

---

## Notable Observations

- `AuthAdmin` extractor does **not** verify admin role — it's semantically identical to `AuthUser`
- No explicit service layer; business logic lives directly in route handlers
- Open CORS (`allow_origin(Any)`) — dev-friendly, should be tightened for production
- Temporary password `"tempPassword"` is hardcoded for new staff users (no reset flow)
