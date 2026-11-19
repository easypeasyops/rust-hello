package app;

import org.springframework.boot.SpringApplication;
import org.springframework.boot.autoconfigure.SpringBootApplication;

/**
 * A minimal service with what the platform's service chart expects of an app.
 *
 * <ul>
 *   <li>Port 8000: the app's routes, {@code /healthz} (liveness: the process runs) and {@code
 *       /readyz} (readiness: it can serve).
 *   <li>Port {@code METRICS_PORT} (9100): {@code /metrics} for Prometheus, on a port of its own so
 *       the public port carries none of it. {@code http_requests_total} and {@code
 *       http_request_duration_seconds} are the names the platform's SLO rules, canary analysis and
 *       dashboard read ({@link RequestMetrics}).
 * </ul>
 *
 * <p>Replace {@link HelloController#hello} with the app; keep the probes and the metrics.
 */
@SpringBootApplication
public class Application {
  public static void main(String[] args) {
    SpringApplication.run(Application.class, args);
  }
}
