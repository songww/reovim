// use criterion::{black_box, criterion_group, criterion_main, Criterion};
// use emojito::find_emoji;
//
// pub fn criterion_benchmark(c: &mut Criterion) {
//     c.bench_function("find emoji", |b| {
//         b.iter(|| find_emoji(black_box("▶\u{fe0f}")))
//     });
// }
//
// criterion_group!(benches, criterion_benchmark);
// criterion_main!(benches);
