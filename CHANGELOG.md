# Changelog

## 0.2.1

- **`reco api`** — generate named APIs (unique key, port, client kit) so other apps use models on this machine.
- Hub: one server, many keys, each key scoped to its model. `--lan` binds `0.0.0.0`.
- Clients: curl, Python, JS, Continue, Cursor, Open WebUI, LangChain, `.env`, OpenAPI.
- `reco serve` without a model starts the hub.

## 0.2.0

Product pass over the 0.1 CLI.

- `reco` with no args is a home dashboard (hardware, engine, disk, recent chats).
- `reco doctor`, `reco models` / `models rm`, `reco config get|unset`, shell completions.
- Catalog TUI: score panel, downloaded badge, `d` filter, `?` help.
- Prueba: scroll, new conversation (`Ctrl+N`), help overlay.
- `reco serve`: CORS, HTML docs, `stream: true`, persistent `sk-reco-…` key, `usage`.
- Downloads resume via HTTP Range; progress bar with speed.
- Failed model lookups suggest nearby repos.
- Desktop Prueba: sidebar of conversations.

## 0.1.0

Hardware detection, Hugging Face GGUF catalog, scoring, TUI, download, Prueba, serve, llama-cli / BYOK, packaging recipes.
