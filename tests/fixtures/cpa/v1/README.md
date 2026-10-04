# C-PA:1 / E08 provider corpus

Owned by SHR PA. `expected.json` describes the exact fixed descriptor and health
states. `caller.c` is a synthetic actual C caller; it opens no devices. Header and
library are supplied by this repository, never copied algorithms in a consumer.

Reproduce from the repository root using the pinned toolchain:

```sh
CARGO_INCREMENTAL=0 cargo +1.97.1 test --locked -j1 --test ffi
CARGO_INCREMENTAL=0 cargo +1.97.1 build --release --locked -j1
mkdir -p artifacts/cpa
cc -std=c11 -Wall -Wextra -Werror -Isrc tests/fixtures/cpa/v1/caller.c \
  -Ltarget/release -lshr_pa -lm -Wl,-rpath,"$PWD/target/release" \
  -o artifacts/cpa/caller
artifacts/cpa/caller
```

Exact delivery source/header/corpus/library SHA-256 values belong in the private
provider manifest after build and review. No library binary is tracked here.
Synthetic acceptance does not establish hardware or calibrated speaker safety.
