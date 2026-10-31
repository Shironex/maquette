/**
 * No Tauri command runs on the main thread. A plain `#[tauri::command] fn`
 * does, and a Keychain or macOS folder privacy prompt then freezes the whole
 * app until the person answers. Every command is an `async fn` or has
 * `#[tauri::command(async)]`, and anything that can wait on the person uses
 * `commands::blocking`.
 */

/** A command attribute without `async`, then any other attributes, then its `fn`. */
const COMMAND =
  /#\[tauri::command\]\s*(?:#\[[^\]]*\]\s*)*(?:pub(?:\([a-z]+\))?\s+)?(async\s+)?fn\s+(\w+)/g;

/** The names of the commands in `source` that would run on the main thread. */
export function mainThreadCommands(source) {
  return [...source.matchAll(COMMAND)].filter(m => !m[1]).map(m => m[2]);
}

/** One violation per such command in the desktop crate. */
export function checkMainThreadCommands(ctx) {
  return ctx.glob('apps/desktop/src-tauri/src/**/*.rs').flatMap(file =>
    mainThreadCommands(ctx.read(file) ?? '').map(name => ({
      file,
      rule: 'commands-off-main-thread',
      message: `${name} runs on the main thread, where a Keychain or privacy prompt freezes the app. Make it async, or use #[tauri::command(async)].`,
    }))
  );
}
