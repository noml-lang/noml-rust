use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use noml::{parse, parse_from_file, Config};
use std::fs;
use tempfile::NamedTempFile;

fn bench_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("parsing");
    
    // Small config
    let small_config = r#"
name = "test-app"
version = "1.0.0"
debug = true

[server]
host = "localhost"
port = 8080
"#;
    
    // Medium config with env vars and native types
    let medium_config = r#"
# Application Configuration
app_name = "MyApp"
version = "2.1.0"
debug = env("DEBUG", "false")
secret_key = env("SECRET_KEY", "default-secret")

# Server Configuration
[server]
host = env("SERVER_HOST", "0.0.0.0")
port = env("SERVER_PORT", "8080")
workers = 4
timeout = @duration("30s")
max_connections = 1000

# Database Configuration
[database]
url = env("DATABASE_URL", "sqlite:memory:")
pool_size = 10
timeout = @duration("10s")
ssl = true

# Cache Configuration
[cache]
enabled = true
ttl = @duration("1h")
max_size = @size("512MB")
redis_url = env("REDIS_URL", "redis://localhost:6379")

# Features
[features]
analytics = true
monitoring = false
rate_limiting = true

# Array of tables
[[workers]]
name = "worker-1"
threads = 4

[[workers]]
name = "worker-2"
threads = 8
"#;

    // Large config with complex structures
    let large_config = r#"
# Comprehensive NOML Configuration
app_name = "Enterprise-App"
version = "3.2.1"
environment = env("ENVIRONMENT", "development")
debug = env("DEBUG", "false")

# Server Configuration
[server]
host = env("SERVER_HOST", "0.0.0.0")
port = env("SERVER_PORT", "8080")
workers = env("WORKERS", "4")
timeout = @duration("30s")
max_connections = 10000
ssl_cert = "/path/to/cert.pem"
ssl_key = "/path/to/key.pem"

# Multiple Database Configurations
[databases.primary]
url = env("PRIMARY_DB_URL", "postgresql://localhost/primary")
pool_size = env("DB_POOL_SIZE", "20")
timeout = @duration("10s")
ssl = true
max_connections = 100

[databases.analytics]
url = env("ANALYTICS_DB_URL", "postgresql://localhost/analytics")
pool_size = 10
timeout = @duration("5s")
ssl = false

[databases.cache]
url = env("REDIS_URL", "redis://localhost:6379")
pool_size = 5
timeout = @duration("2s")

# Cache Configuration
[cache]
enabled = true
ttl = @duration("1h")
max_size = @size("1GB")
compression = true
encryption = env("CACHE_ENCRYPTION", "false")

# Logging Configuration
[logging]
level = env("LOG_LEVEL", "info")
format = "json"
output = "/var/log/app.log"
rotation = @duration("24h")
max_files = 7
max_size = @size("100MB")

# Monitoring & Metrics
[monitoring]
enabled = true
endpoint = "http://monitoring.internal:9090"
interval = @duration("30s")
timeout = @duration("5s")

[monitoring.alerts]
cpu_threshold = 80.0
memory_threshold = @size("2GB")
disk_threshold = 90.0
response_time_threshold = @duration("2s")

# Security Configuration
[security]
jwt_secret = env("JWT_SECRET", "default-jwt-secret")
encryption_key = env("ENCRYPTION_KEY", "default-encryption-key")
session_timeout = @duration("8h")
max_login_attempts = 5
lockout_duration = @duration("15m")

# Feature Flags
[features]
analytics = env("ENABLE_ANALYTICS", "true")
monitoring = true
rate_limiting = true
caching = true
compression = false
beta_features = env("ENABLE_BETA", "false")

# Rate Limiting
[rate_limiting]
enabled = true
requests_per_minute = 1000
burst_size = 100
window_size = @duration("1m")

# File Storage
[storage]
type = "s3"
bucket = env("S3_BUCKET", "my-app-storage")
region = env("AWS_REGION", "us-east-1")
max_file_size = @size("50MB")
allowed_types = ["image/jpeg", "image/png", "application/pdf"]

# Worker Configurations (Array of Tables)
[[workers]]
name = "web-worker-1"
type = "web"
threads = 4
memory_limit = @size("512MB")
timeout = @duration("30s")

[[workers]]
name = "web-worker-2"
type = "web"
threads = 8
memory_limit = @size("1GB")
timeout = @duration("30s")

[[workers]]
name = "background-worker-1"
type = "background"
threads = 2
memory_limit = @size("256MB")
timeout = @duration("5m")

[[workers]]
name = "analytics-worker"
type = "analytics"
threads = 1
memory_limit = @size("2GB")
timeout = @duration("10m")

# Service Discovery
[[services]]
name = "user-service"
url = "http://user-service.internal:8080"
timeout = @duration("5s")
retries = 3
health_check = "/health"

[[services]]
name = "payment-service"
url = "http://payment-service.internal:8080"
timeout = @duration("10s")
retries = 5
health_check = "/health"

[[services]]
name = "notification-service"
url = "http://notification-service.internal:8080"
timeout = @duration("3s")
retries = 2
health_check = "/health"
"#;

    // Benchmark parsing different sizes
    group.bench_with_input(BenchmarkId::new("small_config", "parse"), &small_config, |b, config| {
        b.iter(|| {
            let result = parse(black_box(config));
            black_box(result.unwrap());
        });
    });

    group.bench_with_input(BenchmarkId::new("medium_config", "parse"), &medium_config, |b, config| {
        b.iter(|| {
            let result = parse(black_box(config));
            black_box(result.unwrap());
        });
    });

    group.bench_with_input(BenchmarkId::new("large_config", "parse"), &large_config, |b, config| {
        b.iter(|| {
            let result = parse(black_box(config));
            black_box(result.unwrap());
        });
    });

    group.finish();
}

fn bench_file_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_operations");
    
    let config_content = r#"
app_name = "BenchmarkApp"
version = "1.0.0"
debug = env("DEBUG", "true")

[server]
host = "localhost"
port = 8080
timeout = @duration("30s")

[database]
url = env("DATABASE_URL", "sqlite:memory:")
pool_size = 10
max_size = @size("1GB")
"#;

    group.bench_function("parse_from_file", |b| {
        b.iter_batched(
            || {
                let temp_file = NamedTempFile::new().unwrap();
                fs::write(&temp_file, config_content).unwrap();
                temp_file
            },
            |temp_file| {
                let result = parse_from_file(temp_file.path());
                black_box(result.unwrap());
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

fn bench_config_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("config_operations");
    
    let config_content = r#"
app_name = "ConfigApp"
version = "1.0.0"
debug = true

[server]
host = "localhost"
port = 8080

[database]
host = "localhost"
port = 5432
"#;

    group.bench_function("config_creation", |b| {
        b.iter(|| {
            let config = Config::from_string(black_box(config_content));
            black_box(config.unwrap());
        });
    });

    group.bench_function("config_access", |b| {
        let config = Config::from_string(config_content).unwrap();
        b.iter(|| {
            let app_name = config.get(black_box("app_name"));
            let port = config.get(black_box("server.port"));
            let host = config.get(black_box("database.host"));
            black_box((app_name, port, host));
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_parsing,
    bench_file_operations, 
    bench_config_operations
);
criterion_main!(benches);