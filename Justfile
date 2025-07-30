test:
  cargo nextest run

mutants:
  cargo mutants --test-tool=nextest -j 4

updeps:
  cargo upgrade --verbose
  cargo update -v --recursive
  cargo outdated
