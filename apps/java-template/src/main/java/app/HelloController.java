package app;

import java.util.Map;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.RestController;

@RestController
class HelloController {
  @GetMapping("/")
  Map<String, String> hello() {
    return Map.of("hello", "world");
  }

  @GetMapping("/healthz")
  String healthz() {
    return "ok";
  }

  @GetMapping("/readyz")
  String readyz() {
    // Answer 503 here while something the app can't serve without (its database, a broker)
    // is unreachable: the pod then gets no traffic, and is not restarted.
    return "ok";
  }
}
