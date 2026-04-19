# Overview

Desk is a trading workspace organized around three main experiences:

- `Portfolio`: manual portfolio operations and monitoring
- `Market`: deep chart view for a symbol
- `Projects`: strategy design, saving, and prototype backtesting

## Product Direction

The application is moving in three layers:

1. Manual operations
2. Strategy definition and testing
3. Eventual algorithmic execution

Today the system is strongest in layers 1 and 2.

## Key Concepts

### Portfolio

A portfolio holds positions and is used for manual tracking and operational control.

### Position

A position is a symbol-level holding with quantity, average price, open timestamp, and optional close information.

### Project

A project is a strategy workspace built around:

- a symbol universe
- timeframe settings
- a saved strategy draft

Projects are the bridge between research and future automation.

### Strategy

Strategy text is currently stored as a concise saved outline on the project itself. The current backtest flow uses that saved strategy draft as input for a prototype simulation.

## Important Constraints

- The current backtester is a prototype and does not yet execute a formally parsed strategy model.
- OpenAI integration currently runs from the frontend using a locally stored API key.
- SQLite is used for local persistence and should be treated as a development store for now.

