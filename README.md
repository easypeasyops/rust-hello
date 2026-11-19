# rust-hello

[![CI](https://github.com/easypeasyops/rust-hello/actions/workflows/ci.yml/badge.svg)](https://github.com/easypeasyops/rust-hello/actions/workflows/ci.yml)

A minimal axum service on
[easypeasyops/platform](https://github.com/easypeasyops/platform), started from
[app-template](https://github.com/easypeasyops/app-template)'s `rust-template`. It answers
`{"hello":"world"}` on `/`; staging is `http://staging.rust-hello.localhost`.

The app lives in `apps/rust-hello/`: the service (`src/`), its image (`Dockerfile`,
proven by `scripts/smoke.sh`), its values for the platform's service chart
(`deploy/service/values.yaml`, `deploy/envs/<env>.yaml`) and its SLOs
(`deploy/prometheus/slos.yaml`). app-template's README explains each piece and the app
contract they fill in.

## Develop

From `apps/rust-hello` (rustup; `rust-toolchain.toml` pins the compiler, rustup installs
it on the first `cargo` call):

```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings
cargo llvm-cov --ignore-filename-regex 'src/main\.rs$'   # tests, with coverage: what CI runs
docker build -t rust-hello:dev . && scripts/smoke.sh
```

## Release and deploy

Bump `version` in `apps/rust-hello/Cargo.toml` (via PR), merge, then tag the merge commit
`rust-hello/vX.Y.Z` and push the tag. The release's notes hold the image's digest: set it
as `image.digest` in `apps/rust-hello/deploy/envs/staging.yaml` (a PR; Argo CD deploys it
on merge), then the same digest in `prod.yaml`.

## SLO burn-rate alerts

The SLO alerts' runbook link points here (the app's runbook). An alert
names the SLO (`slo` label) and the namespace; it fires when the error budget is being
spent faster than the objective allows. Start from the platform's SLO dashboard in
Grafana (traffic, errors and latency by route), then the app's logs.
