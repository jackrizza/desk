# API Notes

The backend is served by the `openapi` crate and exposes routes under `/api`.

## Utility

### `GET /api/hello`

Simple health/demo endpoint.

## Market Data

### `GET /api/stock_data`

Query params:

- `symbol`
- `range`
- `interval`
- `prepost`

Returns raw stock bars for charting.

### `GET /api/indicators`

Query params:

- `symbol`
- `range`
- `interval`
- `prepost`
- `indicators` as a comma-separated list

Returns indicator results plus unsupported indicators.

## Projects

### `GET /api/projects`

Returns all projects.

### `POST /api/projects`

Creates a project.

Current project fields include:

- `id`
- `name`
- `description`
- `strategy`
- `created_at`
- `updated_at`
- `symbols`
- `interval`
- `range`
- `prepost`

### `GET /api/projects/:project_id`

Returns a single project.

### `PUT /api/projects/:project_id`

Updates a project, including its saved strategy.

### `DELETE /api/projects/:project_id`

Deletes a project.

## Portfolios

### `GET /api/portfolios`
### `POST /api/portfolios`
### `GET /api/portfolios/:portfolio_id`
### `PUT /api/portfolios/:portfolio_id`
### `DELETE /api/portfolios/:portfolio_id`

Portfolio payloads include:

- `id`
- `name`
- `description`
- `created_at`
- `updated_at`
- `positions`

## Positions

### `GET /api/portfolios/:portfolio_id/positions`
### `POST /api/portfolios/:portfolio_id/positions`
### `GET /api/portfolios/:portfolio_id/positions/:symbol/:position_opened_at`
### `PUT /api/portfolios/:portfolio_id/positions/:symbol/:position_opened_at`
### `DELETE /api/portfolios/:portfolio_id/positions/:symbol/:position_opened_at`

Positions are identified by:

- `portfolio_id`
- `symbol`
- `position_opened_at`

## Source of Truth

For the exact schema and route behavior, use:

- [main.rs](C:\Users\jack\OneDrive\Documents\Code\Rust\desk\crates\openapi\src\main.rs)
- [lib.rs](C:\Users\jack\OneDrive\Documents\Code\Rust\desk\crates\database\src\lib.rs)
- [api.ts](C:\Users\jack\OneDrive\Documents\Code\Rust\desk\frontend\app\lib\api.ts)

