# Interactive Provider CLI Plan

## 1. Goals

- Add an `audetic provider` command group so users can inspect and reconfigure transcription providers without editing `config.toml` manually.
- Keep logic idiomatic to the current clap-based CLI (reuse `CliCommand` in `src/cli/mod.rs`).
- Enforce interactive usage only: if `stdin` is not a TTY, exit immediately with an info-level log telling users to edit `~/.config/audetic/config.toml` instead (per requirement—no stdin fallback).

## 2. Command Surface

- `audetic provider show`: loads `Config`, prints current `whisper.provider` plus relevant fields (mask API keys, show file paths when present).
- `audetic provider configure`: interactive wizard that writes the `[whisper]` section back via `Config::save()`.
- (Optional stretch) `audetic provider test`: instantiates `Transcriber::with_provider` using stored config to surface errors before users start a recording session.

## 3. Implementation Topology

- Extend `CliCommand` with `Provider(ProviderCliArgs)` and add `ProviderCommand` enum for `show | configure | test`.
- Create `src/cli/provider.rs` with helpers:
  - `ensure_interactive()?` (uses `std::io::stdin().is_terminal()` or `atty::is(Stream::Stdin)` to gate `configure`; on failure, log info and return `Err(anyhow!(...))` so main exits gracefully).
  - `prompt_provider_selection(...)` using `dialoguer::Select`.
  - `prompt_api_key(...)`, `prompt_command_path(...)`, etc., each validating input and re-prompting.
- Update `Cargo.toml`: add `dialoguer` (and `console` if password masking is desired). Consider `which` crate for binary detection.
- Config handling:
  - Extend `Config` with helper `fn update_provider(&mut self, updates: WhisperConfigUpdate)` to keep mutation localized.
  - Mask API keys when printing by reusing `dialoguer::Password`, or custom `mask_secret` helper.

## 4. Interactive Flow (`configure`)

1. `ensure_interactive()`; if false, log `info!("Non-interactive session detected; edit ~/.config/audetic/config.toml manually to change providers.")` and return early.
2. Load config (`Config::load()`), clone `whisper` section for mutation.
3. Show current provider summary.
4. Present provider menu with inline descriptions:
   - `audetic-api` – default cloud provider; requires API key.
   - `openai-api` – OpenAI Whisper API.
   - `openai-cli` – local `openai-whisper` binary.
   - `whisper-cpp` – local whisper.cpp binary.
5. After selection, run provider-specific prompts (see §5). Validate each answer immediately (non-empty API keys, `Path::new(&command_path).exists()` etc.).
6. Ask for shared options (language, model) with sensible defaults pulled from current config.
7. Write updates into `config.whisper` and call `config.save()`.
8. Print success summary + reminder to restart the Audetic service if needed (`systemctl --user restart audetic.service`).

## 5. Provider-Specific Prompt Sets

- `audetic-api`
  - Free transcription service from audetic. No api key required.
- `openai-api`
  - Required: API key (masked).
  - Model selector (default `whisper-1`), language (`auto`, `en`, etc.).
  - Optional endpoint override.
- `openai-cli`
  - Command path (`which whisper` as default; allow manual override).
  - Model preset list (`tiny`, `base`, `small`, `medium`, `large-v3`, `large-v3-turbo`).
- `whisper-cpp`
  - Command path (validate executable).
  - Model path (`*.bin`, ensure file exists).
  - Model size dropdown for reference (still stored as `model` string).

## 6. Non-TTY Behavior (Required)

- Detection: `use std::io::IsTerminal; if !std::io::stdin().is_terminal()` (available since Rust 1.70) or `atty`.
- When not interactive:
  - Log at `info!`: `"Non-interactive CLI session detected. Please edit ~/.config/audetic/config.toml manually to change providers."`
  - Return a user-facing error so clap exits with code 1. Do **not** attempt to read from stdin or fall back to defaults.

## 7. Testing Strategy

- Unit tests for helper validation (e.g., command-path validator, config update function).
- CLI tests with `assert_cmd` to ensure:
  - `audetic provider show` prints masked keys.
  - `audetic provider configure` exits with informative message when run via piped stdin (non-tty simulation).
- Manual smoke test for each provider to ensure prompts write expected fields to `~/.config/audetic/config.toml`.

## 8. Documentation Updates

- `docs/configuration.md`: add a short “Using the CLI to change providers” section with examples (`audetic provider configure` and expected prompts).
- `README.md`: mention the new command in setup instructions.
- Optionally link from `docs/adding-providers.md` describing how interactive wizard surfaces new providers (update menu text when new providers are added).

## 9. Work Breakdown

1. Wire new clap subcommands and provider handler scaffolding.
2. Add `dialoguer` + `which` dependencies and helper utilities (masking, path validation).
3. Implement `provider show`.
4. Implement `provider configure` interactive flow + non-tty guard.
5. (Optional) Implement `provider test`.
6. Update docs and example snippets; run fmt/clippy/tests.
