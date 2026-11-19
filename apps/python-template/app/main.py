"""A minimal service with what the platform's service chart expects of an app:

* port 8000: the app's routes, `/healthz` (liveness: the process runs) and `/readyz`
  (readiness: it can serve);
* port METRICS_PORT (9100): `/metrics` for Prometheus, on a port of its own so the public
  port carries none of it. `http_requests_total` and `http_request_duration_seconds` are
  the names the platform's SLO rules, canary analysis and dashboard read.

`python -m app.main`. Replace `hello` with the app; keep the probes and the metrics.
"""

import os
import time

import uvicorn
from prometheus_client import Counter, Histogram, start_http_server
from starlette.applications import Starlette
from starlette.requests import Request
from starlette.responses import JSONResponse, PlainTextResponse, Response
from starlette.routing import Route
from starlette.types import ASGIApp, Message, Receive, Scope, Send

REQUESTS = Counter("http_requests_total", "Requests served", ["method", "route", "status"])
# The default buckets; an SLO's latency threshold must be one of their edges
# (deploy/prometheus/slos.yaml).
LATENCY = Histogram("http_request_duration_seconds", "Time to answer", ["method", "route"])


async def hello(request: Request) -> Response:
    return JSONResponse({"hello": "world"})


async def healthz(request: Request) -> Response:
    return PlainTextResponse("ok")


async def readyz(request: Request) -> Response:
    # Answer 503 here while something the app can't serve without (its database, a broker)
    # is unreachable: the pod then gets no traffic, and is not restarted.
    return PlainTextResponse("ok")


ROUTES = [Route("/", hello), Route("/healthz", healthz), Route("/readyz", readyz)]
_PATHS = frozenset(route.path for route in ROUTES)


class Metrics:
    """Counts and times every request, labelled by route, never by raw path: one series per
    ID or per scanner's guess would grow without bound. A route with a path parameter is
    labelled by its template (`/items/{id}`)."""

    def __init__(self, app: ASGIApp) -> None:
        self.app = app

    async def __call__(self, scope: Scope, receive: Receive, send: Send) -> None:
        if scope["type"] != "http":
            await self.app(scope, receive, send)
            return
        status = 500  # what the client sees if the app raises before responding
        start = time.perf_counter()

        async def send_and_note(message: Message) -> None:
            nonlocal status
            if message["type"] == "http.response.start":
                status = message["status"]
            await send(message)

        try:
            await self.app(scope, receive, send_and_note)
        finally:
            route = scope["path"] if scope["path"] in _PATHS else "unmatched"
            REQUESTS.labels(scope["method"], route, str(status)).inc()
            LATENCY.labels(scope["method"], route).observe(time.perf_counter() - start)


app = Metrics(Starlette(routes=ROUTES))


def main() -> None:  # pragma: no cover  # scripts/smoke.sh runs it, in the image
    start_http_server(int(os.environ.get("METRICS_PORT", "9100")))
    # The kubelet and the Service reach the pod's IP. uvicorn handles SIGTERM itself: it
    # finishes the requests in hand, then exits 0.
    uvicorn.run(app, host="0.0.0.0", port=8000, log_level="info")  # noqa: S104


if __name__ == "__main__":  # pragma: no cover
    main()
