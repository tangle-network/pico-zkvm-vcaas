
In trading-volumn folder

**Build program**
```shell
cd app
RUST_LOG=info cargo pico build 
RUST_LOG=debug cargo pico prove
```

Update this constant to change the trading volumn receipts workload.

```
const MAX_RECEIPT: usize = 64;
```