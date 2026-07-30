//! Shared HTTP helpers for the Spotify and Apple Music API clients:
//! exponential back-off on rate limiting, and URL-component encoding.

use crate::error::AppError;
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use reqwest::{RequestBuilder, Response, StatusCode};

const QUERY_ENCODE_SET: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

/// Percent-encode a string for use in a URL query parameter or
/// `application/x-www-form-urlencoded` body value.
pub(crate) fn url_encode(s: &str) -> String {
    utf8_percent_encode(s, QUERY_ENCODE_SET).to_string()
}

pub(crate) fn parse_retry_after(resp: &Response) -> u64 {
    resp.headers()
        .get("retry-after")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(1)
}

/// Execute a request builder with exponential back-off on 429, respecting
/// the `Retry-After` header. `on_error` maps a non-2xx response body into
/// the caller's own error, so each provider can keep parsing its own
/// error body shape.
pub(crate) async fn request_with_backoff(
    provider: &'static str,
    build: impl Fn() -> RequestBuilder,
    on_error: impl Fn(StatusCode, String) -> AppError,
) -> Result<Response, AppError> {
    let max_retries: u32 = 5;
    let mut attempt = 0u32;

    loop {
        let resp = build().send().await?;

        if resp.status() == StatusCode::TOO_MANY_REQUESTS {
            attempt += 1;
            if attempt > max_retries {
                let retry_after = parse_retry_after(&resp);
                return Err(AppError::RateLimited {
                    retry_after_secs: retry_after,
                });
            }

            let server_wait = parse_retry_after(&resp);
            let backoff = server_wait.max(1) * 2u64.pow(attempt - 1);
            tracing::warn!(
                provider,
                attempt,
                backoff,
                "Rate limited (429), backing off"
            );
            tokio::time::sleep(std::time::Duration::from_secs(backoff)).await;
            continue;
        }

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(on_error(status, text));
        }

        return Ok(resp);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_encode_leaves_unreserved_characters_untouched() {
        assert_eq!(url_encode("abc-XYZ_123.~"), "abc-XYZ_123.~");
    }

    #[test]
    fn url_encode_escapes_reserved_and_whitespace() {
        assert_eq!(url_encode("a b"), "a%20b");
        assert_eq!(url_encode("a&b=c"), "a%26b%3Dc");
        assert_eq!(url_encode("100%"), "100%25");
    }

    #[test]
    fn url_encode_escapes_unicode() {
        // Sigur Rós — the ó needs escaping; a hand-rolled ASCII-only
        // encoder would silently pass it through unescaped.
        assert_eq!(url_encode("Sigur Rós"), "Sigur%20R%C3%B3s");
    }
}
