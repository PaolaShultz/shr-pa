# Maintained scripts

`publication-policy.json` lists the reviewed scripts and Git hooks. Keep one-off
bench/session runners and generated output under ignored `artifacts/`.

| Script | Purpose |
|---|---|
| `check-docs.py` | Local documentation, SVG and application-version checks |
| `check-terminal.py` | Normal offline terminal regression |
| `check-live-controls.py` | Normal controls regression using software-null PCM |
| `check-live.py` | Opt-in hardware smoke test; requires explicit device authorization |
| `render-docs.py` | Opt-in documentation artwork renderer |
| `check_publication.py` | Staged Git and outgoing-commit publication checks |
| `test_publication.py` | Synthetic publication regressions in temporary repositories |

See [validation](../docs/VALIDATION.md) and [publication rules](../docs/PUBLICATION.md).
