//! Async functionality tests for NOML
//!
//! These tests verify that async parsing and configuration management work correctly.

#[cfg(feature = "async")]
#[cfg(test)]
mod async_tests {
    use noml::{parse_async, parse_from_file_async, Config, Value};
    use std::env;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_parse_async_basic() {
        let source = r#"
            name = "async-test"
            version = "1.0.0"
            debug = true
        "#;

        let config = parse_async(source).await.unwrap();

        assert_eq!(
            config.get("name").unwrap().as_string().unwrap(),
            "async-test"
        );
        assert_eq!(config.get("version").unwrap().as_string().unwrap(), "1.0.0");
        assert!(config.get("debug").unwrap().as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_parse_from_file_async() {
        // Create a temporary file
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(
            temp_file,
            r#"
            app_name = "file-test"
            server.port = 8080
            features = ["async", "parsing"]
        "#
        )
        .unwrap();

        let config = parse_from_file_async(temp_file.path()).await.unwrap();

        assert_eq!(
            config.get("app_name").unwrap().as_string().unwrap(),
            "file-test"
        );
        assert_eq!(
            config.get("server.port").unwrap().as_integer().unwrap(),
            8080
        );

        let features = config.get("features").unwrap().as_array().unwrap();
        assert_eq!(features.len(), 2);
        assert_eq!(features[0].as_string().unwrap(), "async");
        assert_eq!(features[1].as_string().unwrap(), "parsing");
    }

    #[tokio::test]
    async fn test_config_load_async() {
        // Create a temporary file
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(
            temp_file,
            r#"
            database.host = "localhost"
            database.port = 5432
            cache.enabled = true
        "#
        )
        .unwrap();

        let config = Config::load_async(temp_file.path()).await.unwrap();

        assert_eq!(
            config.get("database.host").unwrap().as_string().unwrap(),
            "localhost"
        );
        assert_eq!(
            config.get("database.port").unwrap().as_integer().unwrap(),
            5432
        );
        assert!(config.get("cache.enabled").unwrap().as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_config_save_async() {
        let mut config = Config::new();
        config.set("app_name", "save-test").unwrap();
        config.set("server.port", 9000).unwrap();
        config
            .set(
                "features",
                Value::Array(vec![
                    Value::String("async".to_string()),
                    Value::String("save".to_string()),
                ]),
            )
            .unwrap();

        // Save to temporary file
        let temp_file = NamedTempFile::new().unwrap();
        config.save_async(temp_file.path()).await.unwrap();

        // Reload and verify
        let reloaded = Config::load_async(temp_file.path()).await.unwrap();
        assert_eq!(
            reloaded.get("app_name").unwrap().as_string().unwrap(),
            "save-test"
        );
        assert_eq!(
            reloaded.get("server.port").unwrap().as_integer().unwrap(),
            9000
        );
    }

    #[tokio::test]
    async fn test_config_reload_async() {
        // Create initial file
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(
            temp_file,
            r#"
            initial_value = "original"
            counter = 1
        "#
        )
        .unwrap();

        let mut config = Config::load_async(temp_file.path()).await.unwrap();
        assert_eq!(
            config.get("initial_value").unwrap().as_string().unwrap(),
            "original"
        );
        assert_eq!(config.get("counter").unwrap().as_integer().unwrap(), 1);

        // Modify config in memory
        config.set("initial_value", "modified").unwrap();
        config.set("counter", 999).unwrap();

        assert_eq!(
            config.get("initial_value").unwrap().as_string().unwrap(),
            "modified"
        );
        assert_eq!(config.get("counter").unwrap().as_integer().unwrap(), 999);

        // Reload should restore original values
        config.reload_async().await.unwrap();
        assert_eq!(
            config.get("initial_value").unwrap().as_string().unwrap(),
            "original"
        );
        assert_eq!(config.get("counter").unwrap().as_integer().unwrap(), 1);
    }

    #[tokio::test]
    async fn test_async_with_env_vars() {
        env::set_var("ASYNC_TEST_PORT", "7777");
        env::set_var("ASYNC_TEST_HOST", "async.example.com");

        let source = r#"
            host = env("ASYNC_TEST_HOST")
            port = env("ASYNC_TEST_PORT")
            fallback = env("MISSING_VAR", "default_async")
        "#;

        let config = parse_async(source).await.unwrap();

        assert_eq!(
            config.get("host").unwrap().as_string().unwrap(),
            "async.example.com"
        );
        assert_eq!(config.get("port").unwrap().as_string().unwrap(), "7777");
        assert_eq!(
            config.get("fallback").unwrap().as_string().unwrap(),
            "default_async"
        );

        // Clean up
        env::remove_var("ASYNC_TEST_PORT");
        env::remove_var("ASYNC_TEST_HOST");
    }

    #[tokio::test]
    async fn test_async_basic_functionality() {
        let source = r#"
            app_name = "async-interpolation"
            version = "2.0.0"
            enabled = true
            
            [database]
            host = "localhost"
            port = 5432
            
            [features]
            async_support = true
            hot_reload = false
        "#;

        let config = parse_async(source).await.unwrap();

        assert_eq!(
            config.get("app_name").unwrap().as_string().unwrap(),
            "async-interpolation"
        );
        assert_eq!(config.get("version").unwrap().as_string().unwrap(), "2.0.0");
        assert!(config.get("enabled").unwrap().as_bool().unwrap());
        assert_eq!(
            config.get("database.host").unwrap().as_string().unwrap(),
            "localhost"
        );
        assert_eq!(
            config.get("database.port").unwrap().as_integer().unwrap(),
            5432
        );
        assert!(config
            .get("features.async_support")
            .unwrap()
            .as_bool()
            .unwrap());
        assert!(!config
            .get("features.hot_reload")
            .unwrap()
            .as_bool()
            .unwrap());
    }

    #[tokio::test]
    async fn test_async_error_handling() {
        // Test parsing invalid syntax
        let result = parse_async("invalid === syntax").await;
        assert!(result.is_err());

        // Test loading non-existent file
        let result = parse_from_file_async("/path/that/does/not/exist.noml").await;
        assert!(result.is_err());

        // Test reload without source path
        let mut config = Config::new();
        let result = config.reload_async().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_http_includes_error_without_async() {
        // Test that HTTP includes are properly rejected in sync mode
        let source = r#"
            base_config = include "https://example.com/config.noml"
        "#;

        let result = noml::parse(source);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("HTTP includes require async")
                || error_msg.contains("async resolver")
        );
    }

    #[tokio::test]
    async fn test_async_new_native_types() {
        let source = r#"
            server_ip = @ip("192.168.1.1")
            app_version = @semver("1.2.3")
            secret_data = @base64("SGVsbG8gV29ybGQ=")
            user_id = @uuid("550e8400-e29b-41d4-a716-446655440000")
        "#;

        let config = parse_async(source).await.unwrap();

        assert_eq!(
            config.get("server_ip").unwrap().as_string().unwrap(),
            "192.168.1.1"
        );
        assert_eq!(
            config.get("app_version").unwrap().as_string().unwrap(),
            "1.2.3"
        );
        assert_eq!(
            config.get("secret_data").unwrap().as_string().unwrap(),
            "SGVsbG8gV29ybGQ="
        );
        assert_eq!(
            config.get("user_id").unwrap().as_string().unwrap(),
            "550e8400-e29b-41d4-a716-446655440000"
        );
    }
}
