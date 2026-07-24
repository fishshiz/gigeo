# Gigeo

Gigeo is a map-based concert discovery app. Search a city, browse upcoming
shows nearby on an interactive map, and turn what you find into a Spotify or
Apple Music playlist seeded from the artists playing.

## How it works

- **Search & map** — pick a location and Gigeo pulls nearby events from the
  Ticketmaster Discovery API, geocoded with Mapbox, and plots them on a map
  with date filtering.
- **Playlists** — connect Spotify or Apple Music and generate a playlist
  from the artists in your search results, seeded with each artist's top
  tracks.

## Architecture

This is a pnpm/Turborepo monorepo with a Rust backend and a React frontend:

```
apps/
  api/        Rust (Axum) backend — Ticketmaster, Mapbox, Spotify, and
              Apple Music integrations, Postgres via sqlx
  web/        React + TypeScript frontend (Vite)
packages/
  ui/         Shared component library (shadcn/ui-based), consumed by web
```

The frontend calls the backend through relative `/api/*` paths; in
production this expects a reverse proxy in front of both apps that keeps
them same-origin (needed for session cookies). In local dev, Vite's dev
server proxies `/api` to the Rust server for you.

## Setup

Prerequisites: Node 20+, pnpm, Rust (see `rust-toolchain.toml`), and a
Postgres database.

```bash
pnpm install
cp apps/api/.env.example apps/api/.env
cp apps/web/.env.example apps/web/.env
```

Fill in the values in both `.env` files — see [Environment variables](#environment-variables)
below. Then run migrations and start both apps:

```bash
pnpm dev
```

This runs the Vite dev server (`http://localhost:5173`) and the Rust API
(`http://localhost:3000`, proxied under `/api`) together via Turborepo.
Database migrations run automatically on API startup.

## Environment variables

**`apps/api/.env`**

| Variable | Description |
| --- | --- |
| `MAPBOX_PRIVATE_KEY` | Mapbox server-side token, used for geocoding |
| `TICKETMASTER_API_KEY` | Ticketmaster Discovery API key |
| `SPOTIFY_CLIENT_ID` / `SPOTIFY_CLIENT_SECRET` / `SPOTIFY_REDIRECT_URI` | Spotify OAuth app credentials |
| `APPLE_MUSIC_TEAM_ID` / `APPLE_MUSIC_KEY_ID` / `APPLE_MUSIC_PRIVATE_KEY` / `APPLE_MUSIC_STOREFRONT` | Apple Music (MusicKit) credentials — optional, Apple Music routes error out if unset |
| `DATABASE_URL` | Postgres connection string |
| `COOKIE_DOMAIN` / `COOKIE_KEY` / `COOKIE_SECURE` | Session cookie signing/config |
| `CORS_ALLOWED_ORIGINS` | Comma-separated origins allowed to call the API cross-origin; defaults to `http://localhost:5173` |

**`apps/web/.env`**

| Variable | Description |
| --- | --- |
| `VITE_MAPBOX_ACCESS_TOKEN` | Mapbox client-side (public) token |
| `VITE_MAPBOX_DARK_STYLE` / `VITE_MAPBOX_LIGHT_STYLE` | Mapbox style URLs for dark/light mode |

## Adding UI components

Shared components live in `packages/ui`. To add a new shadcn/ui component:

```bash
pnpm dlx shadcn@latest add button -c apps/web
```

This places the component in `packages/ui/src/components`; import it from
`@workspace/ui/components/...` in `apps/web`.

## Scripts

Run from the repo root (via Turborepo):

- `pnpm dev` — run web + api in dev mode
- `pnpm build` — build all apps
- `pnpm lint` — lint all apps
- `pnpm typecheck` — typecheck the frontend
