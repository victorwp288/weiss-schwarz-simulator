# Changelog

This file is maintained by Release Please.

## [0.8.2](https://github.com/victorwp288/weiss-schwarz-simulator/compare/v0.8.1...v0.8.2) (2026-04-28)


### Bug Fixes

* add heuristic public profiles ([2cacacb](https://github.com/victorwp288/weiss-schwarz-simulator/commit/2cacacb2b480377c95b826cc593fb6f987c06d0e))

## [0.8.1](https://github.com/victorwp288/weiss-schwarz-simulator/compare/v0.8.0...v0.8.1) (2026-04-20)


### Bug Fixes

* **ci:** recover benchmark publishing after release merge ([f5ccb77](https://github.com/victorwp288/weiss-schwarz-simulator/commit/f5ccb77418b5f8942a36146a6ec37b345fae00e4))

## [0.8.0](https://github.com/victorwp288/weiss-schwarz-simulator/compare/v0.7.0...v0.8.0) (2026-04-20)


### Features

* finalize simulator release prep ([3653635](https://github.com/victorwp288/weiss-schwarz-simulator/commit/36536358934dfba0348d77574a512eacab2a7da5))


### Bug Fixes

* **ci:** avoid rustdoc ICE in docs publish ([bf96294](https://github.com/victorwp288/weiss-schwarz-simulator/commit/bf962942f348861337ec3d428d520bc3958348f7))
* **ci:** format perf budget checker ([de21404](https://github.com/victorwp288/weiss-schwarz-simulator/commit/de21404c1f300133be01ba8111cf662a310a8b96))
* **ci:** satisfy rust 1.95 clippy ([3febfa4](https://github.com/victorwp288/weiss-schwarz-simulator/commit/3febfa4c06494d8c7f63ad8207fa36ad896d0008))
* **ci:** stabilize reset perf budget gate ([286fca2](https://github.com/victorwp288/weiss-schwarz-simulator/commit/286fca2a8fd990cc675dc798a8c2e4e5e9f3f660))


### Miscellaneous Chores

* **release:** trigger release-please for 0.8.0 ([7b2cc2e](https://github.com/victorwp288/weiss-schwarz-simulator/commit/7b2cc2e2504ae7efab855e5acf54de877cf0bbd4))

## [0.7.0](https://github.com/victorwp288/weiss-schwarz-simulator/compare/v0.6.0...v0.7.0) (2026-02-23)


### Features

* add beginner guide and finalize remaining updates ([d40bf44](https://github.com/victorwp288/weiss-schwarz-simulator/commit/d40bf44b237cee74a13e4fc385d7e6ea1d32499c))
* improve Python API ergonomics and prepare v0.7.0 ([db073a0](https://github.com/victorwp288/weiss-schwarz-simulator/commit/db073a0eae0e43121dc416439c1a7285409ec225))

## [0.6.0](https://github.com/victorwp288/weiss-schwarz-simulator/compare/v0.4.0...v0.6.0) (2026-02-22)


### Features

* align project for v0.6.0 release ([6f58988](https://github.com/victorwp288/weiss-schwarz-simulator/commit/6f5898886510d4d7b4a736a9b3bc9a1b8bf7ffc0))
* formalize RL contract and expand bindings ([8fc39b7](https://github.com/victorwp288/weiss-schwarz-simulator/commit/8fc39b7a558961afbee750f5c7dca1f8054f3e3d))
* formalize RL contract and expand bindings ([44e9322](https://github.com/victorwp288/weiss-schwarz-simulator/commit/44e93227a708b6f6d031b8c742b6204d727e9891))
* **python:** expand RL status API and split pyo3 bindings ([e1b6156](https://github.com/victorwp288/weiss-schwarz-simulator/commit/e1b6156735a2912b05479fbe4edeb59ff74ba637))
* revamp python API and legal pipelines ([5a56e9f](https://github.com/victorwp288/weiss-schwarz-simulator/commit/5a56e9ff71232524292c997aca0ae3232f7b069f))
* **scraper:** add parser v2 rulepacks and coverage reporting ([10fc352](https://github.com/victorwp288/weiss-schwarz-simulator/commit/10fc352be7528d8724794271a37491669eb6f850))


### Bug Fixes

* align package versions with v0.1.1 ([5c3cacf](https://github.com/victorwp288/weiss-schwarz-simulator/commit/5c3cacfda4b899aefb8decae0d5fdca63e11b3ff))
* **catalog:** avoid stale db hash cache on same-mtime rewrites ([45f8789](https://github.com/victorwp288/weiss-schwarz-simulator/commit/45f878971675bd71576d1a43054b3d181c715e57))
* **ci:** resolve clippy/ruff failures and perf venv setup ([576da0b](https://github.com/victorwp288/weiss-schwarz-simulator/commit/576da0bf97eaac49a350de6e7d81a6090c9f2044))
* **perf-ci:** same-runner perf gating and hot-path optimizations ([40e5dc9](https://github.com/victorwp288/weiss-schwarz-simulator/commit/40e5dc962ca7c6b5ec3309a09dd7dc0259d40884))
* **py:** pass deck_lists as keyword in make_pool ([f930dc6](https://github.com/victorwp288/weiss-schwarz-simulator/commit/f930dc6c0a3fc2740ee9506e5ae88ee779713fe4))
* **pytest:** prefer wheel package when in-tree extension is absent ([348a3e5](https://github.com/victorwp288/weiss-schwarz-simulator/commit/348a3e5cf83bb0afa976c255c23aaae22af79cd6))
* repair wheels and benchmark workflows ([210de47](https://github.com/victorwp288/weiss-schwarz-simulator/commit/210de47ff0562e2767d4086cb5aec05aac177dcd))
* stabilize CI filters and wheel/bench workflows ([1532f31](https://github.com/victorwp288/weiss-schwarz-simulator/commit/1532f3142f0a8e8d7cb4862efc7cb7d8765174d3))
* update PyPI metadata ([57133db](https://github.com/victorwp288/weiss-schwarz-simulator/commit/57133db1fb9faebe4aa6a03d23fda5c5573c5a2d))


### Performance Improvements

* tune release and bench profiles ([7871aac](https://github.com/victorwp288/weiss-schwarz-simulator/commit/7871aac8bfa54b48c59cc35b7463c898149c0054))
* tune release and bench profiles ([9168341](https://github.com/victorwp288/weiss-schwarz-simulator/commit/9168341c4c6bd3124b387c27b804bc232aae2230))


### Miscellaneous Chores

* **release:** rebaseline manifest and force 0.6.0 [skip ci] ([fd36259](https://github.com/victorwp288/weiss-schwarz-simulator/commit/fd36259d5bee6adc278f112ba487f5fdb82f4953))
* **release:** trigger release-please for 0.4.0 ([f75d3c2](https://github.com/victorwp288/weiss-schwarz-simulator/commit/f75d3c291443b1ffc0cbf060a48a71b93389c840))
* **release:** trigger release-please for 0.6.0 ([9b3af26](https://github.com/victorwp288/weiss-schwarz-simulator/commit/9b3af26806e34721f43d7f14b7f46d7e0dfaa4cf))

## [0.4.0](https://github.com/victorwp288/weiss-schwarz-simulator/compare/v0.3.0...v0.4.0) (2026-02-18)


### Miscellaneous Chores

* **release:** trigger release-please for 0.4.0 ([f75d3c2](https://github.com/victorwp288/weiss-schwarz-simulator/commit/f75d3c291443b1ffc0cbf060a48a71b93389c840))

## [0.3.0](https://github.com/victorwp288/weiss-schwarz-simulator/compare/v0.2.1...v0.3.0) (2026-02-16)


### Features

* **python:** expand RL status API and split pyo3 bindings ([e1b6156](https://github.com/victorwp288/weiss-schwarz-simulator/commit/e1b6156735a2912b05479fbe4edeb59ff74ba637))
* **scraper:** add parser v2 rulepacks and coverage reporting ([10fc352](https://github.com/victorwp288/weiss-schwarz-simulator/commit/10fc352be7528d8724794271a37491669eb6f850))


### Bug Fixes

* **ci:** resolve clippy/ruff failures and perf venv setup ([576da0b](https://github.com/victorwp288/weiss-schwarz-simulator/commit/576da0bf97eaac49a350de6e7d81a6090c9f2044))
* **perf-ci:** same-runner perf gating and hot-path optimizations ([40e5dc9](https://github.com/victorwp288/weiss-schwarz-simulator/commit/40e5dc962ca7c6b5ec3309a09dd7dc0259d40884))

## [0.2.0](https://github.com/victorwp288/weiss-schwarz-simulator/compare/v0.1.3...v0.2.0) (2026-02-04)


### Features

* formalize RL contract and expand bindings ([8fc39b7](https://github.com/victorwp288/weiss-schwarz-simulator/commit/8fc39b7a558961afbee750f5c7dca1f8054f3e3d))
* formalize RL contract and expand bindings ([44e9322](https://github.com/victorwp288/weiss-schwarz-simulator/commit/44e93227a708b6f6d031b8c742b6204d727e9891))


### Performance Improvements

* tune release and bench profiles ([7871aac](https://github.com/victorwp288/weiss-schwarz-simulator/commit/7871aac8bfa54b48c59cc35b7463c898149c0054))
* tune release and bench profiles ([9168341](https://github.com/victorwp288/weiss-schwarz-simulator/commit/9168341c4c6bd3124b387c27b804bc232aae2230))

## [0.1.3](https://github.com/victorwp288/weiss-schwarz-simulator/compare/v0.1.2...v0.1.3) (2026-01-05)


### Bug Fixes

* update PyPI metadata ([57133db](https://github.com/victorwp288/weiss-schwarz-simulator/commit/57133db1fb9faebe4aa6a03d23fda5c5573c5a2d))

## [0.1.2](https://github.com/victorwp288/weiss-schwarz-simulator/compare/v0.1.1...v0.1.2) (2026-01-05)


### Bug Fixes

* align package versions with v0.1.1 ([5c3cacf](https://github.com/victorwp288/weiss-schwarz-simulator/commit/5c3cacfda4b899aefb8decae0d5fdca63e11b3ff))

## [0.1.1](https://github.com/victorwp288/weiss-schwarz-simulator/compare/v0.1.0...v0.1.1) (2026-01-04)


### Bug Fixes

* repair wheels and benchmark workflows ([210de47](https://github.com/victorwp288/weiss-schwarz-simulator/commit/210de47ff0562e2767d4086cb5aec05aac177dcd))
* stabilize CI filters and wheel/bench workflows ([1532f31](https://github.com/victorwp288/weiss-schwarz-simulator/commit/1532f3142f0a8e8d7cb4862efc7cb7d8765174d3))
