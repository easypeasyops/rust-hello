#!/usr/bin/env bash
# Black-box smoke test of the built image, run the way the platform's service chart runs
# it: as UID 10001 on a read-only root filesystem. It answers the probes, serves metrics on
# their own port and stops cleanly on SIGTERM.
#
#   IMAGE=rust-hello:dev scripts/smoke.sh
set -euo pipefail

IMAGE="${IMAGE:-rust-hello:dev}"
NAME="rust-hello-smoke-$$"
fail() { echo "FAIL: $*" >&2; exit 1; }
pass() { echo "ok   $*"; }
cleanup() {
  status=$?
  [[ $status -eq 0 ]] || { echo "--- container logs ---"; docker logs "$NAME" 2>&1 | tail -50 || true; }
  docker rm -f "$NAME" >/dev/null 2>&1 || true
  exit "$status"
}
trap cleanup EXIT

# The chart mounts an emptyDir on /tmp; --tmpfs stands in for it.
docker run -d --name "$NAME" --read-only --tmpfs /tmp --cap-drop ALL \
  -p 127.0.0.1:8000:8000 -p 127.0.0.1:9100:9100 "$IMAGE" >/dev/null

code=""
for _ in $(seq 1 30); do
  code=$(curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:8000/healthz || true)
  [[ "$code" == 200 ]] && break
  sleep 1
done
[[ "$code" == 200 ]] || fail "/healthz returned ${code:-nothing} after 30 s"
pass "/healthz 200 on a read-only root filesystem"

# Distroless has no `id`: read the user the image runs as.
uid=$(docker inspect -f '{{.Config.User}}' "$IMAGE")
[[ "${uid%%:*}" == 10001 ]] || fail "the image runs as $uid (the chart runs it as 10001)"
pass "runs as uid 10001"

code=$(curl -s -o /dev/null -w '%{http_code}' -m 10 http://127.0.0.1:8000/readyz)
[[ "$code" == 200 ]] || fail "/readyz returned $code (want 200)"
pass "/readyz 200"

# Metrics on their own port, with the probe counted by route.
metrics=$(curl -s http://127.0.0.1:9100/metrics) # captured: grep -q would SIGPIPE curl
grep -q 'http_requests_total{method="GET",route="/readyz",status="200"}' <<<"$metrics" \
  || fail "no request metrics on :9100"
pass "metrics on :9100"

# SIGTERM (what Kubernetes sends) stops it cleanly: the binary is PID 1, not behind a shell,
# finishes the requests in hand and exits 0; 137 would mean docker had to kill it.
docker stop -t 25 "$NAME" >/dev/null
code=$(docker inspect -f '{{.State.ExitCode}}' "$NAME")
[[ "$code" == 0 ]] || fail "exited $code on SIGTERM (want 0)"
logs=$(docker logs "$NAME" 2>&1) # captured: grep -q would SIGPIPE docker logs
grep -q "graceful shutdown complete" <<<"$logs" || fail "no graceful shutdown in the logs"
pass "clean exit on SIGTERM"

echo "smoke test passed"
