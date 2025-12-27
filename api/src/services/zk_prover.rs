//! ZK Prover Service - Real Halo2 Integration
//!
//! # Interview Q&A
//!
//! Q: ZK Proof 생성 과정을 설명해주세요
//! A: 4단계 과정
//!
//!    1. Circuit Setup (일회성)
//!       - 회로 정의 (constraint system)
//!       - keygen() → Proving Key (PK) + Verification Key (VK)
//!       - PK는 수십 MB, 메모리에 캐싱
//!
//!    2. Witness 생성
//!       - Private input (collateral, salt)과 public input (threshold) 준비
//!       - 회로의 모든 와이어(wire) 값 계산
//!
//!    3. Proof 생성
//!       - create_proof(PK, Circuit, instance) → Proof
//!       - 다항식 연산, FFT, MSM 등 수학적 연산
//!       - CPU 집약적 작업 (수 초 소요)
//!
//!    4. Proof 직렬화
//!       - Proof 객체를 bytes로 변환
//!       - Solidity 호환 형식 (Groth16 스타일)
//!
//! Q: Halo2와 Groth16의 차이점은?
//! A: 주요 차이점:
//!    - Setup: Halo2는 Universal Setup (재사용 가능), Groth16은 Per-circuit
//!    - Proof 크기: Halo2 ~수KB, Groth16 ~128B
//!    - 검증 시간: Groth16이 더 빠름 (pairing 1회 vs 다중)
//!    - 유연성: Halo2는 Lookup table, Custom gate 지원
//!
//! Q: 성능 최적화 방법은?
//! A: 1. Proving Key 캐싱 (한 번 생성 후 재사용)
//!    2. 병렬 처리 (rayon crate)
//!    3. 회로 최적화 (constraint 수 줄이기)
//!    4. GPU 가속 (향후 적용)

use anyhow::{Context, Result, anyhow};
use std::sync::Arc;
use tokio::sync::RwLock;

use halo2_proofs::{
    plonk::{create_proof, keygen_pk, keygen_vk, verify_proof, ProvingKey, VerifyingKey},
    poly::{
        commitment::ParamsProver,
        ipa::{
            commitment::{IPACommitmentScheme, ParamsIPA},
            multiopen::ProverIPA,
            strategy::SingleStrategy,
        },
        VerificationStrategy,
    },
    transcript::{Blake2bRead, Blake2bWrite, Challenge255, TranscriptReadBuffer, TranscriptWriterBuffer},
};
use pasta_curves::{Fp, EqAffine};
use ff::PrimeField;
use rand::rngs::OsRng;

use zk_private_lending_circuits::{CollateralCircuit, LTVCircuit, LiquidationCircuit};

use crate::routes::proof::ProofData;

/// ZK Proof 생성 결과
pub struct ProofResult {
    pub proof: ProofData,
    pub public_inputs: Vec<String>,
    pub commitment: String,
}

/// Cached proving context
///
/// # Design Decision
///
/// Proving Key와 Verification Key를 캐싱하는 이유:
/// - keygen은 비용이 큼 (~수 초)
/// - 동일한 회로에 대해 재사용 가능
/// - 메모리 사용량: 각 회로당 ~50-100MB
struct ProvingContext {
    params: ParamsIPA<EqAffine>,
    collateral_pk: Option<ProvingKey<EqAffine>>,
    collateral_vk: Option<VerifyingKey<EqAffine>>,
    ltv_pk: Option<ProvingKey<EqAffine>>,
    ltv_vk: Option<VerifyingKey<EqAffine>>,
    liquidation_pk: Option<ProvingKey<EqAffine>>,
    liquidation_vk: Option<VerifyingKey<EqAffine>>,
}

/// ZK Prover 서비스
///
/// # Architecture
///
/// ```text
/// ┌─────────────────────────────────────────────────────────────┐
/// │                      ZKProver                                │
/// ├─────────────────────────────────────────────────────────────┤
/// │                                                             │
/// │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
/// │  │ Collateral  │  │    LTV      │  │    Liquidation      │ │
/// │  │   Circuit   │  │   Circuit   │  │      Circuit        │ │
/// │  └──────┬──────┘  └──────┬──────┘  └──────────┬──────────┘ │
/// │         │                │                    │             │
/// │         v                v                    v             │
/// │  ┌──────────────────────────────────────────────────────┐  │
/// │  │                 ProvingContext                        │  │
/// │  │  - Params (SRS)                                       │  │
/// │  │  - ProvingKeys (cached)                               │  │
/// │  │  - VerificationKeys (cached)                          │  │
/// │  └──────────────────────────────────────────────────────┘  │
/// │                                                             │
/// └─────────────────────────────────────────────────────────────┘
/// ```
pub struct ZKProver {
    /// Thread-safe proving context
    context: Arc<RwLock<ProvingContext>>,
    /// Circuit size parameter (k = log2(rows))
    k: u32,
}

impl ZKProver {
    /// 새 ZK Prover 생성
    ///
    /// # Arguments
    ///
    /// * `k` - Circuit size parameter (2^k rows).
    ///         Larger k = more rows = larger circuits supported
    ///         Typical values: 17-20 for production
    ///
    /// # Performance
    ///
    /// - Params 생성: ~1-2초
    /// - 메모리 사용: ~2^k * 32 bytes
    pub fn new() -> Result<Self> {
        let k = 17; // 2^17 = 131,072 rows

        tracing::info!("Initializing ZK Prover with k={}...", k);

        // Generate parameters (SRS - Structured Reference String)
        // In production, this would be loaded from a file
        let params = ParamsIPA::<EqAffine>::new(k);

        tracing::info!("SRS parameters generated");

        let context = ProvingContext {
            params,
            collateral_pk: None,
            collateral_vk: None,
            ltv_pk: None,
            ltv_vk: None,
            liquidation_pk: None,
            liquidation_vk: None,
        };

        Ok(Self {
            context: Arc::new(RwLock::new(context)),
            k,
        })
    }

    /// 회로별 Proving Key 생성 (lazy initialization)
    ///
    /// # Interview Q&A
    ///
    /// Q: 왜 lazy initialization을 사용하는가?
    /// A: keygen이 비용이 크기 때문
    ///    - 첫 번째 proof 요청 시에만 생성
    ///    - 서버 시작 시간 단축
    ///    - 사용하지 않는 회로의 키는 생성하지 않음
    async fn ensure_collateral_keys(&self) -> Result<()> {
        let read_guard = self.context.read().await;
        if read_guard.collateral_pk.is_some() {
            return Ok(());
        }
        drop(read_guard);

        tracing::info!("Generating CollateralCircuit keys...");

        let mut write_guard = self.context.write().await;

        // Double-check after acquiring write lock
        if write_guard.collateral_pk.is_some() {
            return Ok(());
        }

        let empty_circuit = CollateralCircuit::<Fp>::default();

        let vk = keygen_vk(&write_guard.params, &empty_circuit)
            .context("Failed to generate verification key")?;

        let pk = keygen_pk(&write_guard.params, vk.clone(), &empty_circuit)
            .context("Failed to generate proving key")?;

        write_guard.collateral_vk = Some(vk);
        write_guard.collateral_pk = Some(pk);

        tracing::info!("CollateralCircuit keys generated successfully");
        Ok(())
    }

    async fn ensure_ltv_keys(&self) -> Result<()> {
        let read_guard = self.context.read().await;
        if read_guard.ltv_pk.is_some() {
            return Ok(());
        }
        drop(read_guard);

        tracing::info!("Generating LTVCircuit keys...");

        let mut write_guard = self.context.write().await;

        if write_guard.ltv_pk.is_some() {
            return Ok(());
        }

        let empty_circuit = LTVCircuit::<Fp>::default();

        let vk = keygen_vk(&write_guard.params, &empty_circuit)
            .context("Failed to generate LTV verification key")?;

        let pk = keygen_pk(&write_guard.params, vk.clone(), &empty_circuit)
            .context("Failed to generate LTV proving key")?;

        write_guard.ltv_vk = Some(vk);
        write_guard.ltv_pk = Some(pk);

        tracing::info!("LTVCircuit keys generated successfully");
        Ok(())
    }

    async fn ensure_liquidation_keys(&self) -> Result<()> {
        let read_guard = self.context.read().await;
        if read_guard.liquidation_pk.is_some() {
            return Ok(());
        }
        drop(read_guard);

        tracing::info!("Generating LiquidationCircuit keys...");

        let mut write_guard = self.context.write().await;

        if write_guard.liquidation_pk.is_some() {
            return Ok(());
        }

        let empty_circuit = LiquidationCircuit::<Fp>::default();

        let vk = keygen_vk(&write_guard.params, &empty_circuit)
            .context("Failed to generate Liquidation verification key")?;

        let pk = keygen_pk(&write_guard.params, vk.clone(), &empty_circuit)
            .context("Failed to generate Liquidation proving key")?;

        write_guard.liquidation_vk = Some(vk);
        write_guard.liquidation_pk = Some(pk);

        tracing::info!("LiquidationCircuit keys generated successfully");
        Ok(())
    }

    /// Commitment 계산
    ///
    /// # Algorithm
    ///
    /// commitment = value * salt + value
    ///
    /// NOTE: This is a simplified commitment for demo purposes.
    /// For production, implement Poseidon hash with circuit constraints.
    ///
    /// # Interview Q&A
    ///
    /// Q: 왜 Poseidon 해시를 사용하는가? (production에서)
    /// A: ZK-friendly 해시 함수
    ///    - SHA256: ~25,000 constraints
    ///    - Poseidon: ~300 constraints (80배 이상 효율적!)
    ///    - 회로 내에서 계산 가능
    pub fn compute_commitment(&self, value: u128, salt: u128) -> Result<Vec<u8>> {
        let value_fp = Fp::from_u128(value);
        let salt_fp = Fp::from_u128(salt);

        // Use the same formula as the circuit
        let commitment = CollateralCircuit::<Fp>::compute_commitment(value_fp, salt_fp);

        // Convert to bytes
        Ok(commitment.to_repr().as_ref().to_vec())
    }

    /// 담보 충분 증명 생성 (실제 Halo2 사용)
    ///
    /// # Circuit Logic
    ///
    /// ```text
    /// Private inputs: collateral, salt
    /// Public inputs: threshold, commitment
    ///
    /// Constraints:
    /// 1. collateral >= threshold (comparison gate)
    /// 2. commitment == Hash(collateral, salt) (commitment gate)
    /// ```
    ///
    /// # Performance
    ///
    /// - 첫 번째 호출: ~5초 (keygen 포함)
    /// - 이후 호출: ~1-2초 (proving only)
    pub async fn generate_collateral_proof(
        &self,
        collateral: u128,
        threshold: u128,
        salt: u128,
    ) -> Result<ProofResult> {
        tracing::info!(
            "Generating collateral proof: collateral={}, threshold={}",
            collateral, threshold
        );

        // Ensure proving key is ready
        self.ensure_collateral_keys().await?;

        // Convert to field elements
        let collateral_fp = Fp::from_u128(collateral);
        let salt_fp = Fp::from_u128(salt);
        let threshold_fp = Fp::from_u128(threshold);

        // Compute commitment
        let commitment = CollateralCircuit::<Fp>::compute_commitment(collateral_fp, salt_fp);

        // Create circuit instance
        let circuit = CollateralCircuit::new(collateral_fp, salt_fp, threshold_fp, commitment);

        // Public inputs
        let public_inputs = vec![threshold_fp, commitment];

        // Generate proof
        let proof_bytes = {
            let context = self.context.read().await;
            let pk = context.collateral_pk.as_ref()
                .ok_or_else(|| anyhow!("Proving key not initialized"))?;

            let mut transcript = Blake2bWrite::<Vec<u8>, EqAffine, Challenge255<EqAffine>>::init(vec![]);

            create_proof::<
                IPACommitmentScheme<EqAffine>,
                ProverIPA<'_, EqAffine>,
                _,
                _,
                _,
                _,
            >(
                &context.params,
                pk,
                &[circuit],
                &[&[&public_inputs]],
                OsRng,
                &mut transcript,
            ).context("Failed to create proof")?;

            transcript.finalize()
        };

        // Convert to Solidity-compatible format
        let proof = self.serialize_proof_to_groth16(&proof_bytes);

        let commitment_hex = self.fp_to_hex(commitment);

        Ok(ProofResult {
            proof,
            public_inputs: vec![
                self.fp_to_hex(threshold_fp),
                commitment_hex.clone(),
            ],
            commitment: commitment_hex,
        })
    }

    /// LTV 비율 증명 생성
    ///
    /// # Circuit Logic
    ///
    /// Proves: debt/collateral <= max_ltv/100
    /// Without division: debt * 100 <= collateral * max_ltv
    pub async fn generate_ltv_proof(
        &self,
        collateral: u128,
        debt: u128,
        max_ltv: u64,
        collateral_salt: u128,
        debt_salt: u128,
    ) -> Result<ProofResult> {
        tracing::info!(
            "Generating LTV proof: collateral={}, debt={}, max_ltv={}%",
            collateral, debt, max_ltv
        );

        self.ensure_ltv_keys().await?;

        let collateral_fp = Fp::from_u128(collateral);
        let debt_fp = Fp::from_u128(debt);
        let max_ltv_fp = Fp::from(max_ltv);
        let collateral_salt_fp = Fp::from_u128(collateral_salt);
        let debt_salt_fp = Fp::from_u128(debt_salt);

        // Compute commitments using the circuit formula
        let debt_commitment = LTVCircuit::<Fp>::compute_commitment(debt_fp, debt_salt_fp);
        let collateral_commitment = LTVCircuit::<Fp>::compute_commitment(collateral_fp, collateral_salt_fp);

        // Create circuit with correct argument order: (debt, collateral, salt_d, salt_c, max_ltv)
        let circuit = LTVCircuit::new(
            debt_fp,
            collateral_fp,
            debt_salt_fp,
            collateral_salt_fp,
            max_ltv_fp,
        );

        let public_inputs = vec![max_ltv_fp, debt_commitment, collateral_commitment];

        let proof_bytes = {
            let context = self.context.read().await;
            let pk = context.ltv_pk.as_ref()
                .ok_or_else(|| anyhow!("LTV proving key not initialized"))?;

            let mut transcript = Blake2bWrite::<Vec<u8>, EqAffine, Challenge255<EqAffine>>::init(vec![]);

            create_proof::<
                IPACommitmentScheme<EqAffine>,
                ProverIPA<'_, EqAffine>,
                _,
                _,
                _,
                _,
            >(
                &context.params,
                pk,
                &[circuit],
                &[&[&public_inputs]],
                OsRng,
                &mut transcript,
            ).context("Failed to create LTV proof")?;

            transcript.finalize()
        };

        let proof = self.serialize_proof_to_groth16(&proof_bytes);

        Ok(ProofResult {
            proof,
            public_inputs: vec![
                self.fp_to_hex(max_ltv_fp),
                self.fp_to_hex(debt_commitment),
                self.fp_to_hex(collateral_commitment),
            ],
            commitment: self.fp_to_hex(collateral_commitment),
        })
    }

    /// 청산 가능 증명 생성
    ///
    /// # Circuit Logic
    ///
    /// Proves: health_factor < 1.0
    /// = (collateral * price * liq_threshold) < (debt * 100 * 1e8)
    pub async fn generate_liquidation_proof(
        &self,
        collateral: u128,
        debt: u128,
        price: u128,
        liquidation_threshold: u64,
        salt: u128,
    ) -> Result<ProofResult> {
        tracing::info!(
            "Generating liquidation proof: collateral={}, debt={}, price={}",
            collateral, debt, price
        );

        self.ensure_liquidation_keys().await?;

        let collateral_fp = Fp::from_u128(collateral);
        let debt_fp = Fp::from_u128(debt);
        let price_fp = Fp::from_u128(price);
        let liq_threshold_fp = Fp::from(liquidation_threshold);
        let salt_fp = Fp::from_u128(salt);

        // Compute position hash using the circuit formula
        let position_hash = LiquidationCircuit::<Fp>::compute_position_hash(
            collateral_fp,
            debt_fp,
            salt_fp,
        );

        let circuit = LiquidationCircuit::new(
            collateral_fp,
            debt_fp,
            salt_fp,
            price_fp,
            liq_threshold_fp,
        );

        let public_inputs = vec![price_fp, liq_threshold_fp, position_hash];

        let proof_bytes = {
            let context = self.context.read().await;
            let pk = context.liquidation_pk.as_ref()
                .ok_or_else(|| anyhow!("Liquidation proving key not initialized"))?;

            let mut transcript = Blake2bWrite::<Vec<u8>, EqAffine, Challenge255<EqAffine>>::init(vec![]);

            create_proof::<
                IPACommitmentScheme<EqAffine>,
                ProverIPA<'_, EqAffine>,
                _,
                _,
                _,
                _,
            >(
                &context.params,
                pk,
                &[circuit],
                &[&[&public_inputs]],
                OsRng,
                &mut transcript,
            ).context("Failed to create liquidation proof")?;

            transcript.finalize()
        };

        let proof = self.serialize_proof_to_groth16(&proof_bytes);

        Ok(ProofResult {
            proof,
            public_inputs: vec![
                self.fp_to_hex(price_fp),
                self.fp_to_hex(liq_threshold_fp),
                self.fp_to_hex(position_hash),
            ],
            commitment: self.fp_to_hex(position_hash),
        })
    }

    /// Field element를 hex 문자열로 변환
    fn fp_to_hex(&self, fp: Fp) -> String {
        let bytes = fp.to_repr();
        format!("0x{}", hex::encode(bytes.as_ref()))
    }

    /// Halo2 proof를 Groth16 스타일 형식으로 변환
    ///
    /// # Note
    ///
    /// Halo2는 Groth16과 다른 proof 형식을 사용
    /// 온체인 검증을 위해 변환이 필요
    ///
    /// # Interview Q&A
    ///
    /// Q: 왜 Groth16 형식으로 변환하는가?
    /// A: EVM 호환성
    ///    - EVM에는 BN254 pairing 프리컴파일만 있음
    ///    - Halo2 native proof는 직접 검증 불가
    ///    - 실제로는 Halo2→Groth16 래퍼 사용하거나
    ///    - 커스텀 온체인 검증기 배포
    fn serialize_proof_to_groth16(&self, proof_bytes: &[u8]) -> ProofData {
        // For demonstration, we'll create a mock Groth16-style proof
        // In production, you would use a proper conversion or a different approach

        // Take portions of the Halo2 proof to create Groth16-style elements
        let len = proof_bytes.len();

        // Create G1 point A (64 bytes = 2 x 32-byte coordinates)
        let a_x = if len >= 32 { &proof_bytes[0..32] } else { &[0u8; 32] };
        let a_y = if len >= 64 { &proof_bytes[32..64] } else { &[0u8; 32] };

        // Create G2 point B (128 bytes = 2 x 2 x 32-byte coordinates)
        let b_x1 = if len >= 96 { &proof_bytes[64..96] } else { &[0u8; 32] };
        let b_x2 = if len >= 128 { &proof_bytes[96..128] } else { &[0u8; 32] };
        let b_y1 = if len >= 160 { &proof_bytes[128..160] } else { &[0u8; 32] };
        let b_y2 = if len >= 192 { &proof_bytes[160..192] } else { &[0u8; 32] };

        // Create G1 point C
        let c_x = if len >= 224 { &proof_bytes[192..224] } else { &[0u8; 32] };
        let c_y = if len >= 256 { &proof_bytes[224..256] } else { &[0u8; 32] };

        ProofData {
            a: [
                format!("0x{}", hex::encode(a_x)),
                format!("0x{}", hex::encode(a_y)),
            ],
            b: [
                [
                    format!("0x{}", hex::encode(b_x1)),
                    format!("0x{}", hex::encode(b_x2)),
                ],
                [
                    format!("0x{}", hex::encode(b_y1)),
                    format!("0x{}", hex::encode(b_y2)),
                ],
            ],
            c: [
                format!("0x{}", hex::encode(c_x)),
                format!("0x{}", hex::encode(c_y)),
            ],
        }
    }

    /// Proof 검증 (선택적)
    ///
    /// 서버에서 proof를 반환하기 전에 검증할 수 있음
    /// 클라이언트가 잘못된 proof를 받지 않도록 보장
    #[allow(dead_code)]
    async fn verify_collateral_proof(
        &self,
        proof_bytes: &[u8],
        public_inputs: &[Fp],
    ) -> Result<bool> {
        let context = self.context.read().await;
        let vk = context.collateral_vk.as_ref()
            .ok_or_else(|| anyhow!("Verification key not initialized"))?;

        let mut transcript = Blake2bRead::<_, _, Challenge255<_>>::init(proof_bytes);

        let strategy = SingleStrategy::new(&context.params);
        let instances: &[&[Fp]] = &[public_inputs];
        let result = verify_proof::<_, _, _, _, _>(
            &context.params,
            vk,
            strategy,
            &[instances],
            &mut transcript,
        );

        Ok(result.is_ok())
    }
}

/// Mock ZKProver for testing (no actual Halo2)
#[cfg(test)]
pub struct MockZKProver;

#[cfg(test)]
impl MockZKProver {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }

    pub fn compute_commitment(&self, value: u128, salt: u128) -> Vec<u8> {
        use sha3::{Keccak256, Digest};
        let mut hasher = Keccak256::new();
        hasher.update(value.to_le_bytes());
        hasher.update(salt.to_le_bytes());
        hasher.finalize().to_vec()
    }

    pub async fn generate_collateral_proof(
        &self,
        collateral: u128,
        threshold: u128,
        salt: u128,
    ) -> Result<ProofResult> {
        let commitment = self.compute_commitment(collateral, salt);
        let commitment_hex = format!("0x{}", hex::encode(&commitment));

        Ok(ProofResult {
            proof: self.mock_proof(),
            public_inputs: vec![
                format!("0x{:064x}", threshold),
                commitment_hex.clone(),
            ],
            commitment: commitment_hex,
        })
    }

    fn mock_proof(&self) -> ProofData {
        ProofData {
            a: [
                "0x0000000000000000000000000000000000000000000000000000000000000001".to_string(),
                "0x0000000000000000000000000000000000000000000000000000000000000002".to_string(),
            ],
            b: [
                [
                    "0x0000000000000000000000000000000000000000000000000000000000000003".to_string(),
                    "0x0000000000000000000000000000000000000000000000000000000000000004".to_string(),
                ],
                [
                    "0x0000000000000000000000000000000000000000000000000000000000000005".to_string(),
                    "0x0000000000000000000000000000000000000000000000000000000000000006".to_string(),
                ],
            ],
            c: [
                "0x0000000000000000000000000000000000000000000000000000000000000007".to_string(),
                "0x0000000000000000000000000000000000000000000000000000000000000008".to_string(),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commitment_deterministic() {
        let prover = MockZKProver::new().unwrap();

        let c1 = prover.compute_commitment(100, 12345);
        let c2 = prover.compute_commitment(100, 12345);

        assert_eq!(c1, c2, "Same inputs should produce same commitment");
    }

    #[test]
    fn test_commitment_different_salt() {
        let prover = MockZKProver::new().unwrap();

        let c1 = prover.compute_commitment(100, 12345);
        let c2 = prover.compute_commitment(100, 67890);

        assert_ne!(c1, c2, "Different salts should produce different commitments");
    }

    #[tokio::test]
    async fn test_mock_collateral_proof() {
        let prover = MockZKProver::new().unwrap();

        let result = prover.generate_collateral_proof(1000, 500, 12345).await.unwrap();

        assert!(!result.commitment.is_empty());
        assert_eq!(result.public_inputs.len(), 2);
    }
}
