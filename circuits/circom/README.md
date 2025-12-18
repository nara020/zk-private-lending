# Circom Circuits (Secondary)

> DSL-based rapid prototyping

## Circuits

| Circuit | Status | Description |
|---------|--------|-------------|
| `collateral.circom` | Planned | Same logic, Circom DSL |

## Key Features

### Circom DSL
- Template-based circuit definition
- Signal flow (input → computation → output)
- Component composition

### Dependencies
- circom 2.1.0
- snarkjs
- circomlib (LessThan, Poseidon, etc.)

## Usage

```bash
# Compile
circom collateral.circom --r1cs --wasm --sym

# Generate witness
node collateral_js/generate_witness.js

# Prove
snarkjs groth16 prove
```

## circomlib Components Used

- `LessThan`: Comparison operations
- `Poseidon`: Efficient hashing
- `Num2Bits`: Bit decomposition
