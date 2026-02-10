// Rate limiting is configured inline in main.rs using tower_governor.
// This module serves as documentation for the rate limit configuration.
//
// Configuration: `rate_limit.login_requests_per_minute` in ServerConfig (default: 5)
// Uses SmartIpKeyExtractor for per-IP rate limiting.
