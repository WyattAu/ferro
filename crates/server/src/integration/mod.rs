pub mod audit_integration;
pub mod cache_layer;
pub mod config_integration;
pub mod crdt_integration;
pub mod distributed_integration;
pub mod e2ee_integration;
pub mod event_dispatch;
pub mod health_integration;
pub mod multi_tenant_integration;
pub mod offline_integration;
pub mod rate_limit;
pub mod search;
pub mod session_integration;
pub mod storage_integration;
pub mod webhook_integration;

#[cfg(test)]
mod tests {
    use ferro_rate_limiter::RateLimiter;

    use super::*;

    #[tokio::test]
    async fn test_create_ip_limiter() {
        let limiter = rate_limit::create_ip_limiter(10, 5);
        let result = limiter.check("test-key").await.unwrap();
        assert!(result.allowed);
        assert_eq!(result.remaining, 9);
    }

    #[test]
    fn test_cache_key_generation() {
        let key = cache_layer::cache_key("GET", "/files/test.txt", "type=json");
        assert_eq!(key, "GET:/files/test.txt:type=json");
    }

    #[test]
    fn test_search_index_create_and_search() {
        let index = search::create_file_search_index();
        search::index_file(
            &index,
            "doc-1",
            "report.pdf",
            "/docs/report.pdf",
            "application/pdf",
            1024,
        );
        let results = search::search_files(&index, "report", 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document_id, "doc-1");
    }

    #[test]
    fn test_audit_log_create_and_record() {
        let log = audit_integration::create_audit_log();
        assert!(log.count().is_ok());
    }

    #[test]
    fn test_event_bus_create() {
        let bus = event_dispatch::create_event_bus();
        assert_eq!(bus.handler_count("file.created"), 0);
    }

    #[test]
    fn test_health_checker_create() {
        let checker = health_integration::create_health_checker();
        assert_eq!(checker.probe_count(), 0);
    }
}
