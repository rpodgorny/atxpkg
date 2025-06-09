test:
  cargo nextest run

updeps:
  cargo upgrade --verbose
  cargo update -v --recursive
  cargo outdated
