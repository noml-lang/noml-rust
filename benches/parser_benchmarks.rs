// extern crate test is no longer needed when using the external crate

use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_example(c: &mut Criterion) {
	c.bench_function("example", |b| {
		b.iter(|| {
			// Example code to benchmark
			let x = 2 + 2;
			black_box(x);
		});
	});
}

criterion_group!(benches, bench_example);
criterion_main!(benches);