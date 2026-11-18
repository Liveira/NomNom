# Contributing to NomNom

Thanks for wanting to help. NomNom is a small project, so the process is small too.

## Ways to help

- **Report a bug.** Use the [bug report form](https://github.com/Liveira/NomNom/issues/new?template=bug_report.yml). Say your system (Windows 11, Arch, Ubuntu, desktop environment) and what you did.
- **Suggest something.** Use the [feature request form](https://github.com/Liveira/NomNom/issues/new?template=feature_request.yml). Explaining the problem helps more than describing a solution.
- **Translate.** All app text lives in `apps/desktop/src/lib/i18n/catalog/`. Each entry has three texts: English, Português (Brasil) and Español. A new language needs one more column and a matching entry in the Rust side (`apps/desktop/src-tauri/src/i18n.rs`).
- **Fix something.** Issues labelled [`good first issue`](https://github.com/Liveira/NomNom/labels/good%20first%20issue) are a good start. For anything bigger, open an issue first so we can agree on the approach before you spend time on it.

## Setting up

You need Rust (stable) and Node.js 20 or newer. The platform packages are listed in the [README](README.md#build).

```bash
git clone https://github.com/Liveira/NomNom.git
cd NomNom
npm install
npm run dev
```

## Before you open a pull request

Run the same checks CI runs:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm run check
npm test
```

Keep each pull request about one thing, and say in the description what you changed and how you tried it. If it changes how something looks, a screenshot helps.

## Rules the code follows

These keep NomNom safe to run on real files. A pull request that breaks them will be asked to change.

- **Library crates never depend on Tauri.** Everything under `crates/` is plain Rust. Tauri code lives only in `apps/desktop/src-tauri`.
- **No shell.** Programs are started through `nomnom_core::process::spawn` with an argument list, never through a shell string.
- **Never overwrite, never guess.** Anything that moves or creates a user's file uses `nomnom_core::fs::move_no_replace` or an equivalent that picks a free name. Anything that deletes shows a plan first and goes to the Trash or Recycle Bin by default.
- **Tests never touch real folders.** Use `tempfile`. Code that changes files must be tested in a throwaway directory.
- **Text is plain and direct.** Write short sentences that say what happens. No marketing words. A test scans the interface and Rust messages for them.
- **Every visible string is translated.** Add the English text, then the Portuguese and Spanish ones, in the catalog. `npm test` checks that nothing is missing.
- **Contrast and keyboard.** New screens must be reachable and usable with the keyboard and keep the text contrast tests passing, in every theme.

## Adding a tool

A tool has three parts: a library crate under `crates/` (no Tauri), a thin layer in `apps/desktop/src-tauri/src/` that exposes it as commands and checks the input again, and a Svelte view under `apps/desktop/src/modules/`. It is registered in `apps/desktop/src-tauri/src/modules/builtin.rs`. File Tools (`crates/nomnom-converter`, `filetools.rs`, `modules/filetools`) is a complete small example to copy from.

## Commits

Short, present-tense summaries, with a scope when it helps: `feat(storage): …`, `fix(organizer): …`, `docs: …`. One logical change per commit.

## License

By contributing you agree that your work is released under the [MIT License](LICENSE).
