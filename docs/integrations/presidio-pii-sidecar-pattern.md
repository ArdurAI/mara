# Presidio-class PII masking (M2-19)

Mara ships **built-in regex redaction** and privacy modes (`docs/privacy-modes-m1-07.md`). For **Microsoft Presidio**-grade detectors (NER, context-aware PII), run Presidio as a **sidecar or upstream HTTP service** that scrubs prompts **before** they reach the model or Mara's proxy listener.

## Pattern

1. Client → **Presidio Analyzer / Anonymizer API** → Mara `llm_proxy` → upstream LLM.
2. Or: Client → Mara proxy → upstream, with a **policy hook** (future) calling Presidio on opted-in traffic only.

## Failure modes

- Presidio timeouts become user-visible latency; set strict deadlines and fail closed or open per policy.
- False positives may strip legitimate tokens; log Presidio decisions separately from Mara events.

This path remains **opt-in**; the builtin regex redactor stays the default lightweight control.
