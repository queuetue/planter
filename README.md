# Planter

**Planter** is a stateless, inspectable execution engine for **interpreted declarative plans** based on the [Phase Manifest Protocol](./PROTOCOL.md).

It provides a universal backend for applying, inspecting, and evolving structured workflows — from simulations to deployments to orchestrated processes. Planter receives structured `Phase` objects over HTTP, diffs them against existing state, and executes them deterministically.

> Planter speaks the [Phase Manifest Protocol](./PROTOCOL.md). Any compliant tool can send it plans.

---

## Features

* **Plan Receiver** – Accepts structured JSON `Phase` manifests over HTTP (`POST /plan`)
* **Phase-Based Runtime** – Executes phases in order, respecting declared dependencies
* **Diffing** – Highlights plan differences from current state (planned)
* **Dry-Run Execution** – Simulate execution with no side effects
* **Logging & Events** – Emits structured logs and events per phase and hook
* **Introspection Endpoints** – APIs for status, diff, logs, and replay
* **Graceful Shutdown & Reload** – `/STOP` endpoint and signals save state to file and shut down; `/RELOAD` reloads state from file
* **Stateless by Default** – Optional Redis support for history or coordination
* **Open by Design** – No built-in authentication; access is by knowing the endpoint
* **Container-First** – Runs as a Kubernetes pod, Docker container, or bare-metal binary

---

## Architecture

Planter is written in Rust and powered by:

* `axum` for HTTP routing
* `tokio` for asynchronous execution
* `serde` for robust JSON serialization
* `redis` (optional) for state tracking and pub/sub

Execution units are `Phase` objects, each with:

* Human-readable `description`
* Label-based `selector` for targeting
* `waitFor` dependencies
* `retry` and error-handling policies
* Optional success/failure hooks

---

## Phase Manifest Format

Phase manifests are submitted as JSON arrays of `Phase` objects:

```json
[
  {
    "kind": "Phase",
    "id": "initialization",
    "spec": {
      "description": "Generate IDs and bootstrap state",
      "selector": { "matchLabels": { "phase": "initialization" } },
      "waitFor": { "phases": ["preflight"] },
      "retry": { "maxAttempts": 5 },
      "onFailure": {
        "action": "continue",
        "spec": {
          "message": ["Initialization failed, continuing with defaults"],
          "labels": { "mode": "fallback" }
        }
      }
    }
  }
]
```

This schema is loosely inspired by Kubernetes resource patterns, but is designed for direct runtime execution and traceable change.

---

## Getting Started

### Run Locally

```bash
cargo run
```

Planter will bind to `0.0.0.0:3030`.

### Submit a Plan

```bash
curl -X POST http://localhost:3030/plan \
     -H "Content-Type: application/json" \
     -d @rendered_plan.json
```

### Run in Docker

```bash
docker build -t planter .
docker run -p 3030:3030 planter
```

# Docker Usage

## Build Locally

```sh
# Build the Docker image locally
$ docker build -t planter:latest .
```

## Run Locally

```sh
# Run Planter with Redis using Docker Compose
$ docker-compose up
```

## Use from Docker Hub

```sh
# Pull the latest image from Docker Hub
$ docker pull queuetue/planter:latest

# Run with Redis using Docker Compose
$ docker-compose up
```

## Environment Variables
 `REDIS_URL`: Set to your Redis instance (default: `redis://redis:6379`)
 `PLANTER_PREFIX`: If set, all API endpoints will be served under this prefix. Example: if `PLANTER_PREFIX=/api/v1`, then `/plan` becomes `/api/v1/plan`.

## Ports
- Planter API: `3030`
- Redis: `6379`
#### Prefixing API Endpoints

To serve all endpoints under a custom prefix, set the `PLANTER_PREFIX` environment variable:

```bash
export PLANTER_PREFIX="/api/v1"
cargo run
```

All endpoints will now be available under `/api/v1/*`, e.g. `POST /api/v1/plan`, `GET /api/v1/state`, etc.

## Repository
- Source: [github.com/queuetue/planter](https://github.com/queuetue/planter)
- Docker Hub: [docker.com/r/queuetue/planter](https://hub.docker.com/r/queuetue/planter)

---

## Intended Workflow

1. A compliant tool (e.g., a CLI or CI system) renders a plan
2. The plan is posted to Planter via `/plan`
3. Planter:

   * Parses and validates the plan
   * Computes a diff against stored or inferred state
   * Executes the plan phase-by-phase
   * Emits logs and status updates via API or Redis
4. Tools can query `/state`, `/diff`, `/logs`, etc. (planned)

---

## Ecosystem

| Tool        | Role                                                      |
| ----------- | --------------------------------------------------------- |
| **Janet**   | CLI: plan rendering, macro expansion, validation          |
| **Planter** | Server: plan ingestion, execution, diffing, introspection |
| **You**     | Any tool that emits valid Phase Manifests                 |

Planter is protocol-compliant — not tool-specific. Tools may be written in Python, Go, Bash, etc., as long as they emit valid JSON following the [Phase Manifest Protocol](./PROTOCOL.md).

---

## Roadmap

* [x] `POST /plan` — Submit and execute phase manifests (with diffing and execution)
* [x] `GET /state` — Return active or last-applied plan (basic implementation)
* [x] `GET /diff` — Compare current vs incoming plan (basic endpoint, full logic pending)
* [x] `GET /logs` — Access run-level logs (basic endpoint, full implementation pending)
* [x] `GET /phases/:id` — Inspect or rerun specific phase (basic endpoint, full implementation pending)
* [x] `POST /apply` — Commit staged plan to execution (basic endpoint, full implementation pending)
* [x] Health endpoints (`/health`, `/ready`, `/metrics`) — Prometheus-compatible monitoring
* [x] API prefix support — Configurable endpoint prefixes via `PLANTER_PREFIX`
* [x] Optional Redis storage — For state tracking and coordination
* [ ] Full diff computation — Complete plan comparison logic
* [x] Complete logging system — Persistent log storage and retrieval
* [ ] Phase inspection and replay — Individual phase execution and history
* [ ] Executor abstraction — Support for Python, shell, container, API executors
* [ ] Multi-tenancy/namespacing — Support for multiple isolated plan contexts

---

## Configuration

| Option        | Default         | Description                                      |
| ------------- | --------------- | ------------------------------------------------ |
| Port          | `3030`          | TCP port to bind to                              |
| Redis         | *disabled*      | Optional for state and pub/sub                   |
| Auth          | *none*          | Add via proxy or overlay if needed               |
| PLANTER_ROOT  | `/etc/planter`  | Directory for persistent state file              |

### Shutdown, Reload, and State Sync

**State File:**
- On shutdown (`/STOP` endpoint or SIGTERM/SIGINT), Planter saves its current plan state to `$PLANTER_ROOT/state/state.json` and exits.
- On startup, if the state file exists, it is loaded as the initial state.
- `/RELOAD` endpoint or SIGHUP signal reloads state from file without restarting.

**Endpoints:**
- `POST /STOP` — Save state and shut down
- `POST /RELOAD` — Reload state from file

**Signals:**
- `SIGTERM` or `SIGINT` — Save state and shut down
- `SIGHUP` — Reload state from file


**Sync-from Modes:**
- **One-time sync:** If the environment variable `PLANTER_SYNC_FROM` is set to the base URL of another Planter server, on startup Planter will fetch `/state` from that server and use its phases as the initial state (unless a local state file is present). This enables bootstrapping from a remote Planter instance for migration, failover, or distributed workflows.
- **Periodic sync:** If you also set `PLANTER_SYNC_INTERVAL` (in seconds), Planter will poll the remote server at that interval and override its own state with the remote `/state` response. This is one-way only (no push), and is useful for keeping a Planter instance in sync with a canonical source.

**Example:**
```bash
export PLANTER_SYNC_FROM="http://other-planter:3030"
export PLANTER_SYNC_INTERVAL=30
cargo run
# Planter will fetch http://other-planter:3030/state every 30 seconds and update its own state.
```

Planter favors simple, deterministic behavior and is designed to be embedded into larger systems or CI/CD pipelines. State persistence and remote sync make it suitable for distributed and resilient deployments.

---

## Requirements

- **NATS**: Planter requires a running [NATS server](https://nats.io/) for session-based execution and communication. Set the `NATS_URL` environment variable to your NATS server address.
- **Redis** (optional): For persistent state, history, and coordination, Planter can use [Redis](https://redis.io/). Set the `REDIS_URL` environment variable to enable Redis integration.

---

## License

MIT License
© 2025 [Queuetue, LLC](https://queuetue.com)
