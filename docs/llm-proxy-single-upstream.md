# Single-upstream LLM proxy semantics (M2-21)

Each `[[adapters.llm_proxy]]` entry configures **exactly one** upstream base URI (`upstream = "http://host:port"`). Mara does **not** perform automatic failover, load balancing, or weighted routing across multiple upstreams.

Operators who need HA should place a production-grade reverse proxy or service mesh **in front of** Mara, or run multiple Mara processes with distinct listen ports and external routing.

If multi-upstream is added in the future, the project will document explicit retry, health-check, and selection semantics—this doc captures today’s **one upstream** behavior to avoid opaque gateway routing surprises.
