# Guarded MetaHarness development adaptation

This iteration executes the real `@metaharness/darwin` 0.10.2 numeric optimizer against a frozen development only field evaluator. Candidate genomes are data, not executable code. The wrapper independently validates proposals, rejects regressed or nonfinite scores, enforces the unchanged ten percent gate, and atomically retains or reverts a development pointer.

The protocol and source were committed before the reported run. No holdout seeds are present and no production promotion is possible.

```sh
npm ci --ignore-scripts --no-audit --no-fund
npm test
npm run adapt -- fresh-run-id
npm run verify
```

Every adaptation invocation requires a fresh run identifier. Reusing a populated MetaHarness work root is rejected.
