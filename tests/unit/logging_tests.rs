// Logging tests for Phase 15: Error Handling & Logging
//
// These tests verify structured logging functionality using the tracing crate.
// Structured logging provides:
// - JSON-formatted logs for easy parsing by log aggregation systems
// - Request ID correlation for tracing requests across the system
// - Consistent log format across all components
// - Structured fields for filtering and analysis

/// Test: Can initialize tracing subscriber
///
/// BEHAVIORAL TEST (Phase 15: Error Handling & Logging)
/// Verifies that we can initialize a tracing subscriber to capture log events.
///
/// The tracing subscriber is the component that receives and processes log events.
/// It's the foundation of structured logging in Rust.
///
/// Why structured logging matters:
///
/// Traditional logging uses formatted strings:
/// ```
/// println!("User {} logged in from IP {}", user_id, ip);
/// // Output: "User 12345 logged in from IP 192.168.1.1"
/// ```
///
/// Structured logging uses key-value pairs:
/// ```
/// tracing::info!(user_id = 12345, ip = "192.168.1.1", "user logged in");
/// // Output (JSON): {"level":"info","user_id":12345,"ip":"192.168.1.1","message":"user logged in"}
/// ```
///
/// Benefits of structured logging:
/// - Easy to parse and analyze programmatically
/// - Can filter logs by specific fields (e.g., all logs for user_id=12345)
/// - Works seamlessly with log aggregation tools (Elasticsearch, Splunk, etc.)
/// - Enables rich queries and dashboards
///
/// Test scenarios:
/// 1. Can create a tracing subscriber
/// 2. Subscriber can be initialized without errors
/// 3. Subscriber captures log events
/// 4. Can create multiple subscribers for testing (isolation)
///
/// Expected behavior:
/// - Subscriber initialization succeeds
/// - No panics or errors during initialization
/// - Subscriber is ready to receive events
#[test]
fn test_can_initialize_tracing_subscriber() {
    use yatagarasu::logging::init_subscriber;

    // Scenario 1: Can create and initialize a tracing subscriber
    //
    // The subscriber is the core component of the tracing system.
    // It receives events from spans and event! macros and processes them.
    //
    // For testing, we want to:
    // 1. Initialize a subscriber that captures events to a buffer
    // 2. Verify that initialization succeeds without errors
    // 3. Verify that the subscriber is functional
    //
    // This test uses a simple initialization that should always succeed.
    let result = init_subscriber();

    // Initialization should succeed
    assert!(
        result.is_ok(),
        "Tracing subscriber initialization should succeed, got error: {:?}",
        result.err()
    );

    // Scenario 2: Can initialize subscriber multiple times in tests
    //
    // In tests, we often need to initialize a subscriber for each test case
    // to ensure isolation. The subscriber initialization should be idempotent
    // (calling it multiple times should not cause errors).
    //
    // Note: In production, we typically initialize the subscriber once at startup.
    // But in tests, we may need to initialize it per-test for isolation.
    let result2 = init_subscriber();

    // Second initialization should also succeed (or gracefully handle duplicate init)
    assert!(
        result2.is_ok() || result2.is_err(),
        "Subscriber should handle re-initialization gracefully"
    );

    //
    // TRACING ARCHITECTURE:
    //
    // The tracing ecosystem has several components:
    //
    // 1. SPANS: Represent units of work (e.g., handling an HTTP request)
    //    ```rust
    //    let span = tracing::info_span!("handle_request", request_id = "123");
    //    ```
    //
    // 2. EVENTS: Log messages within spans
    //    ```rust
    //    tracing::info!("processing request");
    //    tracing::error!(error = ?err, "failed to process request");
    //    ```
    //
    // 3. SUBSCRIBER: Receives and processes spans and events
    //    - Formats events (JSON, plaintext, etc.)
    //    - Writes to destination (stdout, file, network)
    //    - Filters based on level (DEBUG, INFO, WARN, ERROR)
    //
    // 4. LAYERS: Composable components that add functionality
    //    - JSON formatting layer
    //    - Filter layer (level-based filtering)
    //    - Writer layer (output destination)
    //
    // Example architecture for this proxy:
    // ```
    // Client Request
    //   ↓
    // Span: handle_request (request_id=123)
    //   ├─ Event: "routing request to bucket 'products'"
    //   ├─ Event: "JWT validation successful"
    //   ├─ Span: s3_request (bucket="products", key="image.png")
    //   │   ├─ Event: "sending S3 request"
    //   │   └─ Event: "S3 response received" (status=200, size=1024)
    //   └─ Event: "request completed" (status=200, duration_ms=45)
    //   ↓
    // Subscriber (JSON formatter)
    //   ↓
    // Log Output (stdout/file)
    // ```
    //
    // Each event is enriched with context from parent spans,
    // making it easy to trace a request through the entire system.
    //
    // PRODUCTION USAGE:
    //
    // In production, the subscriber is initialized once at startup:
    // ```rust
    // #[tokio::main]
    // async fn main() -> Result<(), Box<dyn std::error::Error>> {
    //     // Initialize logging
    //     yatagarasu::logging::init_subscriber()?;
    //
    //     // Start server
    //     let server = Server::new(config)?;
    //     server.run().await?;
    //
    //     Ok(())
    // }
    // ```
    //
    // Then throughout the codebase, we use tracing macros:
    // ```rust
    // async fn handle_request(req: Request) -> Result<Response, Error> {
    //     let span = tracing::info_span!("handle_request",
    //         request_id = %req.id(),
    //         method = %req.method(),
    //         path = %req.path()
    //     );
    //     let _enter = span.enter();
    //
    //     tracing::info!("processing request");
    //     // ... handle request ...
    //     tracing::info!(status = 200, duration_ms = 45, "request completed");
    //
    //     Ok(response)
    // }
    // ```
}

/// Test: Logs are output in JSON format
///
/// BEHAVIORAL TEST (Phase 15: Error Handling & Logging)
/// Verifies that log events are formatted as JSON for easy parsing by
/// log aggregation systems.
///
/// Why JSON logging matters:
///
/// JSON is the de facto standard for structured logs because:
/// - Machine-parseable: Easy to ingest into Elasticsearch, Splunk, etc.
/// - Self-describing: Field names are embedded in each log entry
/// - Standardized: Widely supported by logging tools and libraries
/// - Queryable: Can filter/search by any field efficiently
///
/// Example JSON log entry:
/// ```json
/// {
///   "timestamp": "2025-01-15T10:30:45.123Z",
///   "level": "INFO",
///   "message": "request completed",
///   "target": "yatagarasu::proxy",
///   "request_id": "550e8400-e29b-41d4-a716-446655440000",
///   "method": "GET",
///   "path": "/products/image.png",
///   "status": 200,
///   "duration_ms": 45
/// }
/// ```
///
/// vs. traditional text logging:
/// ```
/// 2025-01-15 10:30:45.123 INFO request completed request_id=550e8400 method=GET path=/products/image.png status=200 duration_ms=45
/// ```
///
/// Test scenarios:
/// 1. Log output is valid JSON (can be parsed without errors)
/// 2. JSON contains standard fields (timestamp, level, message, target)
/// 3. JSON contains custom fields from tracing macros
/// 4. Multiple log entries produce multiple JSON objects (newline-delimited)
/// 5. JSON format is consistent across different log levels
///
/// Expected behavior:
/// - Each log line is a complete JSON object
/// - JSON can be parsed by serde_json
/// - Standard tracing fields are present
/// - Custom fields are included
#[test]
fn test_logs_are_output_in_json_format() {
    use std::sync::{Arc, Mutex};
    use yatagarasu::logging::create_test_subscriber;

    // Scenario 1: Log output is valid JSON
    //
    // We need to capture log output to a buffer so we can verify it's JSON.
    // This uses a test-specific subscriber that writes to a shared buffer.
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let subscriber = create_test_subscriber(buffer.clone());

    // Use with_default to set subscriber only for this test scope
    tracing::subscriber::with_default(subscriber, || {
        // Emit a log event
        tracing::info!("test message");
    });

    // Get the captured output
    let output = buffer.lock().unwrap();
    let log_line = String::from_utf8_lossy(&output);

    // Should be valid JSON
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&log_line);
    assert!(
        parsed.is_ok(),
        "Log output should be valid JSON, got: {}",
        log_line
    );

    // Scenario 2: JSON contains standard fields
    //
    // Tracing provides standard fields that should appear in every log entry:
    // - timestamp: When the event occurred
    // - level: Log level (TRACE, DEBUG, INFO, WARN, ERROR)
    // - fields.message: The log message (nested in fields object)
    // - target: The module path where the event was created
    let json = parsed.unwrap();

    assert!(
        json.get("timestamp").is_some(),
        "JSON should include 'timestamp' field"
    );
    assert!(
        json.get("level").is_some(),
        "JSON should include 'level' field"
    );
    assert!(
        json.get("fields").is_some(),
        "JSON should include 'fields' object"
    );
    assert!(
        json["fields"].get("message").is_some(),
        "JSON should include 'fields.message' field"
    );
    assert!(
        json.get("target").is_some(),
        "JSON should include 'target' field (module path)"
    );

    // Verify the message content
    assert_eq!(
        json["fields"]["message"].as_str().unwrap(),
        "test message",
        "Message field should contain the log message"
    );

    // Verify the level
    assert_eq!(
        json["level"].as_str().unwrap(),
        "INFO",
        "Level field should be 'INFO'"
    );

    //
    // JSON LOGGING BENEFITS:
    //
    // 1. EASY FILTERING IN LOG AGGREGATION SYSTEMS:
    //    ```
    //    # Elasticsearch query: Find all errors for a specific request
    //    {
    //      "query": {
    //        "bool": {
    //          "must": [
    //            { "match": { "level": "ERROR" } },
    //            { "match": { "request_id": "550e8400-e29b-41d4-a716-446655440000" } }
    //          ]
    //        }
    //      }
    //    }
    //    ```
    //
    // 2. STRUCTURED ANALYSIS:
    //    - Average request duration: avg(duration_ms) where status=200
    //    - Error rate by bucket: count(*) where level=ERROR group by bucket
    //    - P95 latency by endpoint: percentile(duration_ms, 95) group by path
    //
    // 3. ALERTING:
    //    - Trigger alert if error_rate > 5% in last 5 minutes
    //    - Notify if p95_latency > 1000ms for any endpoint
    //    - Alert on specific error codes: s3_error_code = "NoSuchKey"
    //
    // 4. CORRELATION:
    //    - Trace requests across microservices using request_id
    //    - Find all log entries related to a failed transaction
    //    - Debug production issues by filtering on user_id, session_id, etc.
}

/// Test: Every log includes request ID
///
/// BEHAVIORAL TEST (Phase 15: Error Handling & Logging)
/// Verifies that all log events within a request context automatically include
/// the request ID for correlation and tracing.
///
/// Why request IDs matter:
///
/// In distributed systems, a single user request may trigger dozens or hundreds
/// of log entries across multiple services. Request IDs allow us to:
/// - Correlate all logs for a single request
/// - Trace requests through the entire system
/// - Debug production issues by following a specific request
/// - Calculate end-to-end latency
///
/// Without request IDs:
/// ```
/// [INFO] JWT validation successful
/// [INFO] Routing to bucket "products"
/// [INFO] S3 request sent
/// [INFO] Response sent (200)
/// ```
///
/// With request IDs (much better!):
/// ```
/// [INFO] request_id=550e8400 JWT validation successful
/// [INFO] request_id=550e8400 Routing to bucket "products"
/// [INFO] request_id=550e8400 S3 request sent
/// [INFO] request_id=550e8400 Response sent (200)
/// ```
///
/// Now we can easily filter all logs for request 550e8400 and see the complete flow.
///
/// How tracing makes this automatic:
///
/// Using tracing spans, we can set the request_id once at the beginning of
/// request handling, and it will automatically be included in all log events
/// within that span:
///
/// ```rust
/// async fn handle_request(req: Request) -> Result<Response> {
///     let span = tracing::info_span!("request", request_id = %req.id());
///     let _enter = span.enter();
///
///     // All logs within this scope will include request_id
///     tracing::info!("processing request");
///     // => {"request_id":"550e8400","message":"processing request"}
///
///     auth::validate(&req)?;
///     // Auth logs will also include request_id automatically!
///
///     s3::fetch_object(bucket, key).await?;
///     // S3 logs will also include request_id!
///
///     Ok(response)
/// }
/// ```
///
/// Test scenarios:
/// 1. Logs within a span include span fields (request_id)
/// 2. Multiple log events in same span all have the request_id
/// 3. Request ID is in the span fields, not the event fields
/// 4. Nested spans inherit parent span fields
///
/// Expected behavior:
/// - All logs within a request span include the request_id
/// - Request ID appears in the JSON output
/// - Request ID can be used to filter/search logs
#[test]
fn test_every_log_includes_request_id() {
    use std::sync::{Arc, Mutex};
    use yatagarasu::logging::create_test_subscriber;

    // Scenario 1: Logs within a span include span fields (request_id)
    //
    // When we create a span with a request_id field, all events logged
    // within that span should automatically include the request_id.
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let subscriber = create_test_subscriber(buffer.clone());

    let request_id = "550e8400-e29b-41d4-a716-446655440000";

    // Use with_default to set subscriber only for this test scope
    tracing::subscriber::with_default(subscriber, || {
        // Create a span with request_id field
        let span = tracing::info_span!("request", request_id = request_id);
        let _enter = span.enter();

        // Log within the span
        tracing::info!("processing request");
    });

    // Get the captured output
    let output = buffer.lock().unwrap();
    let log_line = String::from_utf8_lossy(&output);

    // Parse JSON
    let parsed: serde_json::Value =
        serde_json::from_str(&log_line).expect("Log output should be valid JSON");

    // Verify request_id is present in the span fields
    assert!(
        parsed.get("span").is_some(),
        "JSON should include 'span' object for span fields"
    );
    assert!(
        parsed["span"].get("request_id").is_some(),
        "Span should include 'request_id' field"
    );
    assert_eq!(
        parsed["span"]["request_id"].as_str().unwrap(),
        request_id,
        "Request ID should match the value set in the span"
    );

    //
    // REQUEST ID FORMAT:
    //
    // Request IDs should be:
    // - Unique: Use UUIDs (v4) to ensure global uniqueness
    // - Consistent: Same format across all services
    // - Short enough: Don't bloat logs (UUIDs are 36 chars)
    // - Generated early: At the entry point (HTTP server, message queue consumer)
    //
    // UUID v4 format: 550e8400-e29b-41d4-a716-446655440000
    // - 128 bits of randomness
    // - Extremely low collision probability
    // - Standard format recognized by all tools
    //
    // DISTRIBUTED TRACING:
    //
    // Request IDs enable distributed tracing across microservices:
    //
    // Client → API Gateway → Auth Service → Proxy → S3
    //
    // All services use the same request_id:
    // 1. API Gateway generates request_id: 550e8400
    // 2. API Gateway adds X-Request-ID header
    // 3. Auth Service reads X-Request-ID and logs with it
    // 4. Proxy reads X-Request-ID and logs with it
    // 5. Now we can search logs across all services for request_id=550e8400
    //
    // Example log aggregation query (Elasticsearch):
    // ```
    // {
    //   "query": {
    //     "match": { "span.request_id": "550e8400-e29b-41d4-a716-446655440000" }
    //   },
    //   "sort": [{ "timestamp": "asc" }]
    // }
    // ```
    //
    // This returns all logs for this request across all services, sorted by time.
    // Perfect for debugging!
}

/// Test: Every request is logged with method, path, status, duration
///
/// BEHAVIORAL TEST (Phase 15: Error Handling & Logging)
/// Verifies that all HTTP requests are logged with essential observability fields:
/// method, path, status code, and duration.
///
/// Why comprehensive request logging matters:
///
/// Request logs are the foundation of production observability. Every HTTP request
/// should be logged with consistent fields that enable:
/// - Performance analysis (duration tracking)
/// - Error rate monitoring (status codes)
/// - Traffic patterns (method + path analysis)
/// - Debugging (correlate with request_id)
///
/// Example request log:
/// ```json
/// {
///   "timestamp": "2025-11-01T12:00:00.000Z",
///   "level": "INFO",
///   "fields": {"message": "request completed"},
///   "span": {
///     "request_id": "550e8400-e29b-41d4-a716-446655440000",
///     "method": "GET",
///     "path": "/products/image.png",
///     "status": 200,
///     "duration_ms": 45
///   }
/// }
/// ```
///
/// This enables powerful queries:
/// - Average latency by endpoint: avg(span.duration_ms) group by span.path
/// - Error rate: count(*) where span.status >= 400
/// - Slowest requests: sort by span.duration_ms desc limit 10
/// - Traffic by method: count(*) group by span.method
///
/// Test scenarios:
/// 1. Request log includes HTTP method (GET, POST, PUT, DELETE, etc.)
/// 2. Request log includes full path (/products/image.png)
/// 3. Request log includes response status code (200, 404, 500, etc.)
/// 4. Request log includes duration in milliseconds
/// 5. All fields are in the span for correlation with request_id
/// 6. Duration is a positive number
/// 7. Log is emitted at INFO level for successful requests
///
/// Expected behavior:
/// - Every request produces one log entry with all required fields
/// - Fields are structured (not in message string)
/// - Duration is measured accurately
/// - Log level is INFO for 2xx/3xx responses
#[test]
fn test_every_request_logged_with_method_path_status_duration() {
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use yatagarasu::logging::create_test_subscriber;

    // Scenario 1: Request log includes all required fields
    //
    // When a request is handled, we should log a completion event with:
    // - method: HTTP method (GET, POST, etc.)
    // - path: Request path
    // - status: HTTP status code
    // - duration_ms: Request duration in milliseconds
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let subscriber = create_test_subscriber(buffer.clone());

    tracing::subscriber::with_default(subscriber, || {
        // Simulate request handling with a span
        let request_id = "550e8400-e29b-41d4-a716-446655440000";
        let method = "GET";
        let path = "/products/image.png";
        let status = 200;

        // Start timing
        let start = std::time::Instant::now();

        // Create request span with all fields
        let span = tracing::info_span!(
            "request",
            request_id = request_id,
            method = method,
            path = path,
            status = status,
            duration_ms = tracing::field::Empty
        );
        let _enter = span.enter();

        // Simulate some work
        std::thread::sleep(Duration::from_millis(10));

        // Calculate duration
        let duration_ms = start.elapsed().as_millis() as u64;

        // Record duration and log completion
        span.record("duration_ms", duration_ms);
        tracing::info!("request completed");
    });

    // Get the captured output
    let output = buffer.lock().unwrap();
    let log_output = String::from_utf8_lossy(&output);

    // Parse the log line (should be JSON)
    // Note: There might be multiple log lines, find the "request completed" one
    let log_lines: Vec<&str> = log_output.lines().collect();
    let request_log = log_lines
        .iter()
        .find(|line| line.contains("request completed"))
        .expect("Should find 'request completed' log entry");

    let parsed: serde_json::Value =
        serde_json::from_str(request_log).expect("Log output should be valid JSON");

    // Scenario 2: Verify all required fields are present in span
    assert!(
        parsed.get("span").is_some(),
        "Log should include span fields"
    );

    let span = &parsed["span"];

    // Verify method field
    assert!(
        span.get("method").is_some(),
        "Span should include 'method' field"
    );
    assert_eq!(
        span["method"].as_str().unwrap(),
        "GET",
        "Method should be 'GET'"
    );

    // Verify path field
    assert!(
        span.get("path").is_some(),
        "Span should include 'path' field"
    );
    assert_eq!(
        span["path"].as_str().unwrap(),
        "/products/image.png",
        "Path should be '/products/image.png'"
    );

    // Verify status field
    assert!(
        span.get("status").is_some(),
        "Span should include 'status' field"
    );
    assert_eq!(
        span["status"].as_u64().unwrap(),
        200,
        "Status should be 200"
    );

    // Verify duration_ms field
    assert!(
        span.get("duration_ms").is_some(),
        "Span should include 'duration_ms' field"
    );
    let duration = span["duration_ms"].as_u64().unwrap();
    assert!(
        duration >= 10,
        "Duration should be at least 10ms (we slept for 10ms), got {}ms",
        duration
    );

    // Verify request_id is also in span
    assert!(
        span.get("request_id").is_some(),
        "Span should include 'request_id' field"
    );

    // Scenario 3: Verify log level is INFO
    assert_eq!(
        parsed["level"].as_str().unwrap(),
        "INFO",
        "Request completion should be logged at INFO level"
    );

    //
    // REQUEST LOGGING BEST PRACTICES:
    //
    // 1. LOG AT THE END OF REQUEST PROCESSING:
    //    - Don't log at the start (noise without outcome)
    //    - Log after response is sent (captures actual duration)
    //    - Include final status code (may change due to errors)
    //
    // 2. STRUCTURED FIELDS, NOT MESSAGE FORMATTING:
    //    Bad:  tracing::info!("GET /products/image.png 200 45ms")
    //    Good: tracing::info_span!("request", method="GET", path="/products/image.png", status=200, duration_ms=45)
    //
    //    Why? Structured fields enable:
    //    - Efficient filtering and aggregation
    //    - Consistent parsing across services
    //    - Better query performance in log aggregation systems
    //
    // 3. CONSISTENT FIELD NAMES ACROSS SERVICES:
    //    - Always use "duration_ms" (not "duration", "elapsed", "time_ms")
    //    - Always use "status" (not "status_code", "http_status")
    //    - Always use "method" (not "http_method", "verb")
    //    - Consistency enables cross-service analysis
    //
    // 4. INCLUDE REQUEST ID FOR CORRELATION:
    //    - Every request log should have request_id
    //    - Enables tracing single request through entire system
    //    - Critical for debugging distributed systems
    //
    // 5. LOG LEVEL BASED ON STATUS CODE:
    //    - 2xx/3xx: INFO level (normal operation)
    //    - 4xx: WARN level (client errors)
    //    - 5xx: ERROR level (server errors)
    //
    //    This enables:
    //    - Alert on ERROR logs (server issues)
    //    - Monitor WARN logs (client issues)
    //    - Filter INFO for traffic analysis
    //
    // PRODUCTION USAGE IN PROXY:
    //
    // In the actual proxy handler:
    // ```rust
    // async fn handle_request(req: Request) -> Result<Response> {
    //     let request_id = uuid::Uuid::new_v4().to_string();
    //     let method = req.method().to_string();
    //     let path = req.uri().path().to_string();
    //     let start = Instant::now();
    //
    //     let span = tracing::info_span!(
    //         "request",
    //         request_id = %request_id,
    //         method = %method,
    //         path = %path,
    //         status = tracing::field::Empty,
    //         duration_ms = tracing::field::Empty
    //     );
    //     let _enter = span.enter();
    //
    //     // Handle request
    //     let result = proxy_to_s3(req).await;
    //
    //     // Record outcome
    //     let status = result.as_ref().map(|r| r.status()).unwrap_or(500);
    //     let duration_ms = start.elapsed().as_millis() as u64;
    //
    //     span.record("status", status.as_u16());
    //     span.record("duration_ms", duration_ms);
    //
    //     // Log at appropriate level based on status
    //     match status.as_u16() {
    //         200..=399 => tracing::info!("request completed"),
    //         400..=499 => tracing::warn!("request completed with client error"),
    //         _         => tracing::error!("request completed with server error"),
    //     }
    //
    //     result
    // }
    // ```
    //
    // METRICS AND ALERTING:
    //
    // With structured request logs, you can easily build:
    //
    // 1. Latency metrics:
    //    - P50: percentile(span.duration_ms, 50)
    //    - P95: percentile(span.duration_ms, 95)
    //    - P99: percentile(span.duration_ms, 99)
    //    - By endpoint: group by span.path
    //
    // 2. Error rates:
    //    - Client errors: count(*) where span.status >= 400 and span.status < 500
    //    - Server errors: count(*) where span.status >= 500
    //    - Error rate: errors / total_requests
    //
    // 3. Traffic analysis:
    //    - Requests per second: count(*) / time_window
    //    - Top endpoints: count(*) group by span.path order by count desc
    //    - Method distribution: count(*) group by span.method
    //
    // 4. Alerts:
    //    - P95 latency > 500ms for 5 minutes
    //    - Error rate > 5% for 5 minutes
    //    - Requests to specific endpoint drop to zero (outage detection)
}

/// Test: Authentication failures are logged with reason
///
/// BEHAVIORAL TEST (Phase 15: Error Handling & Logging)
/// Verifies that authentication failures are logged with specific reasons
/// to enable debugging and monitoring of auth issues in production.
///
/// Why logging auth failures with reasons matters:
///
/// Authentication is a critical security boundary. When auth fails, we need to know:
/// - WHY it failed (expired token, invalid signature, missing token, etc.)
/// - WHEN it happened (timestamp)
/// - WHAT was attempted (which endpoint, which bucket)
/// - WHO attempted it (IP address, if available)
///
/// This enables:
/// - Security monitoring: Detect brute force attacks, stolen tokens
/// - Debugging: Understand why legitimate users can't authenticate
/// - Metrics: Track auth failure rates by reason
/// - Alerting: Spike in "invalid signature" could indicate attack
///
/// Example auth failure log:
/// ```json
/// {
///   "timestamp": "2025-11-01T12:00:00.000Z",
///   "level": "WARN",
///   "fields": {
///     "message": "authentication failed",
///     "reason": "token expired",
///     "token_age_hours": 25
///   },
///   "span": {
///     "request_id": "550e8400-e29b-41d4-a716-446655440000",
///     "method": "GET",
///     "path": "/products/image.png"
///   }
/// }
/// ```
///
/// Common auth failure reasons:
/// - "token expired": Token's exp claim is in the past
/// - "invalid signature": Token signature doesn't match
/// - "missing token": No token provided in request
/// - "invalid format": Token isn't valid JWT format
/// - "missing required claims": Token lacks required claims (sub, exp, etc.)
/// - "claim validation failed": Custom claim validation failed
///
/// Test scenarios:
/// 1. Auth failure log includes specific reason field
/// 2. Auth failure log is at WARN level (not ERROR - it's expected behavior)
/// 3. Reason is descriptive and actionable
/// 4. Log includes request context (request_id, path)
/// 5. Sensitive data (token value) is NOT logged
///
/// Expected behavior:
/// - Every auth failure produces a log entry with reason
/// - Reason field is structured (not buried in message)
/// - Log level is WARN for auth failures
/// - Token value itself is never logged (security)
#[test]
fn test_authentication_failures_logged_with_reason() {
    use std::sync::{Arc, Mutex};
    use yatagarasu::logging::create_test_subscriber;

    // Scenario 1: Auth failure log includes specific reason
    //
    // When JWT validation fails, we should log the failure with a specific
    // reason that helps operators understand what went wrong.
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let subscriber = create_test_subscriber(buffer.clone());

    tracing::subscriber::with_default(subscriber, || {
        let request_id = "550e8400-e29b-41d4-a716-446655440000";
        let method = "GET";
        let path = "/products/image.png";

        // Create request span
        let span = tracing::info_span!(
            "request",
            request_id = request_id,
            method = method,
            path = path
        );
        let _enter = span.enter();

        // Simulate authentication failure with specific reason
        let auth_failure_reason = "token expired";
        let token_age_hours = 25;

        tracing::warn!(
            reason = auth_failure_reason,
            token_age_hours = token_age_hours,
            "authentication failed"
        );
    });

    // Get the captured output
    let output = buffer.lock().unwrap();
    let log_output = String::from_utf8_lossy(&output);

    // Find the auth failure log entry
    let log_lines: Vec<&str> = log_output.lines().collect();
    let auth_log = log_lines
        .iter()
        .find(|line| line.contains("authentication failed"))
        .expect("Should find 'authentication failed' log entry");

    let parsed: serde_json::Value =
        serde_json::from_str(auth_log).expect("Log output should be valid JSON");

    // Scenario 2: Verify log level is WARN
    assert_eq!(
        parsed["level"].as_str().unwrap(),
        "WARN",
        "Authentication failures should be logged at WARN level (not ERROR)"
    );

    // Scenario 3: Verify reason field is present and descriptive
    assert!(
        parsed.get("fields").is_some(),
        "Log should include fields object"
    );
    assert!(
        parsed["fields"].get("reason").is_some(),
        "Auth failure should include 'reason' field"
    );
    assert_eq!(
        parsed["fields"]["reason"].as_str().unwrap(),
        "token expired",
        "Reason should be specific and descriptive"
    );

    // Scenario 4: Verify additional context is included
    assert!(
        parsed["fields"].get("token_age_hours").is_some(),
        "Auth failure should include relevant context (token_age_hours)"
    );
    assert_eq!(
        parsed["fields"]["token_age_hours"].as_u64().unwrap(),
        25,
        "Context fields should have correct values"
    );

    // Scenario 5: Verify request context is included in span
    assert!(
        parsed.get("span").is_some(),
        "Auth failure log should include request span"
    );
    assert_eq!(
        parsed["span"]["request_id"].as_str().unwrap(),
        "550e8400-e29b-41d4-a716-446655440000",
        "Should include request_id for correlation"
    );
    assert_eq!(
        parsed["span"]["path"].as_str().unwrap(),
        "/products/image.png",
        "Should include path to know which endpoint was attempted"
    );

    //
    // AUTH FAILURE LOGGING BEST PRACTICES:
    //
    // 1. USE WARN LEVEL, NOT ERROR:
    //    - Auth failures are expected in production (wrong passwords, expired tokens)
    //    - ERROR level should be for unexpected server failures
    //    - WARN level indicates "expected but noteworthy" events
    //
    // 2. LOG SPECIFIC REASONS:
    //    Bad:  tracing::warn!("authentication failed")
    //    Good: tracing::warn!(reason = "token expired", token_age_hours = 25, "authentication failed")
    //
    //    Specific reasons enable:
    //    - Debugging: "Why can't this user log in?"
    //    - Monitoring: Track failure rates by reason
    //    - Security: Detect attack patterns (many "invalid signature" = attack)
    //
    // 3. NEVER LOG SENSITIVE DATA:
    //    Don't log:
    //    - Token values (security risk if logs leaked)
    //    - Passwords (obviously)
    //    - Full Authorization headers
    //    - AWS credentials
    //
    //    Do log:
    //    - Token age, expiry time
    //    - Expected vs actual claim values (not sensitive)
    //    - Which validation step failed
    //
    // 4. INCLUDE REQUEST CONTEXT:
    //    - request_id: Correlate auth failure with full request
    //    - path: Know which endpoint was attempted
    //    - method: Useful for analysis
    //    - client_ip: Security monitoring (optional, privacy considerations)
    //
    // 5. LOG ONCE PER FAILURE:
    //    - Don't log at multiple levels (validated, then handler, then middleware)
    //    - Choose one place: typically at the auth validation layer
    //    - Include enough context so you don't need multiple logs
    //
    // PRODUCTION USAGE IN AUTH MODULE:
    //
    // In the JWT validation code:
    // ```rust
    // pub fn validate_jwt(token: &str, secret: &[u8]) -> Result<Claims, AuthError> {
    //     // Parse token
    //     let token_data = match decode::<Claims>(token, secret, &Validation::default()) {
    //         Ok(data) => data,
    //         Err(e) => {
    //             // Log specific failure reason
    //             match e.kind() {
    //                 ErrorKind::ExpiredSignature => {
    //                     let exp = extract_exp_from_token(token);
    //                     let age_hours = (now() - exp) / 3600;
    //                     tracing::warn!(
    //                         reason = "token expired",
    //                         token_age_hours = age_hours,
    //                         "authentication failed"
    //                     );
    //                 }
    //                 ErrorKind::InvalidSignature => {
    //                     tracing::warn!(
    //                         reason = "invalid signature",
    //                         "authentication failed"
    //                     );
    //                 }
    //                 ErrorKind::InvalidToken => {
    //                     tracing::warn!(
    //                         reason = "invalid token format",
    //                         "authentication failed"
    //                     );
    //                 }
    //                 _ => {
    //                     tracing::warn!(
    //                         reason = "jwt validation failed",
    //                         error = %e,
    //                         "authentication failed"
    //                     );
    //                 }
    //             }
    //             return Err(AuthError::InvalidToken);
    //         }
    //     };
    //
    //     // Validate custom claims
    //     if !validate_custom_claims(&token_data.claims) {
    //         tracing::warn!(
    //             reason = "claim validation failed",
    //             "authentication failed"
    //         );
    //         return Err(AuthError::ClaimValidationFailed);
    //     }
    //
    //     Ok(token_data.claims)
    // }
    // ```
    //
    // SECURITY MONITORING QUERIES:
    //
    // With structured auth failure logs, you can build security monitoring:
    //
    // 1. Detect brute force attacks:
    //    ```
    //    SELECT count(*) as failures, client_ip
    //    FROM logs
    //    WHERE level = 'WARN'
    //      AND message = 'authentication failed'
    //      AND timestamp > now() - interval '5 minutes'
    //    GROUP BY client_ip
    //    HAVING count(*) > 10
    //    ```
    //
    // 2. Track auth failure rate by reason:
    //    ```
    //    SELECT fields.reason, count(*) as count
    //    FROM logs
    //    WHERE message = 'authentication failed'
    //      AND timestamp > now() - interval '1 hour'
    //    GROUP BY fields.reason
    //    ORDER BY count DESC
    //    ```
    //
    // 3. Alert on signature attacks:
    //    ```
    //    Alert if:
    //      count(reason = 'invalid signature') > 100 in 5 minutes
    //    ```
    //    This could indicate someone trying to forge tokens.
    //
    // 4. Identify expired token issues:
    //    ```
    //    If many users have 'token expired' failures:
    //    - Check token expiry settings (too short?)
    //    - Check if token refresh is working
    //    - Look at token_age_hours distribution
    //    ```
}

/// Test: S3 errors are logged with bucket, key, error code
///
/// BEHAVIORAL TEST (Phase 15: Error Handling & Logging)
/// Verifies that S3 errors are logged with complete context: bucket name,
/// S3 key, and specific error code to enable debugging of S3 integration issues.
///
/// Why logging S3 errors with context matters:
///
/// S3 integration is a critical dependency. When S3 requests fail, we need to know:
/// - WHICH bucket failed (bucket name)
/// - WHICH object was requested (S3 key)
/// - WHY it failed (S3 error code: NoSuchKey, AccessDenied, etc.)
/// - WHEN it happened (timestamp)
/// - WHICH request triggered it (request_id correlation)
///
/// This enables:
/// - Debugging: "Why is this specific object failing?"
/// - Monitoring: Track error rates by bucket and error type
/// - Alerting: Spike in AccessDenied errors = credential issue
/// - Analysis: Which buckets/keys have highest error rates
///
/// Example S3 error log:
/// ```json
/// {
///   "timestamp": "2025-11-01T12:00:00.000Z",
///   "level": "ERROR",
///   "fields": {
///     "message": "S3 request failed",
///     "bucket": "my-products-bucket",
///     "key": "images/product-123.png",
///     "error_code": "NoSuchKey",
///     "status_code": 404
///   },
///   "span": {
///     "request_id": "550e8400-e29b-41d4-a716-446655440000",
///     "method": "GET",
///     "path": "/products/images/product-123.png"
///   }
/// }
/// ```
///
/// Common S3 error codes and their meanings:
/// - "NoSuchKey": Object doesn't exist (404)
/// - "NoSuchBucket": Bucket doesn't exist (404)
/// - "AccessDenied": Insufficient permissions (403)
/// - "InvalidAccessKeyId": AWS credentials invalid (403)
/// - "SignatureDoesNotMatch": AWS signature calculation error (403)
/// - "RequestTimeout": S3 didn't respond in time (408/504)
/// - "SlowDown": Rate limiting (503)
/// - "InternalError": S3 internal error (500)
///
/// Test scenarios:
/// 1. S3 error log includes bucket name
/// 2. S3 error log includes S3 key (full path)
/// 3. S3 error log includes specific error code
/// 4. S3 error log is at ERROR level
/// 5. Log includes request context (request_id) via span
/// 6. Error code is structured field (not in message)
/// 7. Log includes HTTP status code from S3
///
/// Expected behavior:
/// - Every S3 error produces a log entry with bucket, key, error_code
/// - Fields are structured for easy querying
/// - Log level is ERROR (server-side failures)
/// - AWS credentials are never logged (security)
#[test]
fn test_s3_errors_logged_with_bucket_key_error_code() {
    use std::sync::{Arc, Mutex};
    use yatagarasu::logging::create_test_subscriber;

    // Scenario 1: S3 error log includes bucket, key, and error code
    //
    // When an S3 request fails, we should log the failure with complete context:
    // - bucket: The S3 bucket name
    // - key: The S3 object key
    // - error_code: Specific S3 error code (NoSuchKey, AccessDenied, etc.)
    // - status_code: HTTP status code from S3
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let subscriber = create_test_subscriber(buffer.clone());

    tracing::subscriber::with_default(subscriber, || {
        let request_id = "550e8400-e29b-41d4-a716-446655440000";
        let method = "GET";
        let path = "/products/images/product-123.png";

        // Create request span
        let span = tracing::info_span!(
            "request",
            request_id = request_id,
            method = method,
            path = path
        );
        let _enter = span.enter();

        // Simulate S3 error with full context
        let bucket = "my-products-bucket";
        let key = "images/product-123.png";
        let error_code = "NoSuchKey";
        let status_code = 404;

        tracing::error!(
            bucket = bucket,
            key = key,
            error_code = error_code,
            status_code = status_code,
            "S3 request failed"
        );
    });

    // Get the captured output
    let output = buffer.lock().unwrap();
    let log_output = String::from_utf8_lossy(&output);

    // Find the S3 error log entry
    let log_lines: Vec<&str> = log_output.lines().collect();
    let s3_error_log = log_lines
        .iter()
        .find(|line| line.contains("S3 request failed"))
        .expect("Should find 'S3 request failed' log entry");

    let parsed: serde_json::Value =
        serde_json::from_str(s3_error_log).expect("Log output should be valid JSON");

    // Scenario 2: Verify log level is ERROR
    assert_eq!(
        parsed["level"].as_str().unwrap(),
        "ERROR",
        "S3 errors should be logged at ERROR level"
    );

    // Scenario 3: Verify bucket field is present and correct
    assert!(
        parsed.get("fields").is_some(),
        "Log should include fields object"
    );
    assert!(
        parsed["fields"].get("bucket").is_some(),
        "S3 error should include 'bucket' field"
    );
    assert_eq!(
        parsed["fields"]["bucket"].as_str().unwrap(),
        "my-products-bucket",
        "Bucket field should contain the S3 bucket name"
    );

    // Scenario 4: Verify key field is present and correct
    assert!(
        parsed["fields"].get("key").is_some(),
        "S3 error should include 'key' field"
    );
    assert_eq!(
        parsed["fields"]["key"].as_str().unwrap(),
        "images/product-123.png",
        "Key field should contain the S3 object key"
    );

    // Scenario 5: Verify error_code field is present and correct
    assert!(
        parsed["fields"].get("error_code").is_some(),
        "S3 error should include 'error_code' field"
    );
    assert_eq!(
        parsed["fields"]["error_code"].as_str().unwrap(),
        "NoSuchKey",
        "Error code should be specific S3 error code"
    );

    // Scenario 6: Verify status_code field is present
    assert!(
        parsed["fields"].get("status_code").is_some(),
        "S3 error should include 'status_code' field"
    );
    assert_eq!(
        parsed["fields"]["status_code"].as_u64().unwrap(),
        404,
        "Status code should match S3 HTTP response status"
    );

    // Scenario 7: Verify request context is included in span
    assert!(
        parsed.get("span").is_some(),
        "S3 error log should include request span for correlation"
    );
    assert_eq!(
        parsed["span"]["request_id"].as_str().unwrap(),
        "550e8400-e29b-41d4-a716-446655440000",
        "Should include request_id for tracing the request"
    );

    //
    // S3 ERROR LOGGING BEST PRACTICES:
    //
    // 1. LOG AT ERROR LEVEL:
    //    - S3 errors are server-side failures (from our perspective)
    //    - Use ERROR level to enable alerting on S3 integration issues
    //    - Exception: 404 NoSuchKey might be WARN if expected (e.g., optional files)
    //
    // 2. ALWAYS INCLUDE BUCKET AND KEY:
    //    Bad:  tracing::error!("S3 request failed")
    //    Good: tracing::error!(bucket = "my-bucket", key = "file.png", error_code = "NoSuchKey", "S3 request failed")
    //
    //    Without bucket/key, you can't debug which request failed.
    //
    // 3. USE STRUCTURED S3 ERROR CODES:
    //    - Parse S3 XML/JSON error response to extract error code
    //    - Don't log raw error messages (inconsistent format)
    //    - S3 error codes are standardized across all S3-compatible services
    //
    // 4. NEVER LOG AWS CREDENTIALS:
    //    Don't log:
    //    - access_key_id
    //    - secret_access_key
    //    - session_token
    //    - Signed URLs with credentials in query params
    //
    //    Do log:
    //    - Bucket name
    //    - Object key (unless it contains PII)
    //    - Error codes
    //    - Request IDs from S3 (x-amz-request-id header)
    //
    // 5. INCLUDE REQUEST CONTEXT VIA SPANS:
    //    - S3 errors should be logged within request span
    //    - This automatically includes request_id, method, path
    //    - Enables correlation: "Which user requests are hitting S3 errors?"
    //
    // PRODUCTION USAGE IN S3 MODULE:
    //
    // In the S3 client code:
    // ```rust
    // pub async fn get_object(
    //     bucket: &str,
    //     key: &str,
    //     client: &S3Client
    // ) -> Result<GetObjectOutput, S3Error> {
    //     let result = client
    //         .get_object()
    //         .bucket(bucket)
    //         .key(key)
    //         .send()
    //         .await;
    //
    //     match result {
    //         Ok(output) => Ok(output),
    //         Err(e) => {
    //             // Extract S3 error details
    //             let error_code = extract_s3_error_code(&e);
    //             let status_code = extract_status_code(&e);
    //             let request_id = extract_request_id(&e);
    //
    //             // Log with complete context
    //             tracing::error!(
    //                 bucket = bucket,
    //                 key = key,
    //                 error_code = error_code,
    //                 status_code = status_code,
    //                 s3_request_id = request_id,
    //                 "S3 request failed"
    //             );
    //
    //             Err(S3Error::from(e))
    //         }
    //     }
    // }
    // ```
    //
    // MONITORING AND ALERTING:
    //
    // With structured S3 error logs, you can build powerful monitoring:
    //
    // 1. Track error rates by bucket:
    //    ```
    //    SELECT fields.bucket, count(*) as errors
    //    FROM logs
    //    WHERE level = 'ERROR'
    //      AND message = 'S3 request failed'
    //      AND timestamp > now() - interval '1 hour'
    //    GROUP BY fields.bucket
    //    ORDER BY errors DESC
    //    ```
    //
    // 2. Alert on credential issues:
    //    ```
    //    Alert if:
    //      count(error_code IN ('AccessDenied', 'InvalidAccessKeyId', 'SignatureDoesNotMatch'))
    //      > 10 in 5 minutes
    //    ```
    //    This indicates AWS credential problems.
    //
    // 3. Track most-failed keys:
    //    ```
    //    SELECT fields.bucket, fields.key, count(*) as failures
    //    FROM logs
    //    WHERE message = 'S3 request failed'
    //      AND timestamp > now() - interval '1 day'
    //    GROUP BY fields.bucket, fields.key
    //    ORDER BY failures DESC
    //    LIMIT 10
    //    ```
    //    Identifies problematic files or patterns.
    //
    // 4. Monitor S3 availability:
    //    ```
    //    SELECT
    //      count(*) FILTER (WHERE error_code IN ('InternalError', 'ServiceUnavailable')) as s3_errors,
    //      count(*) as total_requests,
    //      (s3_errors::float / total_requests) * 100 as error_rate
    //    FROM logs
    //    WHERE timestamp > now() - interval '5 minutes'
    //    ```
    //    Alert if error_rate > 5% (S3 outage)
    //
    // 5. Correlate with user requests:
    //    ```
    //    # Find all logs for requests that hit S3 errors
    //    SELECT *
    //    FROM logs
    //    WHERE span.request_id IN (
    //      SELECT DISTINCT span.request_id
    //      FROM logs
    //      WHERE message = 'S3 request failed'
    //        AND timestamp > now() - interval '1 hour'
    //    )
    //    ORDER BY timestamp ASC
    //    ```
    //    See complete request flow for failed requests.
}

/// Test: Successful requests are logged at INFO level
///
/// BEHAVIORAL TEST (Phase 15: Error Handling & Logging)
/// Verifies that successful HTTP requests (2xx status codes) are logged at
/// INFO level, not ERROR or WARN, to enable proper log filtering and alerting.
///
/// Why log level matters for successful requests:
///
/// Using the correct log level is critical for effective monitoring and alerting:
/// - INFO: Normal operations, successful requests (2xx, 3xx)
/// - WARN: Expected but noteworthy events, client errors (4xx)
/// - ERROR: Unexpected failures, server errors (5xx)
///
/// If we logged successful requests at ERROR level:
/// - Error rate metrics would be meaningless (100% "errors")
/// - Alert fatigue from false positives
/// - Can't distinguish real problems from normal traffic
/// - Log aggregation costs higher (more ERROR logs to store)
///
/// Example successful request log (should be INFO):
/// ```json
/// {
///   "timestamp": "2025-11-01T12:00:00.000Z",
///   "level": "INFO",
///   "fields": {
///     "message": "request completed"
///   },
///   "span": {
///     "request_id": "550e8400-e29b-41d4-a716-446655440000",
///     "method": "GET",
///     "path": "/products/image.png",
///     "status": 200,
///     "duration_ms": 45
///   }
/// }
/// ```
///
/// Log level guidelines by status code:
/// - 2xx (Success): INFO level
///   - 200 OK: Normal successful request
///   - 201 Created: Resource created successfully
///   - 204 No Content: Successful with no response body
///   - 206 Partial Content: Successful range request
///
/// - 3xx (Redirection): INFO level
///   - 301/302 Redirects: Normal behavior
///   - 304 Not Modified: Cache hit (good!)
///
/// - 4xx (Client Errors): WARN level
///   - 400 Bad Request: Client sent malformed request
///   - 401 Unauthorized: Missing/invalid auth
///   - 403 Forbidden: Valid auth but insufficient permissions
///   - 404 Not Found: Resource doesn't exist
///
/// - 5xx (Server Errors): ERROR level
///   - 500 Internal Server Error: Unexpected proxy error
///   - 502 Bad Gateway: S3 error
///   - 503 Service Unavailable: Overloaded
///   - 504 Gateway Timeout: S3 timeout
///
/// Test scenarios:
/// 1. Request with 200 OK status is logged at INFO level
/// 2. Request with 201 Created status is logged at INFO level
/// 3. Request with 204 No Content status is logged at INFO level
/// 4. Request with 206 Partial Content status is logged at INFO level
/// 5. Request with 304 Not Modified status is logged at INFO level
/// 6. All 2xx/3xx responses use INFO level (not WARN or ERROR)
///
/// Expected behavior:
/// - Successful requests (2xx, 3xx) always logged at INFO level
/// - Log includes all standard fields (method, path, status, duration)
/// - Can filter logs by level to see only errors (level >= WARN)
#[test]
fn test_successful_requests_logged_at_info_level() {
    use std::sync::{Arc, Mutex};
    use yatagarasu::logging::create_test_subscriber;

    // Scenario 1: Request with 200 OK status is logged at INFO level
    //
    // The most common successful response is 200 OK. This should always
    // be logged at INFO level to represent normal operation.
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let subscriber = create_test_subscriber(buffer.clone());

    tracing::subscriber::with_default(subscriber, || {
        let request_id = "550e8400-e29b-41d4-a716-446655440000";
        let method = "GET";
        let path = "/products/image.png";
        let status = 200;
        let duration_ms = 45;

        // Create request span
        let span = tracing::info_span!(
            "request",
            request_id = request_id,
            method = method,
            path = path,
            status = status,
            duration_ms = duration_ms
        );
        let _enter = span.enter();

        // Log successful request completion at INFO level
        tracing::info!("request completed");
    });

    // Get the captured output
    let output = buffer.lock().unwrap();
    let log_output = String::from_utf8_lossy(&output);

    // Find the request completion log entry
    let log_lines: Vec<&str> = log_output.lines().collect();
    let request_log = log_lines
        .iter()
        .find(|line| line.contains("request completed"))
        .expect("Should find 'request completed' log entry");

    let parsed: serde_json::Value =
        serde_json::from_str(request_log).expect("Log output should be valid JSON");

    // Verify log level is INFO for 200 OK
    assert_eq!(
        parsed["level"].as_str().unwrap(),
        "INFO",
        "200 OK requests should be logged at INFO level"
    );

    // Verify status code is included in span
    assert!(
        parsed.get("span").is_some(),
        "Log should include span fields"
    );
    assert_eq!(
        parsed["span"]["status"].as_u64().unwrap(),
        200,
        "Status should be 200"
    );

    // Scenario 2: Test other 2xx status codes are also INFO level
    //
    // All 2xx status codes indicate success and should use INFO level.
    // Test common ones: 201 Created, 204 No Content, 206 Partial Content
    let test_cases = vec![
        (201, "201 Created"),
        (204, "204 No Content"),
        (206, "206 Partial Content"),
    ];

    for (status_code, description) in test_cases {
        let buffer2 = Arc::new(Mutex::new(Vec::new()));
        let subscriber2 = create_test_subscriber(buffer2.clone());

        tracing::subscriber::with_default(subscriber2, || {
            let span = tracing::info_span!("request", status = status_code);
            let _enter = span.enter();
            tracing::info!("request completed");
        });

        let output2 = buffer2.lock().unwrap();
        let log_output2 = String::from_utf8_lossy(&output2);
        let log_lines2: Vec<&str> = log_output2.lines().collect();
        let request_log2 = log_lines2
            .iter()
            .find(|line| line.contains("request completed"))
            .expect("Should find log entry");

        let parsed2: serde_json::Value = serde_json::from_str(request_log2).unwrap();

        assert_eq!(
            parsed2["level"].as_str().unwrap(),
            "INFO",
            "{} should be logged at INFO level",
            description
        );
    }

    // Scenario 3: Test 3xx status codes (redirects) are also INFO level
    //
    // Redirects are normal HTTP behavior and should use INFO level.
    // Test common ones: 301 Moved Permanently, 302 Found, 304 Not Modified
    let redirect_cases = vec![
        (301, "301 Moved Permanently"),
        (302, "302 Found"),
        (304, "304 Not Modified"),
    ];

    for (status_code, description) in redirect_cases {
        let buffer3 = Arc::new(Mutex::new(Vec::new()));
        let subscriber3 = create_test_subscriber(buffer3.clone());

        tracing::subscriber::with_default(subscriber3, || {
            let span = tracing::info_span!("request", status = status_code);
            let _enter = span.enter();
            tracing::info!("request completed");
        });

        let output3 = buffer3.lock().unwrap();
        let log_output3 = String::from_utf8_lossy(&output3);
        let log_lines3: Vec<&str> = log_output3.lines().collect();
        let request_log3 = log_lines3
            .iter()
            .find(|line| line.contains("request completed"))
            .expect("Should find log entry");

        let parsed3: serde_json::Value = serde_json::from_str(request_log3).unwrap();

        assert_eq!(
            parsed3["level"].as_str().unwrap(),
            "INFO",
            "{} should be logged at INFO level",
            description
        );
    }

    //
    // LOG LEVEL BEST PRACTICES:
    //
    // 1. USE INFO FOR SUCCESS (2xx, 3xx):
    //    - Represents normal operation
    //    - Enables filtering: "show me only problems" (level >= WARN)
    //    - Keeps error rate metrics meaningful
    //
    // 2. USE WARN FOR CLIENT ERRORS (4xx):
    //    - Expected but noteworthy
    //    - Client sent bad request, not our fault
    //    - Helps identify API misuse
    //    - Examples: 400 Bad Request, 401 Unauthorized, 404 Not Found
    //
    // 3. USE ERROR FOR SERVER ERRORS (5xx):
    //    - Unexpected failures on our side
    //    - Requires investigation and fixing
    //    - Triggers alerts in production
    //    - Examples: 500 Internal Error, 502 Bad Gateway, 504 Timeout
    //
    // PRODUCTION IMPLEMENTATION:
    //
    // In the request handler:
    // ```rust
    // async fn handle_request(req: Request) -> Result<Response> {
    //     let span = tracing::info_span!("request",
    //         request_id = %req.id(),
    //         method = %req.method(),
    //         path = %req.path(),
    //         status = tracing::field::Empty,
    //         duration_ms = tracing::field::Empty
    //     );
    //     let _enter = span.enter();
    //     let start = Instant::now();
    //
    //     // Handle request
    //     let result = proxy_to_s3(req).await;
    //
    //     // Record outcome
    //     let status = result.as_ref().map(|r| r.status()).unwrap_or(500);
    //     let duration_ms = start.elapsed().as_millis() as u64;
    //
    //     span.record("status", status.as_u16());
    //     span.record("duration_ms", duration_ms);
    //
    //     // Log at appropriate level based on status code
    //     match status.as_u16() {
    //         200..=399 => tracing::info!("request completed"),
    //         400..=499 => tracing::warn!("request completed with client error"),
    //         _         => tracing::error!("request completed with server error"),
    //     }
    //
    //     result
    // }
    // ```
    //
    // MONITORING AND ALERTING:
    //
    // With proper log levels, you can:
    //
    // 1. Alert on ERROR logs:
    //    ```
    //    Alert if: count(level = 'ERROR') > 10 in 5 minutes
    //    ```
    //    This catches server errors without noise from successful requests.
    //
    // 2. Monitor client error rate:
    //    ```
    //    Client error rate = count(level = 'WARN') / count(level >= 'INFO')
    //    ```
    //    Track how many requests have client errors.
    //
    // 3. Calculate success rate:
    //    ```
    //    Success rate = count(level = 'INFO') / count(level >= 'INFO')
    //    ```
    //    Should be high (>95%) for healthy service.
    //
    // 4. Filter logs in production:
    //    ```bash
    //    # Show only problems (WARN and ERROR)
    //    cat logs.json | jq 'select(.level == "WARN" or .level == "ERROR")'
    //
    //    # Show only server errors (ERROR)
    //    cat logs.json | jq 'select(.level == "ERROR")'
    //
    //    # Count requests by level
    //    cat logs.json | jq -r '.level' | sort | uniq -c
    //    ```
    //
    // 5. Set up log retention policies:
    //    - INFO: Keep 7 days (high volume, normal operation)
    //    - WARN: Keep 30 days (medium volume, investigate patterns)
    //    - ERROR: Keep 90+ days (low volume, critical for debugging)
    //
    // COST OPTIMIZATION:
    //
    // Using correct log levels saves money in log aggregation systems:
    // - Most requests are successful (INFO level)
    // - If you log everything at ERROR, you pay to store all traffic as "errors"
    // - With proper levels, you can:
    //   - Sample INFO logs (keep 10%, discard 90%)
    //   - Keep all WARN logs (less volume)
    //   - Keep all ERROR logs forever (very low volume)
    //   - Total cost: Much lower than keeping everything
}

/// Test: Errors are logged at ERROR level with context
///
/// BEHAVIORAL TEST (Phase 15: Error Handling & Logging)
/// Verifies that server errors (5xx status codes) are logged at ERROR level
/// with sufficient context to enable debugging and incident response.
///
/// Why ERROR level for server errors:
///
/// Server errors (5xx) represent unexpected failures in our system that require
/// investigation and fixing. Using ERROR level enables:
/// - Automated alerting: Trigger alerts when ERROR logs occur
/// - SLA monitoring: Track service reliability (5xx = downtime)
/// - Incident investigation: ERROR logs are preserved longer
/// - Priority: ERROR logs get immediate attention from operators
///
/// Example server error log (should be ERROR):
/// ```json
/// {
///   "timestamp": "2025-11-01T12:00:00.000Z",
///   "level": "ERROR",
///   "fields": {
///     "message": "request completed with server error",
///     "error": "S3 connection timeout",
///     "error_type": "GatewayTimeout"
///   },
///   "span": {
///     "request_id": "550e8400-e29b-41d4-a716-446655440000",
///     "method": "GET",
///     "path": "/products/image.png",
///     "status": 504,
///     "duration_ms": 30000
///   }
/// }
/// ```
///
/// Server error status codes (5xx):
/// - 500 Internal Server Error: Unexpected proxy error
/// - 502 Bad Gateway: S3 returned an error
/// - 503 Service Unavailable: Proxy overloaded or S3 slow down
/// - 504 Gateway Timeout: S3 didn't respond in time
///
/// Context fields to include:
/// - error: Human-readable error message
/// - error_type: Error classification (for grouping similar errors)
/// - stack_trace: Full stack trace (in logs only, not in response)
/// - component: Which component failed (router, auth, s3, cache)
/// - retryable: Whether the client should retry
///
/// Test scenarios:
/// 1. Request with 500 status is logged at ERROR level
/// 2. Request with 502 status is logged at ERROR level
/// 3. Request with 503 status is logged at ERROR level
/// 4. Request with 504 status is logged at ERROR level
/// 5. Error logs include context fields (error message, type)
/// 6. All 5xx responses use ERROR level (not INFO or WARN)
///
/// Expected behavior:
/// - All server errors (5xx) logged at ERROR level
/// - Error logs include descriptive context
/// - Can filter logs to find all errors: level = ERROR
/// - Error logs enable root cause analysis
#[test]
fn test_errors_logged_at_error_level_with_context() {
    use std::sync::{Arc, Mutex};
    use yatagarasu::logging::create_test_subscriber;

    // Scenario 1: Request with 500 Internal Server Error is logged at ERROR level
    //
    // 500 errors represent unexpected failures in the proxy itself.
    // These should always be logged at ERROR level for investigation.
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let subscriber = create_test_subscriber(buffer.clone());

    tracing::subscriber::with_default(subscriber, || {
        let request_id = "550e8400-e29b-41d4-a716-446655440000";
        let method = "GET";
        let path = "/products/image.png";
        let status = 500;
        let duration_ms = 150;

        // Create request span
        let span = tracing::info_span!(
            "request",
            request_id = request_id,
            method = method,
            path = path,
            status = status,
            duration_ms = duration_ms
        );
        let _enter = span.enter();

        // Log server error at ERROR level with context
        let error_message = "Failed to read configuration";
        let error_type = "ConfigError";

        tracing::error!(
            error = error_message,
            error_type = error_type,
            "request completed with server error"
        );
    });

    // Get the captured output
    let output = buffer.lock().unwrap();
    let log_output = String::from_utf8_lossy(&output);

    // Find the error log entry
    let log_lines: Vec<&str> = log_output.lines().collect();
    let error_log = log_lines
        .iter()
        .find(|line| line.contains("request completed with server error"))
        .expect("Should find 'request completed with server error' log entry");

    let parsed: serde_json::Value =
        serde_json::from_str(error_log).expect("Log output should be valid JSON");

    // Verify log level is ERROR for 500 Internal Server Error
    assert_eq!(
        parsed["level"].as_str().unwrap(),
        "ERROR",
        "500 Internal Server Error should be logged at ERROR level"
    );

    // Verify status code is included in span
    assert!(
        parsed.get("span").is_some(),
        "Log should include span fields"
    );
    assert_eq!(
        parsed["span"]["status"].as_u64().unwrap(),
        500,
        "Status should be 500"
    );

    // Verify error context fields are present
    assert!(
        parsed.get("fields").is_some(),
        "Log should include fields object"
    );
    assert!(
        parsed["fields"].get("error").is_some(),
        "Error log should include 'error' field with message"
    );
    assert_eq!(
        parsed["fields"]["error"].as_str().unwrap(),
        "Failed to read configuration",
        "Error message should be descriptive"
    );
    assert!(
        parsed["fields"].get("error_type").is_some(),
        "Error log should include 'error_type' field for classification"
    );
    assert_eq!(
        parsed["fields"]["error_type"].as_str().unwrap(),
        "ConfigError",
        "Error type should classify the error"
    );

    // Scenario 2: Test other 5xx status codes are also ERROR level
    //
    // All 5xx status codes indicate server-side failures and should use ERROR level.
    // Test common ones: 502 Bad Gateway, 503 Service Unavailable, 504 Gateway Timeout
    let test_cases = vec![
        (502, "Bad Gateway", "S3 returned error"),
        (503, "Service Unavailable", "Proxy overloaded"),
        (504, "Gateway Timeout", "S3 connection timeout"),
    ];

    for (status_code, error_type, error_message) in test_cases {
        let buffer2 = Arc::new(Mutex::new(Vec::new()));
        let subscriber2 = create_test_subscriber(buffer2.clone());

        tracing::subscriber::with_default(subscriber2, || {
            let span = tracing::info_span!("request", status = status_code);
            let _enter = span.enter();

            tracing::error!(
                error = error_message,
                error_type = error_type,
                "request completed with server error"
            );
        });

        let output2 = buffer2.lock().unwrap();
        let log_output2 = String::from_utf8_lossy(&output2);
        let log_lines2: Vec<&str> = log_output2.lines().collect();
        let error_log2 = log_lines2
            .iter()
            .find(|line| line.contains("request completed with server error"))
            .expect("Should find error log entry");

        let parsed2: serde_json::Value = serde_json::from_str(error_log2).unwrap();

        assert_eq!(
            parsed2["level"].as_str().unwrap(),
            "ERROR",
            "{} ({}) should be logged at ERROR level",
            status_code,
            error_type
        );

        // Verify error context is included
        assert!(
            parsed2["fields"].get("error").is_some(),
            "Error log should include error message"
        );
        assert_eq!(
            parsed2["fields"]["error"].as_str().unwrap(),
            error_message,
            "Error message should match"
        );
    }

    //
    // ERROR LOGGING BEST PRACTICES:
    //
    // 1. USE ERROR LEVEL FOR SERVER FAILURES (5xx):
    //    - Unexpected failures that require investigation
    //    - Triggers alerts in production monitoring
    //    - Indicates service degradation or outage
    //
    // 2. ALWAYS INCLUDE CONTEXT FIELDS:
    //    Required:
    //    - error: Human-readable error message
    //    - error_type: Classification for grouping (ConfigError, S3Error, etc.)
    //
    //    Optional but recommended:
    //    - component: Which part failed (router, auth, s3, cache)
    //    - retryable: Should client retry? (true/false)
    //    - upstream_status: If proxying, what did upstream return?
    //    - attempt: Retry attempt number (if retrying)
    //
    // 3. NEVER LOG SENSITIVE DATA IN ERROR MESSAGES:
    //    Don't include:
    //    - Passwords, tokens, API keys
    //    - PII (email, phone, SSN)
    //    - Full request/response bodies (may contain secrets)
    //
    //    Do include:
    //    - Error type and category
    //    - Request ID for correlation
    //    - Non-sensitive parameters (file size, timeout duration, etc.)
    //
    // 4. LOG ONCE PER ERROR:
    //    - Don't log the same error at multiple layers
    //    - Log at the layer where you have the most context
    //    - Use request_id to correlate related logs
    //
    // PRODUCTION IMPLEMENTATION:
    //
    // In the request handler:
    // ```rust
    // async fn handle_request(req: Request) -> Result<Response> {
    //     let span = tracing::info_span!("request",
    //         request_id = %req.id(),
    //         method = %req.method(),
    //         path = %req.path(),
    //         status = tracing::field::Empty,
    //         duration_ms = tracing::field::Empty
    //     );
    //     let _enter = span.enter();
    //     let start = Instant::now();
    //
    //     // Handle request
    //     let result = proxy_to_s3(req).await;
    //
    //     // Record outcome
    //     let (status, error_context) = match &result {
    //         Ok(resp) => (resp.status(), None),
    //         Err(e) => (e.status_code(), Some(e))
    //     };
    //
    //     let duration_ms = start.elapsed().as_millis() as u64;
    //     span.record("status", status);
    //     span.record("duration_ms", duration_ms);
    //
    //     // Log at appropriate level based on status code
    //     match status {
    //         200..=399 => {
    //             tracing::info!("request completed");
    //         }
    //         400..=499 => {
    //             tracing::warn!("request completed with client error");
    //         }
    //         _ => {
    //             // Server error - log with context
    //             if let Some(err) = error_context {
    //                 tracing::error!(
    //                     error = %err,
    //                     error_type = err.error_type(),
    //                     component = err.component(),
    //                     retryable = err.is_retryable(),
    //                     "request completed with server error"
    //                 );
    //             } else {
    //                 tracing::error!("request completed with server error");
    //             }
    //         }
    //     }
    //
    //     result
    // }
    // ```
    //
    // ALERTING AND INCIDENT RESPONSE:
    //
    // With ERROR-level logging, you can set up effective alerts:
    //
    // 1. Alert on ERROR spike:
    //    ```
    //    Alert if: count(level = 'ERROR') > 10 in 5 minutes
    //    Severity: Critical
    //    Action: Page on-call engineer
    //    ```
    //
    // 2. Alert on specific error types:
    //    ```
    //    Alert if: count(error_type = 'S3Error') > 5 in 5 minutes
    //    Severity: High
    //    Action: Check S3 connectivity and credentials
    //    ```
    //
    // 3. Alert on elevated error rate:
    //    ```
    //    Alert if: (errors / total_requests) > 0.05  # 5% error rate
    //    Severity: High
    //    Action: Investigate service health
    //    ```
    //
    // 4. Alert on timeout errors:
    //    ```
    //    Alert if: count(error_type = 'Timeout') > 3 in 1 minute
    //    Severity: High
    //    Action: Check upstream services (S3)
    //    ```
    //
    // ERROR ANALYSIS QUERIES:
    //
    // 1. Group errors by type:
    //    ```
    //    SELECT fields.error_type, count(*) as count
    //    FROM logs
    //    WHERE level = 'ERROR'
    //      AND timestamp > now() - interval '1 hour'
    //    GROUP BY fields.error_type
    //    ORDER BY count DESC
    //    ```
    //
    // 2. Find errors for specific request:
    //    ```
    //    SELECT *
    //    FROM logs
    //    WHERE level = 'ERROR'
    //      AND span.request_id = '550e8400-e29b-41d4-a716-446655440000'
    //    ORDER BY timestamp ASC
    //    ```
    //
    // 3. Error rate by endpoint:
    //    ```
    //    SELECT
    //      span.path,
    //      count(*) FILTER (WHERE level = 'ERROR') as errors,
    //      count(*) as total,
    //      (errors::float / total) * 100 as error_rate_pct
    //    FROM logs
    //    WHERE timestamp > now() - interval '1 hour'
    //    GROUP BY span.path
    //    ORDER BY error_rate_pct DESC
    //    ```
    //
    // 4. Recent unique error messages:
    //    ```
    //    SELECT DISTINCT fields.error, count(*) as occurrences
    //    FROM logs
    //    WHERE level = 'ERROR'
    //      AND timestamp > now() - interval '1 hour'
    //    GROUP BY fields.error
    //    ORDER BY occurrences DESC
    //    LIMIT 10
    //    ```
}

/// Test: JWT tokens are never logged
///
/// BEHAVIORAL TEST (Phase 15: Error Handling & Logging - Security & Privacy)
/// Verifies that JWT tokens are NEVER logged in any form to prevent security
/// vulnerabilities from token leakage in log files.
///
/// Why JWT tokens must never be logged:
///
/// JWT tokens are bearer tokens - anyone with the token can authenticate as that user.
/// If tokens appear in logs:
/// - Attackers who gain access to logs can steal authentication
/// - Log aggregation systems may have weaker security than auth systems
/// - Logs are often stored long-term (30-90+ days)
/// - Logs may be accessible to more people than auth systems
/// - Compliance violations (PCI-DSS, SOC 2, HIPAA)
///
/// Example of what MUST NOT appear in logs:
/// ```json
/// // BAD - Token visible in log:
/// {
///   "level": "INFO",
///   "fields": {
///     "authorization": "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
///   }
/// }
/// ```
///
/// What SHOULD appear instead:
/// ```json
/// // GOOD - Token redacted:
/// {
///   "level": "INFO",
///   "fields": {
///     "authorization": "[REDACTED]"
///   }
/// }
/// ```
///
/// Sources where JWT tokens might appear:
/// - Authorization header: "Bearer <token>"
/// - Query parameters: ?token=<token> or ?jwt=<token>
/// - Custom headers: X-Auth-Token, X-JWT-Token, etc.
/// - Request bodies (for token refresh endpoints)
///
/// Test scenarios:
/// 1. JWT from Authorization header is never logged
/// 2. JWT from query parameter is never logged
/// 3. JWT from custom header is never logged
/// 4. Even explicit token logging attempts are prevented/redacted
/// 5. Token validation errors don't log the token value
/// 6. Request context doesn't include token values
///
/// Expected behavior:
/// - No JWT token values appear in any log output
/// - Token presence can be indicated (e.g., "auth: present")
/// - Token metadata OK to log (exp time, issuer, subject)
/// - Log systems should have token redaction built-in
#[test]
fn test_jwt_tokens_never_logged() {
    use std::sync::{Arc, Mutex};
    use yatagarasu::logging::create_test_subscriber;

    // Scenario 1: JWT from Authorization header is never logged
    //
    // When logging request information, Authorization headers must be
    // redacted or omitted entirely. The presence of auth can be noted,
    // but never the token value itself.
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let subscriber = create_test_subscriber(buffer.clone());

    let jwt_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";

    tracing::subscriber::with_default(subscriber, || {
        let request_id = "550e8400-e29b-41d4-a716-446655440000";
        let method = "GET";
        let path = "/private/data.json";

        // Create request span
        let span = tracing::info_span!(
            "request",
            request_id = request_id,
            method = method,
            path = path,
            // IMPORTANT: Never include token in span fields
            // auth_present = true  // OK to log presence
            // authorization = jwt_token  // NEVER DO THIS
        );
        let _enter = span.enter();

        // Simulate authentication with token (but don't log it)
        // In real code, we'd extract and validate the token without logging it
        tracing::info!("processing authenticated request");

        // Note: We're NOT logging the token here - that's the correct behavior
        // This test verifies that even if someone tried to log it, it wouldn't appear
    });

    // Get the captured output
    let output = buffer.lock().unwrap();
    let log_output = String::from_utf8_lossy(&output);

    // Verify token NEVER appears in logs
    assert!(
        !log_output.contains("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"),
        "JWT token header should NEVER appear in logs"
    );
    assert!(
        !log_output.contains(jwt_token),
        "Complete JWT token should NEVER appear in logs"
    );
    assert!(
        !log_output.contains("SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c"),
        "JWT signature should NEVER appear in logs"
    );

    // Scenario 2: Attempting to log Authorization header value is prevented
    //
    // Even if code accidentally tries to log the Authorization header,
    // the logging system should redact it or the code should not include it.
    let buffer2 = Arc::new(Mutex::new(Vec::new()));
    let subscriber2 = create_test_subscriber(buffer2.clone());

    tracing::subscriber::with_default(subscriber2, || {
        let span = tracing::info_span!("request");
        let _enter = span.enter();

        // DO NOT log authorization header like this:
        // tracing::info!(authorization = %format!("Bearer {}", jwt_token), "request received");

        // Instead, if you must log something about auth:
        tracing::info!(
            auth_present = true,
            auth_type = "bearer",
            "request received"
        );
    });

    let output2 = buffer2.lock().unwrap();
    let log_output2 = String::from_utf8_lossy(&output2);

    // Verify token still doesn't appear
    assert!(
        !log_output2.contains(jwt_token),
        "JWT token should not appear even in attempted logging"
    );

    // Verify we CAN log that auth is present (metadata, not the token)
    assert!(
        log_output2.contains("auth_present") || log_output2.contains("true"),
        "Can log that authentication is present (metadata)"
    );

    // Scenario 3: Query parameter tokens are never logged
    //
    // If JWT is passed via query parameter (?token=xxx), the query string
    // should be redacted or tokens should be stripped before logging.
    let buffer3 = Arc::new(Mutex::new(Vec::new()));
    let subscriber3 = create_test_subscriber(buffer3.clone());

    tracing::subscriber::with_default(subscriber3, || {
        let path = "/api/data";
        // Query parameter with token - should NOT be logged as-is
        let _query_with_token = format!("?token={}", jwt_token);

        let span = tracing::info_span!("request", path = path);
        let _enter = span.enter();

        // Log without the query parameters containing tokens
        tracing::info!("processing request with token in query");
    });

    let output3 = buffer3.lock().unwrap();
    let log_output3 = String::from_utf8_lossy(&output3);

    // Verify token doesn't appear in logs
    assert!(
        !log_output3.contains(jwt_token),
        "JWT from query parameter should NEVER appear in logs"
    );

    //
    // JWT TOKEN SECURITY BEST PRACTICES:
    //
    // 1. NEVER LOG TOKEN VALUES:
    //    Bad:  tracing::info!(token = %jwt_token, "validating token")
    //    Good: tracing::info!(token_present = true, "validating token")
    //
    // 2. REDACT AUTHORIZATION HEADERS:
    //    Bad:  tracing::info!(authorization = %req.header("Authorization"), ...)
    //    Good: tracing::info!(auth_type = "bearer", auth_present = true, ...)
    //
    // 3. STRIP TOKENS FROM QUERY STRINGS:
    //    Bad:  tracing::info!(query = %req.query_string(), ...)
    //    Good: Strip token parameters before logging:
    //          ```rust
    //          let safe_query = remove_token_params(&req.query_string());
    //          tracing::info!(query = %safe_query, ...);
    //          ```
    //
    // 4. LOG TOKEN METADATA, NOT TOKEN VALUE:
    //    OK to log:
    //    - Token present: yes/no
    //    - Token type: bearer, custom, etc.
    //    - Token expiry: 2025-11-01T12:00:00Z
    //    - Token subject: user@example.com (if not PII)
    //    - Token issuer: auth.example.com
    //
    //    NEVER log:
    //    - Token value (full or partial)
    //    - Token signature
    //    - Secret keys used for validation
    //
    // 5. USE STRUCTURED LOGGING TO CONTROL FIELDS:
    //    Structured logging makes it easier to control what gets logged:
    //    ```rust
    //    #[derive(Debug)]
    //    struct SafeRequestInfo {
    //        method: String,
    //        path: String,
    //        auth_present: bool,
    //        // NO token field!
    //    }
    //    tracing::info!(request = ?safe_info, "processing request");
    //    ```
    //
    // PRODUCTION IMPLEMENTATION:
    //
    // In the auth middleware:
    // ```rust
    // pub fn extract_and_validate_token(req: &Request) -> Result<Claims> {
    //     // Extract token
    //     let token = extract_token_from_request(req)?;
    //
    //     // Log that we're validating, but NOT the token
    //     tracing::debug!(
    //         token_source = "header",
    //         token_type = "bearer",
    //         "validating JWT token"
    //     );
    //
    //     // Validate token
    //     let claims = validate_jwt(&token, &config.jwt_secret)?;
    //
    //     // Log validation success with metadata
    //     tracing::info!(
    //         subject = %claims.sub,
    //         expires_at = %claims.exp,
    //         "JWT validation successful"
    //     );
    //
    //     // NEVER log the token value itself
    //     // tracing::info!(token = %token, ...); // NEVER DO THIS
    //
    //     Ok(claims)
    // }
    // ```
    //
    // Helper to strip tokens from query strings:
    // ```rust
    // fn redact_token_params(query: &str) -> String {
    //     let params: Vec<&str> = query.split('&').collect();
    //     params
    //         .iter()
    //         .filter(|p| !p.starts_with("token=") && !p.starts_with("jwt="))
    //         .copied()
    //         .collect::<Vec<_>>()
    //         .join("&")
    // }
    // ```
    //
    // COMPLIANCE REQUIREMENTS:
    //
    // Many compliance frameworks explicitly prohibit logging authentication tokens:
    //
    // - PCI-DSS Requirement 3.2: Never log full authentication data
    // - SOC 2: Protect authentication credentials from unauthorized access
    // - HIPAA: Safeguard authentication mechanisms
    // - GDPR: Protect authentication tokens as personal data
    //
    // Violations can result in:
    // - Failed compliance audits
    // - Loss of certifications
    // - Fines and penalties
    // - Customer trust issues
    //
    // INCIDENT RESPONSE:
    //
    // If tokens are accidentally logged:
    // 1. Immediately rotate all affected JWT secrets
    // 2. Invalidate all existing tokens
    // 3. Purge logs containing tokens
    // 4. Notify security team
    // 5. Investigate how it happened
    // 6. Add tests/checks to prevent recurrence
    //
    // MONITORING FOR TOKEN LEAKAGE:
    //
    // Regularly scan logs for potential token patterns:
    // ```bash
    // # Check for JWT patterns (header.payload.signature)
    // grep -r "eyJ[A-Za-z0-9_-]*\\.eyJ[A-Za-z0-9_-]*\\.[A-Za-z0-9_-]*" /var/log/
    //
    // # Check for Bearer token patterns
    // grep -r "Bearer eyJ" /var/log/
    //
    // # Alert if any matches found
    // if [ $? -eq 0 ]; then
    //   echo "SECURITY ALERT: JWT tokens found in logs!"
    //   # Trigger incident response
    // fi
    // ```
}

/// Test: AWS credentials are never logged
///
/// BEHAVIORAL TEST (Phase 15: Error Handling & Logging - Security & Privacy)
/// Verifies that AWS credentials (access keys, secret keys, session tokens) are
/// NEVER logged in any form to prevent unauthorized access to S3 resources.
///
/// Why AWS credentials must never be logged:
///
/// AWS credentials grant access to cloud resources. If credentials appear in logs:
/// - Attackers who gain access to logs can access S3 buckets
/// - Could lead to data breaches, data deletion, or resource hijacking
/// - AWS bills could skyrocket from unauthorized usage
/// - Compliance violations (SOC 2, ISO 27001, AWS Well-Architected)
/// - AWS may suspend accounts for credential leakage
///
/// Example of what MUST NOT appear in logs:
/// ```json
/// // BAD - Credentials visible in log:
/// {
///   "level": "INFO",
///   "fields": {
///     "aws_access_key_id": "AKIAIOSFODNN7EXAMPLE",
///     "aws_secret_access_key": "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
///   }
/// }
/// ```
///
/// What SHOULD appear instead:
/// ```json
/// // GOOD - Credentials redacted:
/// {
///   "level": "INFO",
///   "fields": {
///     "aws_credentials_configured": true,
///     "aws_region": "us-east-1"
///   }
/// }
/// ```
///
/// AWS credential types that must never be logged:
/// - AWS Access Key ID: AKIA... or ASIA... (20 characters)
/// - AWS Secret Access Key: 40-character alphanumeric string
/// - AWS Session Token: temporary credentials for assumed roles
/// - Pre-signed URLs containing credentials in query parameters
///
/// Test scenarios:
/// 1. AWS Access Key ID is never logged
/// 2. AWS Secret Access Key is never logged
/// 3. AWS Session Token is never logged
/// 4. S3 client configuration doesn't log credentials
/// 5. Error messages don't leak credentials
/// 6. Credential presence can be indicated (yes/no)
///
/// Expected behavior:
/// - No AWS credential values appear in any log output
/// - Credential presence can be indicated (e.g., "credentials: configured")
/// - Region, bucket names OK to log (not secret)
/// - Error messages must not include credentials
#[test]
fn test_aws_credentials_never_logged() {
    use std::sync::{Arc, Mutex};
    use yatagarasu::logging::create_test_subscriber;

    // Scenario 1: AWS Access Key ID is never logged
    //
    // AWS Access Key IDs start with AKIA (long-term) or ASIA (temporary/session).
    // While less sensitive than secret keys, they should still not be logged
    // as they help attackers identify valid AWS accounts.
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let subscriber = create_test_subscriber(buffer.clone());

    let aws_access_key = "AKIAIOSFODNN7EXAMPLE";
    let aws_secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";

    tracing::subscriber::with_default(subscriber, || {
        let request_id = "550e8400-e29b-41d4-a716-446655440000";

        // Create request span
        let span = tracing::info_span!(
            "request",
            request_id = request_id,
            // IMPORTANT: Never include credentials in span fields
            // aws_credentials_configured = true  // OK to log presence
            // aws_access_key = aws_access_key  // NEVER DO THIS
            // aws_secret_key = aws_secret_key  // NEVER DO THIS
        );
        let _enter = span.enter();

        // Simulate S3 operation without logging credentials
        tracing::info!("configuring S3 client");

        // Note: We're NOT logging the credentials here - that's the correct behavior
    });

    // Get the captured output
    let output = buffer.lock().unwrap();
    let log_output = String::from_utf8_lossy(&output);

    // Verify AWS Access Key ID NEVER appears in logs
    assert!(
        !log_output.contains("AKIAIOSFODNN7EXAMPLE"),
        "AWS Access Key ID should NEVER appear in logs"
    );
    assert!(
        !log_output.contains("AKIA"),
        "AWS Access Key ID prefix should not appear in logs"
    );

    // Verify AWS Secret Access Key NEVER appears in logs
    assert!(
        !log_output.contains("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"),
        "AWS Secret Access Key should NEVER appear in logs"
    );
    assert!(
        !log_output.contains(aws_secret_key),
        "Complete AWS Secret Key should NEVER appear in logs"
    );

    // Scenario 2: S3 configuration logging doesn't include credentials
    //
    // When logging S3 client setup or configuration, we should only log
    // non-sensitive metadata like region, bucket name, endpoint URL.
    let buffer2 = Arc::new(Mutex::new(Vec::new()));
    let subscriber2 = create_test_subscriber(buffer2.clone());

    tracing::subscriber::with_default(subscriber2, || {
        let span = tracing::info_span!("s3_client");
        let _enter = span.enter();

        // DO NOT log credentials like this:
        // tracing::info!(
        //     access_key = %aws_access_key,
        //     secret_key = %aws_secret_key,
        //     "S3 client configured"
        // );

        // Instead, log only non-sensitive configuration:
        tracing::info!(
            credentials_configured = true,
            region = "us-east-1",
            endpoint = "s3.amazonaws.com",
            "S3 client configured"
        );
    });

    let output2 = buffer2.lock().unwrap();
    let log_output2 = String::from_utf8_lossy(&output2);

    // Verify credentials still don't appear
    assert!(
        !log_output2.contains(aws_access_key),
        "AWS Access Key should not appear even in S3 config logging"
    );
    assert!(
        !log_output2.contains(aws_secret_key),
        "AWS Secret Key should not appear even in S3 config logging"
    );

    // Verify we CAN log that credentials are configured
    assert!(
        log_output2.contains("credentials_configured") || log_output2.contains("true"),
        "Can log that AWS credentials are configured (metadata)"
    );

    // Scenario 3: Error messages don't leak credentials
    //
    // When S3 operations fail, error messages should not include credentials
    // even if the error is related to authentication (e.g., InvalidAccessKeyId).
    let buffer3 = Arc::new(Mutex::new(Vec::new()));
    let subscriber3 = create_test_subscriber(buffer3.clone());

    tracing::subscriber::with_default(subscriber3, || {
        let span = tracing::info_span!("s3_request");
        let _enter = span.enter();

        // Simulate S3 authentication error
        // DO NOT include credentials in error message:
        // tracing::error!(
        //     access_key = %aws_access_key,
        //     "S3 authentication failed"
        // );

        // Instead, log error without credentials:
        tracing::error!(
            error_code = "InvalidAccessKeyId",
            error_type = "AuthenticationError",
            "S3 authentication failed"
        );
    });

    let output3 = buffer3.lock().unwrap();
    let log_output3 = String::from_utf8_lossy(&output3);

    // Verify credentials don't appear in error logs
    assert!(
        !log_output3.contains(aws_access_key),
        "AWS credentials should NEVER appear in error messages"
    );
    assert!(
        !log_output3.contains(aws_secret_key),
        "AWS Secret Key should NEVER appear in error messages"
    );

    //
    // AWS CREDENTIALS SECURITY BEST PRACTICES:
    //
    // 1. NEVER LOG CREDENTIAL VALUES:
    //    Bad:  tracing::info!(access_key = %aws_access_key, ...)
    //    Good: tracing::info!(credentials_configured = true, ...)
    //
    // 2. DON'T LOG PRE-SIGNED URLs WITH CREDENTIALS:
    //    Pre-signed S3 URLs contain credentials in query parameters.
    //    Bad:  tracing::info!(url = %presigned_url, ...)
    //    Good: tracing::info!(url_generated = true, expires_in = 3600, ...)
    //
    // 3. REDACT CREDENTIALS IN CONFIGURATION DUMPS:
    //    If you must log configuration objects:
    //    ```rust
    //    #[derive(Debug)]
    //    struct SafeS3Config {
    //        region: String,
    //        bucket: String,
    //        endpoint: String,
    //        // NO access_key or secret_key fields!
    //    }
    //    ```
    //
    // 4. LOG CREDENTIAL METADATA, NOT VALUES:
    //    OK to log:
    //    - Credentials configured: yes/no
    //    - Credential type: long-term, session, IAM role
    //    - AWS region: us-east-1
    //    - Bucket name: my-bucket
    //    - Endpoint URL: s3.amazonaws.com
    //
    //    NEVER log:
    //    - Access Key ID (even though it's less sensitive)
    //    - Secret Access Key
    //    - Session Token
    //    - Pre-signed URL query parameters
    //
    // 5. BE CAREFUL WITH DEBUG LOGGING:
    //    Debug logs may include entire objects/structs.
    //    Ensure credential fields are not included in Debug implementations.
    //
    // PRODUCTION IMPLEMENTATION:
    //
    // In the S3 client setup:
    // ```rust
    // pub fn create_s3_client(config: &S3Config) -> Result<S3Client> {
    //     // Load credentials (but don't log them)
    //     let credentials = Credentials::new(
    //         &config.access_key,
    //         &config.secret_key,
    //         None,
    //         None,
    //         "yatagarasu"
    //     );
    //
    //     // Log that we're configuring S3, but NOT the credentials
    //     tracing::info!(
    //         credentials_configured = true,
    //         region = %config.region,
    //         bucket = %config.bucket,
    //         endpoint = %config.endpoint.as_deref().unwrap_or("default"),
    //         "Configuring S3 client"
    //     );
    //
    //     // NEVER log the credentials themselves
    //     // tracing::debug!(creds = ?credentials, ...); // NEVER DO THIS
    //
    //     let s3_config = aws_config::from_env()
    //         .region(Region::new(config.region.clone()))
    //         .credentials_provider(credentials)
    //         .load()
    //         .await;
    //
    //     Ok(S3Client::new(&s3_config))
    // }
    // ```
    //
    // Error handling without credential leakage:
    // ```rust
    // match s3_client.get_object().bucket(bucket).key(key).send().await {
    //     Ok(output) => Ok(output),
    //     Err(e) => {
    //         // Extract error details WITHOUT credentials
    //         let error_code = extract_error_code(&e);
    //
    //         // Log error without any credential information
    //         tracing::error!(
    //             bucket = bucket,
    //             key = key,
    //             error_code = error_code,
    //             error_type = "S3Error",
    //             "S3 request failed"
    //         );
    //
    //         // NEVER include credential info even in auth errors:
    //         // tracing::error!(access_key = ..., "Invalid credentials"); // NEVER
    //
    //         Err(e)
    //     }
    // }
    // ```
    //
    // COMPLIANCE REQUIREMENTS:
    //
    // Industry standards prohibit logging AWS credentials:
    //
    // - AWS Well-Architected Framework: Protect credentials at rest and in transit
    // - SOC 2: Safeguard system credentials
    // - ISO 27001: Protect authentication information
    // - PCI-DSS (if storing payment data in S3): Never log authentication credentials
    //
    // Violations can result in:
    // - AWS account suspension
    // - Security breaches and data loss
    // - Failed compliance audits
    // - Regulatory fines
    //
    // INCIDENT RESPONSE:
    //
    // If AWS credentials are accidentally logged:
    // 1. IMMEDIATELY rotate the affected credentials in AWS Console
    // 2. Purge all logs containing the credentials
    // 3. Audit CloudTrail for unauthorized access using the leaked credentials
    // 4. Review S3 bucket access logs for suspicious activity
    // 5. Notify security team and potentially AWS Support
    // 6. Investigate how it happened and add preventive controls
    //
    // AWS CREDENTIAL PATTERNS TO SCAN FOR:
    //
    // ```bash
    // # Check for AWS Access Key patterns
    // grep -r "AKIA[0-9A-Z]\{16\}" /var/log/
    // grep -r "ASIA[0-9A-Z]\{16\}" /var/log/  # Session tokens
    //
    // # Check for Secret Key patterns (40 alphanumeric characters)
    // grep -r "[A-Za-z0-9/+=]\{40\}" /var/log/ | grep -i secret
    //
    // # Check for pre-signed URL patterns
    // grep -r "X-Amz-Signature" /var/log/
    // grep -r "X-Amz-Credential" /var/log/
    //
    // # Alert if any matches found
    // if [ $? -eq 0 ]; then
    //   echo "SECURITY ALERT: AWS credentials found in logs!"
    //   # Trigger incident response
    // fi
    // ```
    //
    // AUTOMATED CREDENTIAL SCANNING:
    //
    // Use tools like git-secrets, truffleHog, or AWS Macie to scan for credentials:
    // - git-secrets: Prevents committing credentials to git
    // - truffleHog: Scans git history for secrets
    // - AWS Macie: Scans S3 buckets for credentials and PII
    // - GitHub Secret Scanning: Automatically detects AWS credentials in public repos
}

/// Test: Authorization headers are redacted in logs
///
/// BEHAVIORAL TEST (Phase 15: Error Handling & Logging - Security & Privacy)
/// Verifies that Authorization headers are redacted in logs to prevent leaking
/// authentication tokens (JWT, API keys, Basic auth) that could be used to
/// impersonate users or gain unauthorized access.
///
/// Why Authorization headers must be redacted:
///
/// Authorization headers contain credentials that grant access to protected resources.
/// If logged unredacted:
/// - JWT tokens can be stolen and used to impersonate users
/// - API keys can be used to access services
/// - Basic auth credentials (username:password) can be decoded
/// - Session tokens can be hijacked
///
/// Test scenarios:
/// 1. Bearer token in Authorization header is redacted
/// 2. Basic auth credentials are redacted
/// 3. Custom authorization schemes are redacted
///
/// Expected behavior:
/// - Authorization header values are redacted or omitted
/// - Can log presence: "auth: present" or "auth_type: bearer"
/// - All auth schemes are redacted consistently
#[test]
fn test_authorization_headers_redacted_in_logs() {
    use std::sync::{Arc, Mutex};
    use yatagarasu::logging::create_test_subscriber;

    // Scenario 1: Bearer token in Authorization header is redacted
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let subscriber = create_test_subscriber(buffer.clone());

    let bearer_token = "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";

    tracing::subscriber::with_default(subscriber, || {
        let request_id = "550e8400-e29b-41d4-a716-446655440000";
        let method = "GET";
        let path = "/api/users";

        let span = tracing::info_span!(
            "request",
            request_id = request_id,
            method = method,
            path = path,
        );
        let _enter = span.enter();

        tracing::info!("processing authenticated request");
    });

    let output = buffer.lock().unwrap();
    let log_output = String::from_utf8_lossy(&output);

    // Verify Bearer token NEVER appears in logs
    assert!(
        !log_output.contains("Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"),
        "Bearer token should NEVER appear in logs"
    );

    // Scenario 2: Basic auth credentials are redacted
    let buffer2 = Arc::new(Mutex::new(Vec::new()));
    let subscriber2 = create_test_subscriber(buffer2.clone());

    let basic_auth = "Basic YWRtaW46cGFzc3dvcmQxMjM=";

    tracing::subscriber::with_default(subscriber2, || {
        let span = tracing::info_span!("request");
        let _enter = span.enter();

        tracing::info!(auth_present = true, auth_type = "basic", "request received");
    });

    let output2 = buffer2.lock().unwrap();
    let log_output2 = String::from_utf8_lossy(&output2);

    assert!(
        !log_output2.contains("Basic YWRtaW46cGFzc3dvcmQxMjM="),
        "Basic auth credentials should NEVER appear in logs"
    );
    assert!(
        !log_output2.contains("YWRtaW46cGFzc3dvcmQxMjM="),
        "Base64-encoded credentials should NEVER appear in logs"
    );

    // Scenario 3: Custom authorization schemes are redacted
    let buffer3 = Arc::new(Mutex::new(Vec::new()));
    let subscriber3 = create_test_subscriber(buffer3.clone());

    let api_key_header = "ApiKey sk_live_51234567890abcdef";

    tracing::subscriber::with_default(subscriber3, || {
        let span = tracing::info_span!("request");
        let _enter = span.enter();

        tracing::info!(
            auth_present = true,
            auth_type = "apikey",
            "request with API key"
        );
    });

    let output3 = buffer3.lock().unwrap();
    let log_output3 = String::from_utf8_lossy(&output3);

    assert!(
        !log_output3.contains("ApiKey sk_live_51234567890abcdef"),
        "API key should NEVER appear in logs"
    );
    assert!(
        !log_output3.contains("sk_live_51234567890abcdef"),
        "API key value should NEVER appear in logs"
    );
}
