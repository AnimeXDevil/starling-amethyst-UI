# Contributing to Starling

Thanks for helping. Starling is the Android port of [Horizon](https://github.com/ludovic111/horizon), Minecraft: Java Edition 26.3 rewritten in Rust. This page is the short version of how we work.

## Where a change goes

- **The game itself** (gameplay, world generation, rendering shared with the desktop, menus) belongs upstream, in Horizon, where it is checked against vanilla. Follow [Horizon's contributing guide](https://github.com/ludovic111/horizon/blob/main/CONTRIBUTING.md): vanilla is the specification, and gameplay changes come with a parity harness.
- **The Android app** belongs here: the packaging (`packaging/android/`), the app's entry (`crates/horizon-client/src/android_lib.rs`), touch controls (`touch.rs`, `game/touch.rs`), the Starling folder and imports (`import.rs`, `game/imports.rs`, `game/files_offer.rs`) and anything that only matters on a phone.

**Mod-related pull requests go to [arismoko/horizon](https://github.com/arismoko/horizon)**, the fork that carries the
mod API and reviews and merges that work. That covers the mod API, native hooks and events for mods, native mods,
content mods (`.hmod`), mixins, wasm mods, `cargo horizon-mod`, `horizon-mods`, mod builds and the registry, and
their docs ([mods](docs/mods.md), [native mods](docs/native-mods.md)). Open the pull request there from your usual
branch. Mod-related pull requests opened here are closed with a pointer to it. Engine extension points that Horizon's
own crates register (the [layer split](docs/layers.md)), resource and shader pack formats, and gameplay fixes stay
here.

## Build it

<<<<<<< HEAD
You need Rust (stable) with the `aarch64-linux-android` target, [cargo-ndk](https://github.com/bbqsrc/cargo-ndk), the Android SDK (build-tools 35.0.0, platform 36, NDK r28c), a JDK 25 and Python 3.10+.
||||||| 980f0459eb
- **Prerequisites:** Rust (stable), Python 3.10+, JDK 25, and on Linux the ALSA and udev headers
  (`libasound2-dev libudev-dev`).
- **Resources:** Horizon never downloads the game; it prepares its resources from your own copy of Minecraft 26.3, as
  described in [Build from source](README.md#build-from-source).
- **Commands:**
=======
- **Prerequisites:** Rust (stable), Python 3.11+, JDK 25, git 2.38 or newer, and on Linux the ALSA and udev headers
  (`libasound2-dev libudev-dev`).
- **Resources:** Horizon never downloads the game; it prepares its resources from your own copy of Minecraft 26.3, as
  described in [Build from source](README.md#build-from-source).
- **Commands:**
>>>>>>> arismoko/main

```sh
python packaging/android/build_apk.py            # build the library, then the APK
python packaging/android/build_apk.py --install  # and install it over adb
```

The APK carries no game files. On its first start the app signs in, downloads them from Mojang for an account that owns the game and prepares them on the phone; to skip that while testing, prepare the resources from your own copy of Minecraft 26.3 as described in [Build from source](README.md#build-from-source), zip them as a `.starling` file and open it with the app. The setup's hand-drawn skin comes from `python packaging/lowbudget/make.py` (Pillow): run it after changing `hand.py`, `make.py` or `strings.json`, and commit `pack/` with them.

## Test on a phone

Say what you checked on a real device, and on which one. A `horizon-args.txt` in the app's own files adds command-line arguments and environment variables at launch (one per line, `NAME=value` for the environment); with `--control-socket` and `HORIZON_CONTROL_PORT`, `horizon-ctl` drives the game over `adb forward`. Empty the file afterwards so the app starts normally.

## Only original code, or code whose license allows it

Starling is MIT, as Horizon is. Its code is written from Minecraft 26.3 itself and from public documentation. Another project's source may also be read and used, **only when that project's license allows it**:

- **Allowed:** MIT, MIT-0, BSD-2-Clause, BSD-3-Clause, 0BSD, ISC, zlib, Apache-2.0, Unlicense, and the CC0-1.0 public-domain dedication. That code may be read, copied or adapted.
- **Java's own library is GPL too.** OpenJDK's source (`java.lang.Math`, `StrictMath`, `FdLibm`, `HashMap`, the sorts, `Double.toString`, HotSpot's intrinsics...) is GPL-2.0, so it is off limits like the rest. Reproduce Java's behaviour from its API documentation (the Javadoc) and from what a JVM returns, or from a permissive original such as Sun's fdlibm C sources (see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)); see #6741.
- **Everything else is off limits, even to read:** GPL, LGPL (Iris), AGPL, MPL, EPL, Polyform (Sodium), the Creative Commons licenses (CC BY, CC BY-SA, CC BY-NC and the others; CC0 is a public-domain dedication, not one of them), custom or "source available" licenses, all rights reserved (OptiFine), and any project with no license or an unclear one. Don't read, copy, translate or paraphrase their source, and don't send code an AI tool wrote from it. Being able to download a mod or a pack does not give you the right to read its files.
- **Check the license yourself.** Read the LICENSE file at the commit you use and the header of each file you take: some repositories mix licenses, and a file under another license is off limits. A license that changed: the one at your commit counts. In doubt, don't: ask in an issue first.
- **Say it in the pull request:** the project, a link to the commit, its license, and what you read or copied.
- **Copied or adapted code** keeps its copyright line and license notice at the top of the file, and gets an entry in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md). Apache-2.0 code also keeps its NOTICE text and says what was changed.

User documentation and file formats are fine to read, and packs and mods are fine to run and observe, whatever their license.

## Before a pull request

```sh
git config core.hooksPath .githooks
python3 tools/pr_ready.py
```

<<<<<<< HEAD
It runs rustfmt, clippy with warnings denied, the tests of the crates you touched, the Python tests and the public-tree check. Keep pull requests small, and write commit messages in English.
||||||| 980f0459eb
It looks at what your branch changes and runs what matters: rustfmt, clippy with warnings denied, the tests of the
crates you touched (and the launcher's), generated files, `Cargo.lock`, the Python tests, and the vanilla harnesses
your change can affect. Without the vanilla data on your machine it lists those harnesses instead; CI runs them.
`--quick` skips the harnesses. It exits non-zero when something fails.
=======
It looks at what your branch changes and runs what matters: rustfmt, clippy with warnings denied, the tests of the
crates you touched (and the launcher's), generated files, `Cargo.lock` and the Python tests. Like CI, it runs the
vanilla harnesses only when your branch changes a harness, an oracle, a probe or the runner; otherwise it lists the
ones your change can affect, and main's parity run checks them after the merge. `--harnesses` runs them on your
machine anyway, and a change to mod content, assets, blocks or items also runs the `#[ignore]`d content-root tests;
both need the prepared vanilla data and are reported as not run without it. `--quick` never runs the harnesses.
It exits non-zero when something fails.

### CI tiers

CI is split by how long a test takes and what it needs. Main and rollups run only quick checks and unit tests;
every test that boots the game runs once a day. Existing tests that need prepared resources, a child process or the
network and are still in `verify` are being moved by the next change of #4022.

| Tier | Runs on | What |
| --- | --- | --- |
| Push | every pull request push | `format`, `verify-static` |
| Gate | rollup and sync pull requests, pushes to main | `lint`, `verify` (unit tests only), `verify-static`, `launcher` |
| Daily | once a day on main's head | every test that boots the game or needs prepared resources, the wasm fixture, `fn-replace-check`, the SDK build and its linkage probe, the machine sample built against the mod API |

**A unit test** uses no prepared resources, boots no game, starts no child process, uses no network and no compiled
wasm fixture, and finishes in under 2 seconds. Any other test is a daily test: it carries `#[ignore]` and a row in
`tools/ci/ignored-tests/` (the format is in that folder's README). `verify` fails a unit test that takes over 5
seconds and `verify-static` one that boots the game; existing tests over 2 seconds and those that boot the game are
already daily tests, and the existing ones that need resources or a child process are being moved over.
In Linux `verify`, test binaries run normally under an inet guard: opening, binding or connecting an IPv4/IPv6
socket or calling libc name-resolution APIs (`getaddrinfo`, `gethostbyname`/`gethostbyname2`/`gethostbyaddr`, their reentrant
variants, or `getnameinfo`) fails the test by its full name, even if it handles the refusal. Unix sockets remain
allowed. Only binaries recording violations are rerun per test to name the culprits; an unreproducible violation
still fails by binary name. Move a network test with `tools/ci/move_to_daily.py`, using reason `network`; daily runs without the guard.

- A pull request is reviewed and merged on the push and gate tiers. Nothing else is asked of you.
- Write a quick test where a quick test can catch the mistake. A test that needs the game or prepared resources is
  welcome: mark it `#[ignore]` with its policy row, and say in the pull request that it has not run if you could not
  run it.
- If a daily run fails on a test your pull request added or touched, an issue is filed that names the test, where it
  panicked and the pull requests merged since the last green daily run. The fix is an ordinary pull request.

Three kinds of build: a **snapshot**, from main's head on request, needs only the gate tier and no in-game tests
(it is published as `snapshot-<date>-<short>`, which no launcher offers, to test packages and platforms after a
merge); the **nightly** is published by the daily run when every part of it is green; a **release** is cut from a
nightly.

Reviews on this fork read the diff, the design and CI; they do not run your change. So for anything a player or a
mod author can see, please start the game once and try it (single player is enough:
`cargo build --profile fast -p horizon-game`, or `horizon-ctl` for agents) and say in one line what you did, under
"Tried in game" in the template. It is not a gate: "no, could not run it here" is an acceptable answer, and it tells
the reviewer where to look harder.
>>>>>>> arismoko/main

For the game workspace, Clippy checks all targets of the touched crates and their transitive workspace dependents, including examples. The dependency scope includes optional, platform, build and development dependencies. Tests run library, binary, integration and doc tests without building unrelated example executables. Shared Cargo configuration and toolchain changes select the whole workspace; CI also keeps its workspace-wide checks.

<<<<<<< HEAD
Never commit game files, decompiled source or prepared resources (`.local/`): `tools/check_public_tree.py` rejects them (`git config core.hooksPath .githooks` runs it before every commit). The pre-push hook checks the full history reachable from the branches and tags being pushed; `python3 tools/check_public_tree.py --history` audits the history of all local refs.
||||||| 980f0459eb
### Translations

Most translations are machine-written and remain unreviewed until a speaker checks them. See [docs/translations.md](docs/translations.md). Never mark machine-written text as reviewed.

## Claim before you start

Work is tracked in GitHub issues, and an issue that is assigned is taken (so is one with the `in progress` label:
claimed on Discord with Kira's `/claim`): pick another, or ask its holder in the thread. Before you start, comment
**`/claim`** on the issue; you are assigned at once, no role needed. No issue for your work yet? Open one (a line is
enough), then claim it. `/unclaim` gives it back. Issues labelled `tracker`, `proposal` or `not claimable` aren't
claimed whole: claim one of the issues a tracker lists, or open one for the part you take.

A claim with no open pull request linking the issue (`Fixes #123` in the description, or a draft) is released after
5 days, with a comment: open a draft early. Agents claim the same way.

You can hold at most 5 claimed issues that no open pull request of yours links. A sixth `/claim` is refused with a
comment that gives the count; opening a pull request for one of them, or `/unclaim` on one, frees a slot. The
repository's collaborators, and the trusted contributors listed in the repository variable `CLAIM_LIMIT_EXEMPT`, are
not limited.

When your pull request changes the same functions as another open one, or closes the same issue, a bot comment names
it (edited on every push, never repeated): talk to its author in the thread before you both finish the same work.

## Pull requests

- **One change.** One concern per pull request (a hook, a handle, a feature or a fix), at whatever size it
  needs. There's no line limit; split by concern, never by size, and don't bundle unrelated changes.
- **Fill in the template**: what and why, the vanilla reference, the pr_ready block, screenshots for anything you
  can see.
- **Tick "Allow edits by maintainers".** When something small is left (a lint, a rebase, a nit) we push the fixup
  ourselves instead of sending it back to you; only design questions come back.
- **Once we've pushed to your branch, merge main into it; don't rebase or force-push.** A rebase or force-push
  replaces our commits with yours and erases the fixes we added (it happened on #5188 and #5203). Tell your agent
  too: `git merge origin/main`, never `git rebase` or `git push --force` on a branch a maintainer has touched. If
  our fixes get erased anyway, we finish the work in our own pull request that keeps your commits and credits you.
- **Tick its kind** in the template: `parity` (Horizon now does what vanilla does), `bug` (a crash or breakage),
  `perf`, `feature` (new for players), `extra` (Horizon-only, off by default), `tooling` or `docs`. A workflow sets
  that label, and area labels (`launcher`, `mobs`, `worldgen`, `rendering`...) from the paths you change.
  `label:parity` is the list of real progress toward vanilla.
- **Release labels** (we set them if you can't): `feature` also puts it under "New" in the release notes,
  `skip-release` when players see no change (tooling, docs, tests), nothing for a fix. `release-now` is for crash
  fixes that shouldn't wait. pr_ready suggests one.

Who reviews and merges: the Horizon Team, on CI for ordinary pull requests; risky ones (saves, network, world
generation, the launcher's updates) wait for a local proof first ([merging](docs/release-flow.md#merging)).

Every merge to main is released automatically ([release flow](docs/release-flow.md)), so main always stays playable.
How agents, checks, releases and signed updates keep this safe: [security](docs/security.md).

### Large pull requests

500 lines stays the default. A pull request that can't be split below it starts from a spec:

1. **Open an issue with the spec** before the code: what changes, the files it touches, the behaviour players and
   other code will see, and the tests or harnesses that prove it.
2. **A maintainer approves the spec** in the issue. Code written before that may have to change or wait.
3. **The pull request links the approved issue** (`Spec: #123` in the description) and stays within it; a change of
   plan goes back to the issue first.
4. **An agent check compares the code with the spec**, run by the maintainers' agents: anything the spec doesn't
   cover, or that it promises and the code doesn't do, comes back to you.
5. **Then the normal review**, as for any pull request.

## Parity checks on forks

The `verify` check (formatting, lints, tests) runs on every pull request. The `parity` check runs the affected
harnesses on a maintainer's machine, which holds the game files, so it never runs a fork's code on its own: a
maintainer reads your change, then adds the `run-parity` label, which runs it once on your current commit. After
new pushes, we add it again.

## Talk to us

[Discord](https://discord.gg/playhorizon) for questions and ideas, [issues](https://github.com/horizon-rust/horizon/issues)
for bugs. To take something on, `/claim` its issue ([above](#claim-before-you-start)).

How Horizon is run, and how people join the Team or lead a group: [GOVERNANCE.md](GOVERNANCE.md).
=======
### Translations

Most translations are machine-written and remain unreviewed until a speaker checks them. See [docs/translations.md](docs/translations.md). Never mark machine-written text as reviewed.

## Claim before you start

Work is tracked in GitHub issues, and an issue that is assigned is taken (so is one with the `in progress` label:
claimed on Discord with Kira's `/claim`): pick another, or ask its holder in the thread. Before you start, comment
**`/claim`** on the issue; you are assigned at once, no role needed. No issue for your work yet? Open one (a line is
enough), then claim it. `/unclaim` gives it back. Issues labelled `tracker`, `proposal` or `not claimable` aren't
claimed whole: claim one of the issues a tracker lists, or open one for the part you take.

A claim with no open pull request linking the issue (`Fixes #123` in the description, or a draft) is released after
5 days, with a comment: open a draft early. Agents claim the same way.

You can hold at most 5 claimed issues that no open pull request of yours links. A sixth `/claim` is refused with a
comment that gives the count; opening a pull request for one of them, or `/unclaim` on one, frees a slot. The
repository's collaborators, and the trusted contributors listed in the repository variable `CLAIM_LIMIT_EXEMPT`, are
not limited.

When your pull request changes the same functions as another open one, or closes the same issue, a bot comment names
it (edited on every push, never repeated): talk to its author in the thread before you both finish the same work.

## Pull requests

- **One change.** One concern per pull request (a hook, a handle, a feature or a fix), at whatever size it
  needs. There's no line limit; split by concern, never by size, and don't bundle unrelated changes.
- **Fill in the template**: what and why, the vanilla reference, the pr_ready block, screenshots for anything you
  can see.
- **We never push to your branch on this fork.** When something small is left (a lint, a rebase, a nit) we carry
  it in our own commits on top of yours in a rollup, and your pull request shows as merged with your commits
  intact; only design questions come back to you. You don't need to tick "Allow edits by maintainers".
- **Keep your branch yours.** You can push to it as you like. If you want to bring it up to date, `git merge
  origin/main` is the safe way; a rebase or force-push is fine too, since nothing of ours lives on your branch.
- **Title: `kind(area): what changes`**, so the queue reads at a glance: `parity(mobs): a ghast's fireball uses its
  ExplosionPower`, `perf(launcher): download the game's files in parallel`, `bug(saves): keep /attribute modifiers
  through a save`. The kind is one of the words below; the area is an area label, several with commas
  (`perf(chunks,rendering): ...`), or leave it out (`docs: ...`). Release notes drop the prefix.
- **Tick its kind** in the template: `parity` (Horizon now does what vanilla does), `bug` (a crash or breakage),
  `perf`, `feature` (new for players), `extra` (Horizon-only, off by default), `tooling` or `docs`. A workflow sets
  that label from the box, else from the title, and area labels (`launcher`, `mobs`, `worldgen`, `rendering`...)
  from the title and the paths you change.
  `label:parity` is the list of real progress toward vanilla.
- **Release labels** (we set them if you can't): `feature` also puts it under "New" in the release notes,
  `skip-release` when players see no change (tooling, docs, tests), nothing for a fix. `release-now` is for crash
  fixes that shouldn't wait. pr_ready suggests one.

Who reviews and merges: the Horizon Team, on CI for ordinary pull requests; risky ones (saves, network, world
generation, the launcher's updates) wait for a local proof first ([merging](docs/release-flow.md#merging)).

Merges reach players through the nightly, which the daily run publishes only when every in-game test passed
([release flow](docs/release-flow.md)).
How agents, checks, releases and signed updates keep this safe: [security](docs/security.md).

### Large pull requests

500 lines of code ([what counts](#pull-requests)) stays the default. A pull request that can't be split below it
starts from a spec:

1. **Open an issue with the spec** before the code: what changes, the files it touches, the behaviour players and
   other code will see, and the tests or harnesses that prove it.
2. **A maintainer approves the spec** in the issue. Code written before that may have to change or wait.
3. **The pull request links the approved issue** (`Spec: #123` in the description) and stays within it; a change of
   plan goes back to the issue first.
4. **An agent check compares the code with the spec**, run by the maintainers' agents: anything the spec doesn't
   cover, or that it promises and the code doesn't do, comes back to you.
5. **Then the normal review**, as for any pull request.

## Parity checks on forks

A pull request from a fork gets the push tier ([CI tiers](#ci-tiers)); the gate runs when a rollup carries it. The
`parity` check runs the affected
harnesses on a maintainer's machine, which holds the game files, so it never runs a fork's code on its own: a
maintainer reads your change, then adds the `run-parity` label, which runs it once on your current commit. After
new pushes, we add it again.

## Talk to us

[Discord](https://discord.gg/playhorizon) for questions and ideas, [issues](https://github.com/horizon-rust/horizon/issues)
for bugs. To take something on, `/claim` its issue ([above](#claim-before-you-start)).

How Horizon is run, and how people join the Team or lead a group: [GOVERNANCE.md](GOVERNANCE.md).
>>>>>>> arismoko/main
