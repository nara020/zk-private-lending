# Halo2 Circuits (Primary)

> PSE halo2_proofs 0.3 based implementation

## Circuits

| Circuit | Status | Description |
|---------|--------|-------------|
| `collateral.rs` | Planned | Proves collateral >= threshold |
| `ltv.rs` | Planned | Proves LTV ratio within bounds |
| `liquidation.rs` | Planned | Proves liquidation eligibility |

## Key Features

### PLONKish Arithmetization
- Custom gates for complex constraints
- Lookup tables for efficient range checks
- Selector columns for gate activation

### Column Types
- **Instance**: Public inputs (threshold, commitment hash)
- **Advice**: Private witness (actual collateral amount)
- **Fixed**: Constants (lookup table values)
- **Selector**: Gate activation flags

## Development

```bash
cargo test
cargo bench
```

## References

- [PSE Halo2 Book](https://zcash.github.io/halo2/)
- [Scroll's Halo2 Usage](https://github.com/scroll-tech/halo2)
