# Whispra
<img width="784" height="320" alt="whispra-logo" src="https://github.com/user-attachments/assets/9f96f31f-514d-4d8b-9ed9-8ac56bace363" />


[![License](https://img.shields.io/badge/License-AGPLv3-blue.svg)](LICENSE)
[![GitHub Stars](https://img.shields.io/github/stars/Chaotic-Bastard-Creations/Whispra)](...)

**A privacy-first, open-source messaging platform. No spying. No metadata. No bullshit.**

[Documentation] • [Self-Host Guide] • [Roadmap]

##  Features
- End-to-end encryption
- Voice & video chat
- Community servers
- Zero telemetry / metadata logging
- Fully self-hostable
- AGPLv3 licensed



###  Essential Files to Add
| File                    | Purpose                              | Priority |
|-------------------------|--------------------------------------|----------|
| `LICENSE`               | AGPLv3                               | Must     |
| `README.md`             | Main landing                         | Must     |
| `CONTRIBUTING.md`       | How to contribute                    | High     |
| `CODE_OF_CONDUCT.md`    | Professional standard                | High     |
| `.github/ISSUE_TEMPLATE`| Bug + Feature request templates      | High     |
| `.github/PULL_REQUEST_TEMPLATE` | PR template                   | High     |
| `docs/` folder          | Documentation                        | Medium   |
| `SECURITY.md`           | Security policy                      | Medium   |

---

## Running the Bridge

The `whispra-bridge` binary exposes the Whispra bulletin-board protocol to a browser-based frontend over local HTTP and WebSocket.

### Build

```sh
cargo build --bin whispra-bridge --bin whispra-server
```

### Start the upstream server

```sh
cargo run --bin whispra-server
```

### Start the bridge

```sh
cargo run --bin whispra-bridge -- \
  --upstream 127.0.0.1:3000 \
  --upstream-key <64-hex-char server static public key> \
  --listen 127.0.0.1:7000
```

The bridge completes a Noise NK handshake with the upstream server, starts the epoch loop (one PUT + 16 GETs every 250 ms), and begins listening for browser connections.

### HTTP endpoints (all JSON)

| Method | Path        | Body                                                   | Description                              |
|--------|-------------|--------------------------------------------------------|------------------------------------------|
| `POST` | `/pair`     | `{"name":"alice","role":"initiator","secret_hex":"…"}` | Derive a contact from a shared secret. `role` is `"initiator"` or `"responder"`. |
| `POST` | `/send`     | `{"to":"alice","message":"hello"}`                     | Queue a message for the next epoch PUT.  |
| `GET`  | `/contacts` | —                                                      | List contacts (name, role, counters).    |
| `GET`  | `/status`   | —                                                      | `{"connected":true,"contact_count":N}`   |
| `POST` | `/quit`     | —                                                      | Graceful shutdown.                       |

Error responses: `{"error":"…"}` with an appropriate HTTP status code.

### WebSocket event stream

```
GET ws://127.0.0.1:7000/events
```

Connect with a browser `WebSocket` or `websocat ws://127.0.0.1:7000/events`. The bridge fans out JSON text frames:

```jsonc
{"type":"message","from":"alice","counter":0,"payload":"hello"}
{"type":"contact_added","name":"alice"}
{"type":"status","connected":true}
{"type":"lagged"}
```

`"lagged"` means the subscriber fell behind the broadcast channel; refetch `/contacts` and `/status` to re-sync.

### Quick smoke-test

```sh
# Pair with alice
curl -s -X POST http://localhost:7000/pair \
  -H 'Content-Type: application/json' \
  -d '{"name":"alice","role":"initiator","secret_hex":"deadbeef…"}'

# Send a message
curl -s -X POST http://localhost:7000/send \
  -H 'Content-Type: application/json' \
  -d '{"to":"alice","message":"hi"}'

# Watch the event stream
websocat ws://127.0.0.1:7000/events
```
