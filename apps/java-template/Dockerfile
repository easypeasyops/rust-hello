# syntax=docker/dockerfile:1
#
# Two stages: a JDK that builds the jar, and a distroless JRE that runs it. What the
# platform's service chart needs of the image: it runs as UID 10001 (the chart's runAsUser),
# writes nothing outside /tmp (the chart mounts the root read-only and an emptyDir on /tmp,
# where Tomcat keeps its work files) and handles SIGTERM (exec-form ENTRYPOINT, java is PID 1).
#
# Base images are pinned by digest (tags move); Dependabot bumps the digests. Keep both on
# the same Java feature release as pom.xml's java.version.

# ---------------------------------------------------------------------------- builder
# The jar is the same on every architecture: build it natively, not under QEMU.
FROM --platform=$BUILDPLATFORM eclipse-temurin:25-jdk-noble@sha256:80f67fb3d6f76e2d817d98a4cc2a9cf05f054d4e6adc5e2144bde671d722e63f AS builder

# mvnw needs unzip to unpack the Maven it pins (and checks by checksum). Builder only.
RUN apt-get update \
 && apt-get install --yes --no-install-recommends unzip \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /src
COPY .mvn .mvn
COPY mvnw pom.xml ./
# Dependencies only: this layer is rebuilt only when pom.xml changes.
RUN --mount=type=cache,target=/root/.m2 ./mvnw --batch-mode --no-transfer-progress dependency:go-offline
COPY src/main src/main
# Tests, formatting and coverage are CI's check job (reusable-java-check.yml), not the image's.
RUN --mount=type=cache,target=/root/.m2 ./mvnw --batch-mode --no-transfer-progress package \
    -Dmaven.test.skip=true -Dspotless.check.skip=true -Djacoco.skip=true

# ---------------------------------------------------------------------------- runtime
# Distroless: a JRE and nothing else (no shell, no package manager), so little to scan.
FROM gcr.io/distroless/java21-debian13:nonroot@sha256:6b60e44d652d66a9e2e584364c96adac4df7133674b2834b6f71741b7cdb1ff3 AS runtime

WORKDIR /app
# Owned by root and only readable by the app: a compromised process can't rewrite it.
COPY --from=builder /src/target/app.jar ./app.jar

# Fixed, non-root UID/GID: predictable file ownership and Kubernetes `runAsNonRoot`.
USER 10001:10001
EXPOSE 8000 9100

# The heap follows the container's memory limit. No perf data file: nothing to write.
ENTRYPOINT ["java", "-XX:MaxRAMPercentage=75", "-XX:-UsePerfData", "-jar", "/app/app.jar"]
