# Phase Manifest Protocol (PMP)

**Status**: Draft  
**Version**: 1.0.0  
**Content-Type**: `application/vnd.phase-manifest+json`  
**Protocol ID**: `phase-manifest-pmp`

---

## Overview

The **Phase Manifest Protocol (PMP)** defines a structured, machine-readable format and transport method for declarative **phase-based workflows**. It is designed for stateless execution servers that receive, diff, apply, and introspect plans composed of sequential or interdependent _phases_.

_Phases_ in this protocol are conceptually similar to init phases in a Unix-style system boot sequence: each phase prepares some aspect of the system or environment, may depend on prior phases completing successfully, and may trigger follow-on actions. Just as a traditional init system coordinates service startup in a predictable, introspectable order, a Phase Manifest coordinates declarative workflows in a structured, inspectable sequence.

PMP is transport-agnostic, supporting both HTTP and message-based interfaces (like NATS). This document defines:
- The JSON schema for manifests
- Default HTTP semantics
- Standardized NATS subject patterns and message formats

---

## Goals

- Provide a universal JSON-based schema for phase execution plans
- Enable drift-aware execution via diffing and inspection
- Support dry-run, introspection, and log-based observability
- Remain lightweight and extensible, compatible with domain-specific toolchains

---

## Terminology

- **Phase**: A named unit of work, with metadata, selectors, dependencies, and outcome handlers.
- **Manifest**: A list of `Phase` objects, sent as a single JSON array.
- **PMP Server**: A stateless receiver/actuator for manifests (e.g. a plan executor or orchestrator).
- **Compliant Tool**: Any client that can render and transmit a valid Phase Manifest over HTTP or NATS.

---

## Phase Manifest Format


### Field Naming Conventions

PMP manifests use specific case conventions for JSON fields:

- **Top-level fields** (e.g., in each Phase object): PascalCase (e.g., `Kind`, `Id`, `Spec`)
- **Nested fields** (inside `Spec`): snake_case (e.g., `match_labels`, `instance_mode`, `wait_for`, `on_failure`, `on_success`)

This applies to all JSON submitted to PMP-compliant servers. Example:

```json
[
  {
    "Kind": "Phase",
    "Id": "preflight",
    "Spec": {
      "description": "Ensure dependencies are met",
      "selector": {
        "match_labels": { "phase": "preflight" }
      },
      "instance_mode": "immediate",
      "wait_for": {},
      "retry": { "max_attempts": 3 },
      "on_failure": {
        "action": "raise",
        "spec": {
          "message": ["Preflight failed"],
          "notify": { "email": "ops@example.com" }
        }
      }
    }
  }
]
```

The protocol accepts a **Phase Manifest** as a `POST` body at `/plan`. The content type must be:

```http
Content-Type: application/vnd.phase-manifest+json
```

### Example

```json
[
  {
    "kind": "Phase",
    "id": "preflight",
    "spec": {
      "description": "Ensure dependencies are met",
      "selector": {
        "matchLabels": { "phase": "preflight" }
      },
      "instanceMode": "immediate",
      "waitFor": {},
      "retry": { "maxAttempts": 3 },
      "onFailure": {
        "action": "raise",
        "spec": {
          "message": ["Preflight failed"],
          "notify": { "email": "ops@example.com" }
        }
      }
    }
  }
]
```

### Top-Level Fields

| Field  | Type   | Description                     |
| ------ | ------ | ------------------------------- |
| `kind` | string | Always `"Phase"`                |
| `id`   | string | Unique identifier for the phase |
| `spec` | object | Execution spec (see below)      |

---

## Phase Specification Schema

A `spec` object supports the following fields:

| Field          | Type            | Description                     |
| -------------- | --------------- | ------------------------------- |
| `description`  | string          | Human-readable summary          |
| `selector`     | matchLabels map | Used to match execution targets |
| `instanceMode` | string (opt)    | e.g. `"immediate"` or `"onUse"` |
| `waitFor`      | object (opt)    | List of phases to wait for      |
| `retry`        | object (opt)    | Retry policy                    |
| `onFailure`    | object (opt)    | Failure handler spec            |
| `onSuccess`    | object (opt)    | Success handler spec            |

---

## Scheduling Semantics

The PMP supports flexible orchestration strategies. Systems may:

- Allow **Planter** (or another PMP server) to direct the next target phase
- Let the **runtime engine** (e.g. Plantangenet session) decide phase progression based on internal state
- Combine both in a **hybrid model** where Planter proposes phase targets and the runtime acknowledges or rejects them

### Recommended Hybrid Model

- **Planter computes dependency graphs** and identifies ready phases
- **Planter publishes targets** via:

```nats
topic: plan.session.<id>.target
```

#### Example message:
```json
{
  "phaseId": "setup",
  "reason": "dependencies satisfied"
}
```

- The runtime responds on:
```nats
topic: plan.session.<id>.ack
```

#### Example message:
```json
{
  "phaseId": "setup",
  "accepted": true,
  "queued": true
}
```

This enables the PMP server to remain canonical while the runtime maintains autonomy over lifecycle nuances.

---

## HTTP Endpoints

### `POST /plan`

Submit a new manifest.

- **Content-Type**: `application/vnd.phase-manifest+json`
- **Body**: JSON array of `Phase` objects

**Responses**:
- `200 OK`: Plan accepted
- `400 Bad Request`: Invalid manifest
- `409 Conflict`: Already executing or conflicting manifest

### Optional Endpoints (Planned)

| Endpoint      | Description                                   |
| ------------- | --------------------------------------------- |
| `GET /status` | Return live execution status                  |
| `GET /diff`   | Show differences from previously applied plan |
| `POST /apply` | Apply and commit the current plan             |
| `GET /logs`   | Retrieve structured execution logs            |

---

## NATS Interface

Planter and other PMP servers may expose a NATS-based interface for reactive and distributed usage.

### Subject Schema

| Subject Pattern                   | Purpose                        |
|----------------------------------|--------------------------------|
| `plan.session.<id>.start`        | Send manifest to begin session |
| `plan.session.<id>.control`      | Control commands (pause, etc.) |
| `plan.session.<id>.log`          | Log events stream              |
| `plan.session.<id>.events`       | Lifecycle events (start, fail) |
| `plan.session.<id>.state`        | Per-phase status updates       |
| `plan.session.<id>.get_state`    | Request/reply for full state   |
| `plan.session.<id>.diff`         | Request/reply for diff info    |
| `plan.session.<id>.target`       | Proposed next phase target     |
| `plan.session.<id>.ack`          | Runtime acknowledgment of phase|

### Message Schema (examples)

#### `plan.session.<id>.start`
```json
{
  "manifest": [ /* Phase objects */ ],
  "dryRun": false
}
```

#### `plan.session.<id>.control`
```json
{
  "command": "pause" | "resume" | "cancel"
}
```

#### `plan.session.<id>.state`
```json
{
  "phaseId": "preflight",
  "status": "running" | "complete" | "failed",
  "updated": "2025-07-19T20:00:00Z"
}
```

---

## Media Type Registration (Proposed)

```txt
Type name: application
Subtype name: vnd.phase-manifest+json
Required parameters: none
Optional parameters: version
Encoding considerations: UTF-8
Security considerations: None intrinsic; implementers must ensure endpoint access control and validation
```

---

## Compliance

To be considered **PMP-compliant**, a tool must:

1. Produce valid Phase Manifest JSON matching this spec
2. Transmit via HTTP or NATS using the schemas above
3. Respect execution order and handler results
4. Avoid extending `spec` with conflicting reserved fields

Custom extensions are allowed, provided they are prefixed (e.g. `x-`).

---

## References

- [JSON](https://www.json.org/)
- [RFC 6838](https://tools.ietf.org/html/rfc6838): Media Type Specifications
- [Kubernetes Object Model](https://kubernetes.io/docs/concepts/overview/working-with-objects/kubernetes-objects/)
- [Planter](https://github.com/queuetue/planter)
- [Meatball](https://github.com/queuetue/meatball)
- [NATS Protocol](https://docs.nats.io/)

---

## License

This protocol is released under the [MIT License](LICENSE).
