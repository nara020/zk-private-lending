//! Benchmarks for ZK circuits
//!
//! Run with: cargo bench

use criterion::{criterion_group, criterion_main, Criterion};
use halo2_proofs::{
    circuit::Value,
    dev::MockProver,
    pasta::Fp,
};
use zk_private_lending_circuits::{CollateralCircuit, LTVCircuit, LiquidationCircuit};

fn bench_collateral_proof(c: &mut Criterion) {
    let k = 17;

    let collateral = Fp::from(1000u64);
    let salt = Fp::from(12345u64);
    let threshold = Fp::from(500u64);
    let commitment = CollateralCircuit::compute_commitment(collateral, salt);

    let circuit = CollateralCircuit::new(collateral, salt, threshold, commitment);
    let public_inputs = vec![threshold, commitment];

    c.bench_function("CollateralProof MockProver", |b| {
        b.iter(|| {
            let prover = MockProver::run(k, &circuit.clone(), vec![public_inputs.clone()]).unwrap();
            prover.verify().unwrap();
        });
    });
}

fn bench_ltv_proof(c: &mut Criterion) {
    let k = 17;

    let debt = Fp::from(60u64);
    let collateral = Fp::from(100u64);
    let salt_d = Fp::from(11111u64);
    let salt_c = Fp::from(22222u64);
    let max_ltv = Fp::from(80u64);

    let debt_commitment = LTVCircuit::compute_commitment(debt, salt_d);
    let collateral_commitment = LTVCircuit::compute_commitment(collateral, salt_c);

    let circuit = LTVCircuit::new(debt, collateral, salt_d, salt_c, max_ltv);
    let public_inputs = vec![max_ltv, debt_commitment, collateral_commitment];

    c.bench_function("LTVProof MockProver", |b| {
        b.iter(|| {
            let prover = MockProver::run(k, &circuit.clone(), vec![public_inputs.clone()]).unwrap();
            prover.verify().unwrap();
        });
    });
}

criterion_group!(benches, bench_collateral_proof, bench_ltv_proof);
criterion_main!(benches);
