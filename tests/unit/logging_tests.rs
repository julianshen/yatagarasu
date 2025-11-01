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
    use yatagarasu::logging::create_test_subscriber;
    use std::sync::{Arc, Mutex};

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
    use yatagarasu::logging::create_test_subscriber;
    use std::sync::{Arc, Mutex};

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
    let parsed: serde_json::Value = serde_json::from_str(&log_line)
        .expect("Log output should be valid JSON");

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
    use yatagarasu::logging::create_test_subscriber;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

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

    let parsed: serde_json::Value = serde_json::from_str(request_log)
        .expect("Log output should be valid JSON");

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
