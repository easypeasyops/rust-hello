package app;

import io.micrometer.core.instrument.Counter;
import io.micrometer.core.instrument.MeterRegistry;
import io.micrometer.core.instrument.Timer;
import jakarta.servlet.FilterChain;
import jakarta.servlet.ServletException;
import jakarta.servlet.http.HttpServletRequest;
import jakarta.servlet.http.HttpServletResponse;
import java.io.IOException;
import java.time.Duration;
import java.util.stream.DoubleStream;
import org.springframework.stereotype.Component;
import org.springframework.web.filter.OncePerRequestFilter;
import org.springframework.web.servlet.HandlerMapping;

/**
 * Counts and times every request on the app's port as {@code http_requests_total} and {@code
 * http_request_duration_seconds}, the names the platform reads (Spring's own {@code
 * http_server_requests_seconds} is not one of them). Labelled by route, never by raw path: one
 * series per ID or per scanner's guess would grow without bound. A route with a path variable is
 * labelled by its pattern ({@code /items/{id}}), a path no route matches as {@code unmatched}.
 */
@Component
class RequestMetrics extends OncePerRequestFilter {
  /**
   * The buckets Python's prometheus_client uses, so every app's histograms line up. An SLO's
   * latency threshold must be one of them (deploy/prometheus/slos.yaml).
   */
  private static final Duration[] BUCKETS =
      DoubleStream.of(0.005, 0.01, 0.025, 0.05, 0.075, 0.1, 0.25, 0.5, 0.75, 1, 2.5, 5, 7.5, 10)
          .mapToObj(s -> Duration.ofNanos((long) (s * 1e9)))
          .toArray(Duration[]::new);

  private final MeterRegistry registry;

  RequestMetrics(MeterRegistry registry) {
    this.registry = registry;
  }

  @Override
  protected void doFilterInternal(
      HttpServletRequest request, HttpServletResponse response, FilterChain chain)
      throws ServletException, IOException {
    long start = System.nanoTime();
    // What the client sees if the app throws: the container answers 500 after this filter.
    boolean threw = true;
    try {
      chain.doFilter(request, response);
      threw = false;
    } finally {
      Object pattern = request.getAttribute(HandlerMapping.BEST_MATCHING_PATTERN_ATTRIBUTE);
      String route = pattern == null ? "unmatched" : pattern.toString();
      String status = threw ? "500" : Integer.toString(response.getStatus());
      String method = request.getMethod();
      Counter.builder("http.requests")
          .description("Requests served")
          .tags("method", method, "route", route, "status", status)
          .register(registry)
          .increment();
      Timer.builder("http.request.duration")
          .description("Time to answer")
          .tags("method", method, "route", route)
          .serviceLevelObjectives(BUCKETS)
          .register(registry)
          .record(Duration.ofNanos(System.nanoTime() - start));
    }
  }
}
