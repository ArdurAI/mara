FROM gcr.io/distroless/static-debian12:nonroot

LABEL org.opencontainers.image.title="Mara"
LABEL org.opencontainers.image.description="AI-native telemetry shipper for AI agents and LLM workloads."
LABEL org.opencontainers.image.source="https://github.com/ArdurAI/mara"
LABEL org.opencontainers.image.licenses="Apache-2.0"

COPY mara /usr/local/bin/mara

ENV MARA_CONFIG=/etc/mara/mara.toml \
    XDG_STATE_HOME=/var/lib/mara \
    XDG_DATA_HOME=/var/lib/mara

USER 65532:65532
EXPOSE 4317 4318 9099

ENTRYPOINT ["/usr/local/bin/mara"]
CMD ["run"]
