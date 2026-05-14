# LLM HTTP proxy: non-loopback bind threat model

This note covers **`[[adapters.llm_proxy]]`** when `http_listen` is **not** bound to a loopback address (`127.0.0.1`, `::1`). It complements the workspace STRIDE overview in [`threat-model.md`](threat-model.md).

## Trust boundary

| Party | Trust |
|------|--------|
| Operator | Trusted to edit `mara.toml`, run `mara`, and place controls in front of the proxy. |
| Upstream LLM (Ollama, OpenAI-compat server) | Trusted for availability and correctness of model output; TLS to upstream is the operator’s responsibility (`upstream` URL scheme). |
| **Inbound HTTP clients** | **Untrusted** whenever the listen address is reachable from other principals (LAN, VPC, `0.0.0.0`, public IP). They can send arbitrary HTTP paths, headers, and bodies compatible with Ollama/OpenAI APIs. |

On **loopback-only** binds, the kernel restricts clients to local processes (same isolation model as binding Ollama on `127.0.0.1`). That is the **default posture** Mara documents for single-user laptops.

## Default: loopback-only

- There is **no** implicit `0.0.0.0` listen for the LLM proxy in shipped examples; operators choose `http_listen` explicitly.
- If `http_listen` resolves to a **non-loopback** `SocketAddr`, configuration validation **fails** unless this adapter sets:

  ```toml
  allow_non_loopback_listen = true
  ```

  Use that flag only after you accept the risks below and have **TLS termination and authentication** on a reverse proxy (or equivalent) in front of Mara’s listener.

## STRIDE (proxy-specific)

### Spoofing / tampering

- **Client identity**: the proxy does not authenticate callers. Anyone who can open a TCP connection to `http_listen` can impersonate a “legitimate” app from Mara’s perspective.
- **Header forwarding**: hop-by-hop headers are stripped; other client headers are forwarded upstream (see `http_proxy.rs`). Malicious clients can stress upstream parsers or attempt protocol smuggling; place a hardened reverse proxy in front for HTTP edge rules.

### Information disclosure

- **Prompts and completions** flow through the process. Policy stages (privacy modes, redaction) apply to **emitted events**, not necessarily to what a malicious client could infer from upstream error text returned to them.
- **Telemetry**: events may contain model names, usage, and optionally bodies depending on policy. Sinks remain the confidentiality boundary—secure them as usual.

### Denial of service

- **Body size (bounded per direction)**
  `max_body_bytes` (default 10 MiB) limits how much of each **request** and **response** body is buffered for normalization. Oversized bodies are truncated for capture; the proxy still forwards full bodies to upstream for unary responses where the implementation reads the full upstream body up to the same cap for non-streaming paths—operators should align this with upstream limits.

- **Upstream HTTP client timeouts**
  The in-process `hyper` client used for upstream forwarding does **not** currently set explicit connect/read/write timeouts. Slow or hung upstreams can tie up a task until the peer closes. Mitigations: keep upstream on loopback or a low-latency LAN, size upstream timeouts inside the model server where supported, and **terminate at a reverse proxy** that enforces deadlines.

- **Concurrent connections**
  Each accepted TCP connection is handled in a separate task; there is **no** global in-process cap today. A volumetric attack can exhaust file descriptors or CPU. Mitigate with OS firewall rules, `iptables`/nftables rate limits, cloud security groups, or a front proxy’s connection limits (see M2-15 for the separate **metrics** HTTP server cap).

### Elevation of privilege

- Binding to `0.0.0.0` or a routable IP increases exposure if the host firewall is misconfigured. Prefer binding to a **specific** interface IP or keeping the proxy on loopback and using **port publish** only from a container edge.

## Recommended pattern: TLS + auth off-process

1. Run Mara’s `http_listen` on loopback inside a network namespace, VM, or pod (e.g. `127.0.0.1:11435`).
2. Run **nginx**, **Envoy**, **HAProxy**, or a service mesh ingress with:
   - TLS 1.2+ and modern ciphers toward clients;
   - **Authentication** (mTLS, OAuth bearer validation, network ACLs + private mesh) appropriate to your threat model;
   - Request/body size limits and idle timeouts aligned with your LLM workload.
3. Forward validated traffic to Mara’s loopback listener.

This matches the broader product direction: **mTLS for non-loopback** on first-party receivers is documented as a post-MVP hardening goal in planning material; the LLM proxy path should follow the same pattern when exposed beyond localhost.

## Configuration reference

| Field | Role |
|-------|------|
| `http_listen` | `host:port` for `TcpListener::bind`. |
| `allow_non_loopback_listen` | Must be `true` when the address is not loopback-only, or `mara` refuses to start. |
| `max_body_bytes` | Per-direction capture cap (see above). |
| `upstream` | Base URL for the real LLM server (typically still `http://127.0.0.1:…` when Mara and Ollama share a host). |

## Related code and docs

- Proxy implementation: `crates/mara-adapter-llm-proxy/src/http_proxy.rs`
- Config types: `crates/mara-core/src/config.rs` (`LlmProxyAdapterConfig`)
- Operator capabilities summary: [`ollama-llm-proxy-capabilities.md`](ollama-llm-proxy-capabilities.md)
