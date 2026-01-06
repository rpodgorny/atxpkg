test:
  cargo nextest run

coverage:
  cargo llvm-cov nextest --text

audit:
  cargo audit
  osv-scanner .

mutants:
  cargo mutants --test-tool=nextest -j 4

updeps:
  cargo upgrade --verbose
  cargo update -v --recursive
  cargo outdated
