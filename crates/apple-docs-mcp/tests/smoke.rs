use apple_docs_mcp::run_server;

#[tokio::test]
async fn server_starts_with_defaults() {
    std::env::set_var("APPLEDOC_CACHE_DIR", "./target/tmp-cache");
    let result = run_server().await;
    assert!(
        result.is_ok(),
        "expected server scaffold to succeed: {result:?}"
    );
}
