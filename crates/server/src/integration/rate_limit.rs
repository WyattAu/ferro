use std::time::Duration;

use ferro_rate_limiter::TokenBucketLimiter;

pub fn create_ip_limiter(max_tokens: u32, refill_rate: u32) -> TokenBucketLimiter {
    TokenBucketLimiter::new(
        max_tokens,
        refill_rate,
        Duration::from_secs(1),
    )
}

#[cfg(test)]
mod tests {
    use ferro_rate_limiter::RateLimiter;

    use super::*;

    #[tokio::test]
    async fn test_limiter_allows_within_capacity() {
        let limiter = create_ip_limiter(5, 1);
        for i in 0..5 {
            let result = limiter.check(&format!("ip-{}", i)).await.unwrap();
            assert!(result.allowed);
        }
    }

    #[tokio::test]
    async fn test_limiter_blocks_over_capacity() {
        let limiter = create_ip_limiter(2, 0);
        assert!(limiter.check("single-ip").await.unwrap().allowed);
        assert!(limiter.check("single-ip").await.unwrap().allowed);
        assert!(!limiter.check("single-ip").await.unwrap().allowed);
    }
}
