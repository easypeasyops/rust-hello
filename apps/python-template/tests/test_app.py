import re
from pathlib import Path

from prometheus_client import REGISTRY
from starlette.testclient import TestClient

from app.main import app

client = TestClient(app)


def _count(route: str, status: str) -> float:
    labels = {"method": "GET", "route": route, "status": status}
    return REGISTRY.get_sample_value("http_requests_total", labels) or 0.0


def test_hello() -> None:
    response = client.get("/")
    assert response.status_code == 200
    assert response.json() == {"hello": "world"}


def test_the_probes_answer() -> None:
    assert client.get("/healthz").status_code == 200
    assert client.get("/readyz").status_code == 200


def test_requests_are_counted_by_route_and_status() -> None:
    before = _count("/", "200")
    client.get("/")
    assert _count("/", "200") == before + 1
    labels = {"method": "GET", "route": "/"}
    assert REGISTRY.get_sample_value("http_request_duration_seconds_count", labels)


def test_unknown_paths_share_one_route_label() -> None:
    before = _count("unmatched", "404")
    assert client.get("/no/such/path").status_code == 404
    assert client.get("/another").status_code == 404
    assert _count("unmatched", "404") == before + 2


def test_ci_pins_the_platform_commit_in_platform_lock() -> None:
    """CI's reusable workflows, and the platform checkout its deploy job makes, are the
    commit platform.lock names: one platform version per commit of this repo."""
    root = Path(__file__).parents[3]
    locked = re.findall(
        r"easypeasyops/platform@([0-9a-f]{40})", (root / "platform.lock").read_text()
    )
    ci = (root / ".github/workflows/ci.yml").read_text()
    pinned = re.findall(r"easypeasyops/platform/[\w./-]+@([0-9a-f]{40})", ci)
    pinned += re.findall(r"ref: ([0-9a-f]{40}) # platform\.lock", ci)
    assert len(locked) == 1
    assert len(pinned) >= 3  # a check, an image and the deploy checkout, per template
    assert set(pinned) == set(locked)
