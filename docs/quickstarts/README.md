# Mara quickstarts (M2-07)

Smoke-tested patterns:

| Pack | File | Use case |
|------|------|------------|
| Ollama-heavy | [ollama-heavy.toml](ollama-heavy.toml) | Local Ollama + JSONL capture |
| Mixed-runtime | [mixed-runtime.toml](mixed-runtime.toml) | OTLP ingest + file sink (adjust ports) |
| Privacy-first | [privacy-first.toml](privacy-first.toml) | Regex redaction + metadata-friendly defaults |

Copy one file to `mara.toml`, adjust paths and ports, then run `mara run --config mara.toml`.
