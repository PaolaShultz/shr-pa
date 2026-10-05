# C-PA v2 provider corpus

These strict JSON fixtures are deserialized and rendered by normal `graph_v2`
tests, not only schema examples. stereo3way is actual 2×6 LR24; stereo4way is
actual 2×8 LR24. matrix4x8 explicitly uses L=.5×input0+.25×input2 and
R=.75×input1−.5×input3, feeding the same four-way topology. No normalization.

From the repository root, using the shared nonblocking build slot on the lab host:

```sh
flock -xn /home/shome/p/.gigpies-build.lock env CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.97.1 test --locked -j1 --test graph_v2 --test render_allocation
flock -xn /home/shome/p/.gigpies-build.lock env CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.97.1 build --release --locked -j1
mkdir -p artifacts/cpa-v2
flock -xn /home/shome/p/.gigpies-build.lock cc -std=c11 -Wall -Wextra -Werror -Isrc tests/fixtures/cpa/v2/caller.c -Ltarget/release -Wl,-rpath,"$PWD/target/release" -lshr_pa -lm -o artifacts/cpa-v2/caller
artifacts/cpa-v2/caller tests/fixtures/cpa/v2/stereo3way.json
artifacts/cpa-v2/caller tests/fixtures/cpa/v2/stereo4way.json
artifacts/cpa-v2/caller tests/fixtures/cpa/v2/matrix4x8.json
```

No audio device, network listener or physical route is opened. Runtime binaries
stay ignored. The host owns library lifetime and physical mapping; JSON vector
indices are logical module ports. [Full contract](../../../../docs/EMBEDDING_V2.md).
