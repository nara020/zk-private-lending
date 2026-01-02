# arkworks Circuits (Secondary)

> R1CS-based low-level implementation

## Circuits

| Circuit | Status | Description |
|---------|--------|-------------|
| `collateral.rs` | Planned | Same logic as Halo2, R1CS constraints |

## Key Features

### R1CS Constraints
- Standard multiplication gates: a·b = c
- Bit decomposition for range checks
- ConstraintSynthesizer trait implementation

### Dependencies
```toml
ark-ff = "0.4.2"
ark-ec = "0.4.2"
ark-bn254 = "0.4.0"
ark-groth16 = "0.4.0"
ark-relations = "0.4.0"
```

## Comparison with Halo2

| Aspect | arkworks (R1CS) | Halo2 (PLONKish) |
|--------|-----------------|------------------|
| Range Check (8-bit) | ~16 constraints | 1 lookup |
| Flexibility | Lower | Higher |
| Learning Curve | Moderate | Steep |
