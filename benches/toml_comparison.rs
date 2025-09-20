use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_toml_vs_noml(c: &mut Criterion) {
    // Use basic TOML-compatible content for fair comparison
    let toml_content = r#"
name = "test-app"
version = "1.0.0"
debug = true
numbers = [1, 2, 3, 4, 5]

[server]
host = "localhost" 
port = 8080
workers = 4
ssl = true

[database]
driver = "postgres"
host = "db.example.com"
port = 5432
name = "myapp"
pool_size = 10

[logging]
level = "info"
format = "json"
file = "/var/log/app.log"

[[workers]]
name = "worker1"
type = "background"
enabled = true

[[workers]] 
name = "worker2"
type = "api"
enabled = false
"#;

    let mut group = c.benchmark_group("toml_vs_noml");
    
    group.bench_function("toml_parse", |b| {
        b.iter(|| {
            let _: toml::Value = black_box(toml_content).parse().unwrap();
        })
    });
    
    group.bench_function("noml_parse", |b| {
        b.iter(|| {
            let _ = noml::parse(black_box(toml_content)).unwrap();
        })
    });
    
    group.finish();
}

criterion_group!(benches, bench_toml_vs_noml);
criterion_main!(benches);