use criterion::{black_box, criterion_group, criterion_main, Criterion};
use designguard::prelude::*;
use std::path::PathBuf;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn bench_validate_schematic(c: &mut Criterion) {
    let options = ValidationOptions {
        enable_ai: false,
        offline_mode: true,
        strict_mode: false,
        rules: vec![],
    };

    c.bench_function("validate_schematic", |b| {
        b.iter(|| {
            DesignGuardCore::validate_schematic(
                black_box(&fixture_path("valid_design.kicad_sch")),
                black_box(options.clone()),
            )
        });
    });
}

fn bench_parse_schematic(c: &mut Criterion) {
    c.bench_function("parse_schematic", |b| {
        b.iter(|| {
            designguard::parse_schematic(black_box(&fixture_path("valid_design.kicad_sch")))
        });
    });
}

criterion_group!(benches, bench_validate_schematic, bench_parse_schematic);
criterion_main!(benches);
