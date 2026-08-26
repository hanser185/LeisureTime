# Repository Guidelines

## Project Structure

- `src/`: Vue 3 + TypeScript frontend. Reusable components live in `components/`, windows in `views/`, and shared state in `stores/`.
- `src-tauri/`: Rust core and desktop entry point. Key modules are `activity.rs`, `state.rs`, `scheduler.rs`, `storage.rs`, `tray.rs`, and `commands.rs`.
- `doc/`: PRD, architecture, testing, review, and packaging documents.
- `scripts/`: icon and green portable packaging helpers.
- `.github/`: CI workflow and PR template.

Generated directories include `dist/`, `node_modules/`, `src-tauri/target/`, and `src-tauri/target-*/`.

## Build, Test, and Development Commands

From the repository root:

- `npm ci`: install pinned frontend dependencies.
- `npm run dev`: start the Vite development server.
- `npm run type-check`: run `vue-tsc --noEmit`.
- `npm run build`: build the frontend into `dist/`.
- `npm run tauri dev`: build and run the desktop application.

From `src-tauri/`:

- `cargo fmt --all -- --check`: verify Rust formatting.
- `cargo clippy --all-targets -- -D warnings`: lint with warnings treated as errors.
- `cargo test`: run Rust unit tests.

CI runs the frontend checks followed by formatting, clippy, and tests on every push or pull request to `main`.

## Coding Style & Naming Conventions

- Rust follows `rustfmt`; run `cargo fmt` before committing.
- Frontend uses TypeScript strict mode and scoped component styles.
- Vue components use PascalCase filenames and explicit `props`/`emits` types.
- Always handle `invoke(...)` errors and keep lock scopes short.
- Keep Windows-only Rust logic behind `#[cfg(windows)]`.

## Testing Guidelines

- Add or update `#[cfg(test)]` cases for pure logic changes in Rust.
- Run `cargo test` from `src-tauri/`.
- There is no separate frontend test framework; CI verifies frontend changes with `npm run type-check` and `npm run build`.

## Commit & Pull Request Guidelines

- Use Conventional Commits: `feat`, `fix`, `docs`, `refactor`, `test`, `style`, `build`, or `chore`, with an optional scope such as `(ui)`, `(tauri)`, or `(rust)`.
- Use branch names like `feature/xxx`, `fix/xxx`, and `hotfix/xxx`; merge with squash.
- Fill `.github/PULL_REQUEST_TEMPLATE.md`, link the related issue, keep a PR under 400 changed lines, and include screenshots for UI changes.
- Require all CI checks to pass before merge.

## Security & Configuration Tips

- User data stays local under `%LOCALAPPDATA%\rest-reminder\`.
- Do not add network calls, data uploads, secrets, certificates, `.env`, `node_modules/`, `dist/`, or `src-tauri/target*/` to the repository.
- See `doc/09-代码审查标准与流程.md` for the detailed review checklist.
