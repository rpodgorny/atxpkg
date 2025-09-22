test:
  cargo nextest run

coverage:
  cargo llvm-cov nextest --text

mutants:
  cargo mutants --test-tool=nextest -j 4

updeps:
  cargo upgrade --verbose
  cargo update -v --recursive
  cargo outdated
