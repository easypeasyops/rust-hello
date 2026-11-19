# syntax=docker/dockerfile:1
#
# Two stages: a Rust toolchain that builds the binary, and a distroless image that runs it.
# What the platform's service chart needs of the image: it runs as UID 10001 (the chart's
# runAsUser), writes nothing outside /tmp (the chart mounts the root read-only and an
# emptyDir on /tmp; this app writes nothing at all) and handles SIGTERM (exec-form
# ENTRYPOINT, the binary is PID 1 and shuts down gracefully on it).
#
# Base images are pinned by digest (tags move); Dependabot bumps the digests. Keep the rust
# image on rust-toolchain.toml's release, and both images on the same Debian (13, trixie):
# the binary links against the builder's glibc and runs on the runtime's.

# ---------------------------------------------------------------------------- builder
# Built natively and cross-compiled to the target architecture: compiling Rust under QEMU
# takes many times longer.
FROM --platform=$BUILDPLATFORM rust:1.100-slim-trixie@sha256:6f14b2b4b278ec0d50aa1d69541c304ce0f95e0f6268487aa9924af1c2e9aebe AS builder
ARG BUILDARCH TARGETARCH

# Cross builds only: the target's C toolchain, for linking.
RUN case "$TARGETARCH" in \
      amd64) echo x86_64-unknown-linux-gnu >/target; gcc=gcc-x86-64-linux-gnu ;; \
      arm64) echo aarch64-unknown-linux-gnu >/target; gcc=gcc-aarch64-linux-gnu ;; \
      *) echo "unsupported architecture $TARGETARCH" >&2; exit 1 ;; \
    esac \
 && if [ "$TARGETARCH" != "$BUILDARCH" ]; then \
      apt-get update \
      && apt-get install --yes --no-install-recommends "$gcc" libc6-dev-"$TARGETARCH"-cross \
      && rm -rf /var/lib/apt/lists/*; \
    fi

WORKDIR /src
COPY rust-toolchain.toml ./
RUN rustup toolchain install && rustup target add "$(cat /target)"
ENV CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-linux-gnu-gcc \
    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc

COPY Cargo.toml Cargo.lock ./
COPY src src
# The registry and target/ are cache mounts: a rebuild compiles only what changed. Tests,
# formatting, lints and coverage are CI's check job (reusable-rust-check.yml), not the image's.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target,id=rust-template-target-$TARGETARCH \
    target=$(cat /target) \
 && if [ "$TARGETARCH" = "$BUILDARCH" ]; then \
      unset CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER; \
    fi \
 && cargo build --release --locked --target "$target" \
 && cp "target/$target/release/rust-template" /rust-template

# ---------------------------------------------------------------------------- runtime
# Distroless cc: glibc and libgcc, nothing else (no shell, no package manager), so little to
# scan.
FROM gcr.io/distroless/cc-debian13:nonroot@sha256:cb87acc574abe719b6e5c547384defa2f7b1f24af9605d3f618cf852495d8be2 AS runtime

# Owned by root and only executable by the app: a compromised process can't rewrite it.
COPY --from=builder /rust-template /usr/local/bin/rust-template

# Fixed, non-root UID/GID: predictable file ownership and Kubernetes `runAsNonRoot`.
USER 10001:10001
EXPOSE 8000 9100

ENTRYPOINT ["/usr/local/bin/rust-template"]
