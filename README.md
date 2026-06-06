# uplog

**A lightweight, single-binary system agent written in Rust.** uplog runs quietly in the background on any server or developer machine. It tails log files, collects system metrics on a schedule, batches everything together, and ships it as JSON to any HTTP endpoint you configure.

**Backend-agnostic by design.** uplog is not tied to any specific logging platform or observability stack. Point it at your logs, set your ingest URL, and it works with whatever backend accepts an HTTP POST — your own API, a serverless function, a queue, or a future project like [Sentinel](https://github.com/diptanshu1044).

> Tails log files. Collects CPU, memory, disk, and network metrics. Ships batched JSON to any HTTP endpoint. One binary, zero runtime dependencies.

| Resource | Link |
| --- | --- |
| Website | [uplog.in](https://uplog.in) |
| GitHub | [github.com/diptanshu1044/uplog](https://github.com/diptanshu1044/uplog) |
| crates.io | [crates.io/crates/uplog](https://crates.io/crates/uplog) |
| License | MIT |

---

## Table of Contents

- [Why uplog?](#why-uplog)
- [Features](#features)
- [Quick Start](#quick-start)
- [Installation](#installation)
- [Configuration](#configuration)
- [JSON Payload](#json-payload)
- [Backend Integration](#backend-integration)
- [CLI Reference](#cli-reference)
- [Running with pm2](#running-with-pm2)
- [How It Works](#how-it-works)
- [Building from Source](#building-from-source)
- [Pre-built Binaries](#pre-built-binaries)
- [Platform Notes](#platform-notes)
- [Troubleshooting](#troubleshooting)
- [Roadmap](#roadmap)
- [Contributing](#contributing)
- [License](#license)

---

## Why uplog?

Most log shipping agents are either heavyweight (require Java, Elasticsearch, or a full observability suite) or tightly coupled to a vendor. uplog takes the opposite approach:

- **Single static binary** — no Node.js, Python, or JVM on the host
- **Push model** — works behind firewalls; your agent calls out, nothing listens inbound
- **Minimal config** — one TOML file, four sections, done
- **Resilient** — retries on network failure and keeps running; never exits because the backend went down
- **Cross-platform** — Linux, macOS, and Windows from one codebase

If you run Node.js apps under [pm2](https://pm2.keymetrics.io/), uplog fits your workflow naturally: install the binary, run `uplog init`, and `pm2 start uplog -- start`.

---

## Features

| Capability | Description |
| --- | --- |
| **Log tailing** | Watches one or more log files continuously. New lines are captured and buffered — like `tail -f`, but programmatic. Starts at the end of each file (no historical replay). |
| **Metric collection** | On a configurable interval, collects CPU %, memory (used/total), disk (used/total), and network bytes sent/received. |
| **Batching** | Log lines and the latest metrics snapshot are held in memory and shipped together. Your backend is not hammered on every new log line. |
| **HTTP shipping** | POSTs JSON to any endpoint on a schedule. Optional `Authorization: Bearer <api_key>` header. Retries up to 3 times with a 5-second delay between attempts. |
| **Config discovery** | Searches four standard locations for `uplog.toml`. Override with `--config`. |
| **Interactive setup** | `uplog init` walks you through creating `~/.uplog.toml`. |
| **Validation** | `uplog check` loads and validates config without starting the agent. |

---

## Quick Start

**1. Install uplog**

```bash
curl -fsSL https://uplog.in/install | sh
```

Or download a pre-built binary from [GitHub Releases](https://github.com/diptanshu1044/uplog/releases).

**2. Create a config**

```bash
uplog init
```

This writes `~/.uplog.toml` interactively. Defaults are sensible for pm2 users (`~/.pm2/logs/`, `http://localhost:3000/ingest`).

**3. Validate**

```bash
uplog check
```

**4. Start under pm2**

```bash
pm2 start uplog -- start
pm2 save
```

Your backend will start receiving batched log lines and system metrics within one ship interval (default: 60 seconds).

---

## Installation

### Install script (recommended)

```bash
curl -fsSL https://uplog.in/install | sh
```

The script detects your OS and architecture, downloads the matching release binary, and places `uplog` on your `PATH`.

### GitHub Releases

Download the asset for your platform from the [latest release](https://github.com/diptanshu1044/uplog/releases):

| Asset | Platform |
| --- | --- |
| `uplog-linux-x86_64` | Linux (x86_64, musl) |
| `uplog-linux-aarch64` | Linux (ARM64, musl) |
| `uplog-macos-x86_64` | macOS (Intel) |
| `uplog-macos-aarch64` | macOS (Apple Silicon) |
| `uplog-windows-x86_64.exe` | Windows (x86_64) |

```bash
# Example: Linux x86_64
curl -LO https://github.com/diptanshu1044/uplog/releases/latest/download/uplog-linux-x86_64
chmod +x uplog-linux-x86_64
sudo mv uplog-linux-x86_64 /usr/local/bin/uplog
```

### crates.io

```bash
cargo install uplog
```

Requires a Rust toolchain. Builds from source and installs to `~/.cargo/bin/uplog`.

### Build from source

See [Building from Source](#building-from-source).

---

## Configuration

uplog reads a TOML config file. Resolution order — **first existing file wins**:

| Priority | Path | Use case |
| --- | --- | --- |
| 1 | `--config <path>` | Explicit override |
| 2 | `./uplog.toml` | Project-local (good for development) |
| 3 | `~/.uplog.toml` | User-level (where `uplog init` writes) |
| 4 | `/etc/uplog/uplog.toml` | System-wide (servers, systemd) |

### Full example

```toml
[agent]
id = "prod-api-01"

[logs]
paths = [
  "~/.pm2/logs/app-out.log",
  "~/.pm2/logs/app-error.log",
  "/var/log/nginx/access.log"
]

[metrics]
collect_interval_seconds = 30

[shipper]
endpoint = "https://mybackend.com/ingest"
ship_interval_seconds = 60
api_key = "your-secret-key"
```

An annotated example lives at [`examples/uplog.toml`](examples/uplog.toml).

### Field reference

#### `[agent]`

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `id` | string | yes | Stable identifier for this machine or service. Appears as `agent_id` in every payload. Use something meaningful (`web-prod-01`, hostname, etc.). |

#### `[logs]`

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `paths` | array of strings | yes | Log files to tail. At least one path required. Supports `~` for home directory. Each file is watched in a separate async task. |

**Behavior:** uplog opens each file and seeks to the **end** — it does not ship historical content that existed before the agent started. If a file does not exist yet, uplog logs a warning and retries every 2 seconds until it appears.

#### `[metrics]`

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `collect_interval_seconds` | integer | yes | How often to sample system metrics. Must be greater than 0. Default in `init`: 30. |

#### `[shipper]`

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `endpoint` | string (URL) | yes | HTTP(S) URL to POST JSON payloads to. |
| `ship_interval_seconds` | integer | yes | How often to ship batched data. Must be greater than 0. Default in `init`: 60. |
| `api_key` | string | no | If set, sent as `Authorization: Bearer <api_key>`. Omit for backends that do not require auth. |

### Validation rules

`uplog check` and `uplog start` enforce:

- `agent.id` is non-empty (whitespace-only fails)
- `logs.paths` has at least one entry
- `metrics.collect_interval_seconds` > 0
- `shipper.endpoint` is non-empty
- `shipper.ship_interval_seconds` > 0

On failure, uplog prints a clear `[uplog error]` message and exits with code 1.

---

## JSON Payload

Every ship tick, uplog POSTs a single JSON object to your endpoint.

### Top-level shape

```json
{
  "agent_id": "prod-api-01",
  "timestamp": "2024-01-15T14:03:00Z",
  "metrics": { ... },
  "logs": [ ... ]
}
```

| Field | Type | Description |
| --- | --- | --- |
| `agent_id` | string | From `[agent].id` |
| `timestamp` | ISO 8601 UTC | When the payload was assembled (ship time) |
| `metrics` | object or `null` | Latest metrics snapshot at drain time, or `null` if none collected yet |
| `logs` | array | All log lines buffered since the last successful drain (may be empty) |

### Metrics object

```json
{
  "cpu_usage_percent": 87.3,
  "memory_used_mb": 3200,
  "memory_total_mb": 8192,
  "disk_used_gb": 45.2,
  "disk_total_gb": 100.0,
  "net_bytes_sent": 1048576,
  "net_bytes_received": 2097152,
  "collected_at": "2024-01-15T14:02:55Z"
}
```

| Field | Type | Description |
| --- | --- | --- |
| `cpu_usage_percent` | float | Global CPU usage across all cores |
| `memory_used_mb` | integer | Used RAM in megabytes |
| `memory_total_mb` | integer | Total RAM in megabytes |
| `disk_used_gb` | float | Sum of used space across mounted disks (GB) |
| `disk_total_gb` | float | Sum of total space across mounted disks (GB) |
| `net_bytes_sent` | integer | Cumulative bytes transmitted (all interfaces) |
| `net_bytes_received` | integer | Cumulative bytes received (all interfaces) |
| `collected_at` | ISO 8601 UTC | When this snapshot was taken |

Only the **latest** metrics snapshot is kept between ship ticks. If metrics are collected multiple times before a ship, earlier snapshots are overwritten.

### Log line object

```json
{
  "source": "file:/var/log/myapp.log",
  "line": "ERROR: database connection refused",
  "timestamp": "2024-01-15T14:02:58Z"
}
```

| Field | Type | Description |
| --- | --- | --- |
| `source` | string | Origin identifier. Currently `file:<path>` for tailed files. |
| `line` | string | Raw log line content (no trailing newline) |
| `timestamp` | ISO 8601 UTC | When uplog captured the line |

### HTTP request details

| Header | Value |
| --- | --- |
| `Content-Type` | `application/json` |
| `Authorization` | `Bearer <api_key>` (only if `api_key` is set in config) |

- **Method:** `POST`
- **Timeout:** 10 seconds per request
- **Retries:** 3 attempts, 5-second fixed delay between failures
- **Success:** Any 2xx status code
- **Empty batches:** If there are no logs and no metrics since the last drain, the ship tick is skipped entirely (no HTTP request)

If all retries fail, uplog logs `[uplog error] shipper: batch dropped after 3 attempts` and continues — **the process does not exit**.

---

## Backend Integration

Your backend only needs to accept `POST` requests with a JSON body. Here is a minimal handler in a few languages.

### Node.js (Express)

```javascript
const express = require("express");
const app = express();

app.use(express.json());

app.post("/ingest", (req, res) => {
  const { agent_id, timestamp, metrics, logs } = req.body;

  console.log(`[${agent_id}] ${logs.length} log lines, metrics: ${metrics ? "yes" : "no"}`);

  for (const entry of logs) {
    console.log(`  ${entry.source}: ${entry.line}`);
  }

  if (metrics) {
    console.log(`  CPU: ${metrics.cpu_usage_percent}%`);
  }

  res.sendStatus(200);
});

app.listen(3000, () => console.log("listening on :3000"));
```

### Python (Flask)

```python
from flask import Flask, request

app = Flask(__name__)

@app.post("/ingest")
def ingest():
    data = request.get_json()
    agent_id = data["agent_id"]
    logs = data.get("logs", [])
    metrics = data.get("metrics")

    print(f"[{agent_id}] {len(logs)} log lines")

    if metrics:
        print(f"  CPU: {metrics['cpu_usage_percent']}%")

    return "", 200

if __name__ == "__main__":
    app.run(port=3000)
```

### curl (manual test)

```bash
curl -X POST http://localhost:3000/ingest \
  -H "Content-Type: application/json" \
  -d '{
    "agent_id": "test",
    "timestamp": "2024-01-15T14:03:00Z",
    "metrics": null,
    "logs": [{"source": "file:/tmp/test.log", "line": "hello", "timestamp": "2024-01-15T14:02:58Z"}]
  }'
```

### Auth

If you set `api_key` in config, validate the header on your backend:

```
Authorization: Bearer your-secret-key
```

Return any non-2xx status to trigger uplog's retry logic.

---

## CLI Reference

uplog is designed to run in the **foreground** under a process manager. Six commands, nothing else.

```bash
uplog start                               # start the agent
uplog start --config /path/to/uplog.toml  # start with explicit config
uplog init                                # interactive wizard → ~/.uplog.toml
uplog check                               # validate config
uplog check --config /path/to/uplog.toml  # validate a specific file
uplog version                             # print version (e.g. uplog 0.1.0)
uplog help                                # print help text
```

Running `uplog` with no subcommand prints help automatically.

**`stop` and `status` are intentionally omitted.** uplog has no built-in daemon mode. Use your process manager instead (`pm2 stop uplog`, `systemctl stop uplog`, etc.).

### `uplog init`

Interactive wizard. Prompts in order:

1. **Agent ID** — default: `$HOSTNAME` or `my-server`
2. **Backend endpoint** — default: `http://localhost:3000/ingest`
3. **Log paths** — comma-separated, default: `~/.pm2/logs/`
4. **Metric interval** — default: 30 seconds
5. **Ship interval** — default: 60 seconds

Writes to `~/.uplog.toml`. If the file already exists, asks before overwriting. After writing, runs the same validation as `check`.

Ends with:

```
Config written to ~/.uplog.toml

Run:  pm2 start uplog -- start
```

### `uplog check`

Loads config from the search chain (or `--config`), validates all fields, and prints a summary:

```
Config valid. Loaded from: /home/user/.uplog.toml

agent id:           prod-api-01
endpoint:           https://mybackend.com/ingest
log paths:          ~/.pm2/logs/app-out.log
                    ~/.pm2/logs/app-error.log
metric interval:    30s
ship interval:      60s
```

### `uplog start`

Loads and validates config, then spawns three concurrent async tasks (log collector, metrics collector, HTTP shipper) and runs until killed. All recoverable errors are logged to stderr; the agent keeps running.

---

## Running with pm2

pm2 is the recommended process manager for Node.js developers — and uplog is built with that workflow in mind.

```bash
# One-time setup
curl -fsSL https://uplog.in/install | sh
uplog init
uplog check

# Start and persist
pm2 start uplog -- start
pm2 save
pm2 startup    # register with system init (servers)
```

Useful pm2 commands:

```bash
pm2 status uplog          # is it running?
pm2 logs uplog            # view uplog's own stderr (warnings, ship errors)
pm2 restart uplog         # restart after config change
pm2 stop uplog            # stop the agent
```

To use a custom config path:

```bash
pm2 start uplog -- start --config /etc/uplog/uplog.toml
```

### systemd (alternative)

For servers without pm2, a minimal systemd unit:

```ini
[Unit]
Description=uplog agent
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/uplog start --config /etc/uplog/uplog.toml
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
```

---

## How It Works

uplog runs three concurrent tasks on the Tokio async runtime. They share an in-memory buffer protected by `Arc<Mutex<>>`.

```
main.rs
  ├── loads config
  └── spawns 3 async tasks:
        ├── collectors/logs.rs    ──┐
        │                            ├──► buffer.rs
        ├── collectors/metrics.rs ──┘        │
        │                                    │
        └── shipper/http.rs  ◄───────────────┘
                      │
                      └──► HTTP POST to backend
```

### Log collector

- One async task per configured log path
- Opens the file, seeks to end, polls for new lines every 200ms
- Pushes each line into the shared buffer with a UTC timestamp
- If a file cannot be opened, warns and retries every 2 seconds

### Metrics collector

- Runs on `collect_interval_seconds` timer
- Uses [`sysinfo`](https://crates.io/crates/sysinfo) for cross-platform metrics
- Overwrites the latest snapshot in the buffer on each tick

### Shipper

- Runs on `ship_interval_seconds` timer (waits one full interval before the first ship)
- Drains the buffer: takes all pending logs and the latest metrics snapshot
- Serializes to JSON and POSTs to the configured endpoint
- On failure: retries up to 3 times with 5-second delays, then drops the batch and continues

### Project layout

```
uplog/
├── Cargo.toml
├── examples/uplog.toml      # annotated config example
├── LICENSE
├── README.md
└── src/
    ├── main.rs              # CLI entry point, task orchestration
    ├── config.rs            # config load, search chain, validation
    ├── models.rs            # Config, LogLine, MetricsSnapshot, Payload
    ├── buffer.rs            # shared in-memory store (push, drain)
    ├── error.rs             # typed errors and exit helpers
    ├── collectors/
    │   ├── mod.rs
    │   ├── logs.rs          # log file tailing
    │   └── metrics.rs       # system metrics collection
    ├── shipper/
    │   ├── mod.rs
    │   └── http.rs          # batch shipping with retries
    └── utils/
        ├── mod.rs
        └── time.rs
```

---

## Building from Source

**Requirements:** Rust 1.85+ (edition 2024)

```bash
git clone https://github.com/diptanshu1044/uplog.git
cd uplog
cargo build --release
```

The release binary is at `target/release/uplog`.

Release profile is optimized for size: LTO, `opt-level = "z"`, symbols stripped. Typical binary size is a few MB.

### Run tests

```bash
cargo test
```

### Cross-compile

GitHub Actions builds for all release targets using [cross](https://github.com/cross-rs/cross). To reproduce locally:

```bash
# Linux musl (from any host with cross installed)
cross build --release --target x86_64-unknown-linux-musl

# macOS (native)
cargo build --release --target aarch64-apple-darwin
```

---

## Pre-built Binaries

Tagged releases (`v*`) trigger [`.github/workflows/release.yml`](.github/workflows/release.yml), which builds and uploads:

- `uplog-linux-x86_64`
- `uplog-linux-aarch64`
- `uplog-macos-x86_64`
- `uplog-macos-aarch64`
- `uplog-windows-x86_64.exe`

Releases are created as drafts; publish manually after verifying assets.

To cut a release:

```bash
git tag v0.1.0
git push origin v0.1.0
```

---

## Platform Notes

### Disk metrics

Disk usage sums all mounted volumes. On macOS, system volumes under `/System/Volumes/` and `/private/var/vm` are excluded to avoid inflated numbers.

### Log rotation

uplog reopens a log file if reading fails (e.g. file deleted). Rotation via **rename + create** (common with `logrotate`) is a known edge case — the file descriptor may keep pointing at the old inode. Full inode-based rotation detection is planned for a future release. If you rely on rename-based rotation, restart uplog after rotation or use copy-truncate rotation instead.

### Windows

Use the `.exe` release asset. Config paths and log paths use standard Windows path syntax. `~` expansion works for the home directory in config search and `init`.

### Resource usage

uplog targets under 10 MB RAM at steady state. CPU usage is negligible between collection and ship ticks.

---

## Troubleshooting

| Symptom | Likely cause | Fix |
| --- | --- | --- |
| `config: no config file found` | No `uplog.toml` in search chain | Run `uplog init` or create config manually |
| `config: invalid field` | Missing or zero interval, empty agent id | Run `uplog check` and fix the reported field |
| `logs: cannot open <path>` | File does not exist yet | Create the file or fix the path; uplog retries automatically |
| `shipper: POST ... returned 401` | Wrong or missing API key | Match `api_key` in config with backend auth |
| `shipper: batch dropped after 3 attempts` | Backend down or unreachable | Fix endpoint; uplog will ship the next batch on the following tick |
| No data arriving | Ship interval not elapsed yet, or buffer empty | Wait for `ship_interval_seconds`; ensure logs are being written |
| pm2 shows `errored` | Config invalid at start | Run `pm2 logs uplog` and `uplog check` |

All runtime warnings and errors are printed to **stderr** with the `[uplog error]` prefix.

---

## Roadmap

### v1.1 — Reliability

- Disk-based buffer (survive agent crashes mid-batch)
- Gzip compression before shipping
- Exponential backoff on retries
- Inode-based log rotation detection

### v1.2 — Filtering

- Regex filter for log lines
- Optional metrics (disable disk or network)
- Independent collect vs. ship intervals with sampling

### v1.3 — Process Monitoring

- Watch processes by name
- Detect crashes and restarts
- Per-process CPU and memory

### v1.4 — Multiple Destinations

- Ship to multiple backends
- Route logs and metrics to different endpoints

### v2.0 — Pull Mode

- Expose `/metrics` for Prometheus-style scraping
- Backend polls instead of agent pushing

### v2.1 — Security

- Mutual TLS (mTLS) between agent and backend

---

## Contributing

Contributions are welcome — bug reports, docs improvements, and pull requests.

1. Fork the repository
2. Create a feature branch (`git checkout -b fix/log-rotation`)
3. Make your changes
4. Run `cargo test` and `cargo clippy`
5. Open a pull request with a clear description of what changed and why

Please keep changes focused. uplog is intentionally small; resist scope creep unless it maps to the roadmap.

---

## License

MIT License — see [LICENSE](LICENSE).

Copyright (c) 2026 Diptanshu Banerjee
