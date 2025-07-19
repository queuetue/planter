# syntax=docker/dockerfile:1

FROM rust:1.77 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bullseye-slim
LABEL org.opencontainers.image.title="Planter"
LABEL org.opencontainers.image.description="Stateless execution engine for declarative plans (Phase Manifest Protocol)"
LABEL org.opencontainers.image.url="https://github.com/queuetue/planter"
LABEL org.opencontainers.image.source="https://github.com/queuetue/planter"
LABEL org.opencontainers.image.licenses="MIT"
LABEL org.opencontainers.image.vendor="Queuetue, LLC"
LABEL org.opencontainers.image.authors="Queuetue <scott@queuetue.com>"
LABEL maintainer="Queuetue <scott@queuetue.com>"
WORKDIR /app
COPY --from=builder /app/target/release/planter /usr/local/bin/planter
COPY . .
RUN apt-get update && apt-get install -y libssl-dev pkg-config && rm -rf /var/lib/apt/lists/*
EXPOSE 3030
CMD ["planter"]
