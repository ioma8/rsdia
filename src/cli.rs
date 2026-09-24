//! rsdia — terminal ASCII diagram editor.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crossterm::event::{
    self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    Event, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use crossterm::execute;
use ratatui::DefaultTerminal;

use crate::core::export::{export_text, is_wrapper, Charset, ExportConfig};
use crate::core::layer::Layer;
use crate::core::text::text_to_layer;
use crate::storage::config::{load_config, save_config, Config};
use crate::storage::drawings::{DrawingStore, FILE_EXT};
use crate::tui::app::{App, AppOptions, Clipboard, OpenDrawing};
use crate::tui::theme::{detect_terminal_colors, ThemeName, PALETTE_WAIT_MS};

fn help(store: &DrawingStore) -> String {
    format!(
        "rsdia {} — terminal ASCII diagram editor

usage:
  rsdia                         open the last drawing (or \"untitled\")
  rsdia <name|path>             open or create a drawing
  rsdia --import diagram.txt    new drawing from a text file
  rsdia export <name|path> [--basic] [--comment <style>] [--fenced] [-o out.txt]
  rsdia list                    list saved drawings
  rsdia --version | --help

comment styles: none standard filled quotes hashes slashes three-slashes
                dashes apostrophes backticks four-spaces semicolons

drawings live in {}",
        env!("CARGO_PKG_VERSION"),
        store.dir.display()
    )
}

fn comment_aliases(comment: &str) -> &str {
    match comment {
        "standard" => "star",
        "filled" => "star-filled",
        "quotes" => "triple-quotes",
        "hashes" => "hash",
        "slashes" => "slash",
        "dashes" => "dash",
        "apostrophes" => "apostrophe",
        "semicolons" => "semicolon",
        other => other,
    }
}

fn fail(message: &str) -> ! {
    eprintln!("rsdia: {message}");
    std::process::exit(1);
}

fn is_path_like(arg: &str) -> bool {
    arg.contains('/')
        || Path::new(arg)
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("json"))
}

/// A drawing created from a text file. The name comes from the first positional
/// argument when there is one, and from the file's own stem otherwise.
fn import_drawing(
    store: &DrawingStore,
    file: &str,
    base: Option<&str>,
) -> Result<OpenDrawing, String> {
    let text = std::fs::read_to_string(file).map_err(|_| format!("can't read {file}"))?;
    let base = base.map_or_else(
        || {
            Path::new(file).file_stem().map_or_else(
                || "imported".to_string(),
                |s| s.to_string_lossy().to_string(),
            )
        },
        ToString::to_string,
    );
    let name = store.unique_name(&base);
    let layer = text_to_layer(&text, crate::core::vector::Pos::default());
    let path = store.path_for(&name);
    store
        .save(&path, &name, &layer)
        .map_err(|e| format!("can't write {}: {e}", path.display()))?;
    Ok(OpenDrawing { path, name, layer })
}

/// Resolves a CLI argument to a drawing file path, if one exists.
fn find_drawing(store: &DrawingStore, arg: &str) -> Option<PathBuf> {
    if is_path_like(arg) {
        let path = absolute(arg);
        return path.exists().then_some(path);
    }
    let path = store.path_for(arg);
    path.exists().then_some(path)
}

fn absolute(arg: &str) -> PathBuf {
    let path = PathBuf::from(arg);
    if path.is_absolute() {
        path
    } else {
        std::env::current_dir().unwrap_or_default().join(path)
    }
}

fn name_from_path(arg: &str) -> String {
    let base = Path::new(arg)
        .file_name()
        .map_or_else(|| arg.to_string(), |n| n.to_string_lossy().to_string());
    base.strip_suffix(FILE_EXT)
        .or_else(|| base.strip_suffix(".json"))
        .unwrap_or(&base)
        .to_string()
}

fn open_or_create(store: &DrawingStore, arg: &str) -> Result<OpenDrawing, String> {
    if let Some(existing) = find_drawing(store, arg) {
        // A file this build cannot read — a drawing from a newer version, a
        // half-written one, one worth repairing by hand — is never written over.
        let (name, layer) = store
            .load(&existing)
            .map_err(|e| format!("can't open {}: {e}", existing.display()))?;
        return Ok(OpenDrawing {
            path: existing,
            name,
            layer,
        });
    }
    let path = if is_path_like(arg) {
        absolute(arg)
    } else {
        store.path_for(arg)
    };
    let name = if is_path_like(arg) {
        name_from_path(arg)
    } else {
        arg.to_string()
    };
    store
        .save(&path, &name, &Layer::new())
        .map_err(|e| format!("can't write {}: {e}", path.display()))?;
    Ok(OpenDrawing {
        path,
        name,
        layer: Layer::new(),
    })
}

/// What to print and stop, if anything: the two flags that are not options.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Print {
    Help,
    Version,
}

/// Every flag both commands understand, with no parser dependency. Value-taking
/// flags accept `--flag value` and `--flag=value`; anything else is a positional.
#[derive(Default)]
struct Args {
    positional: Vec<String>,
    characters: Charset,
    fenced: bool,
    print: Option<Print>,
    comment: Option<String>,
    output: Option<String>,
    import: Option<String>,
}

fn parse_args(args: &[String]) -> Args {
    let mut out = Args::default();
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        let (name, inline) = match arg.split_once('=') {
            Some((name, value)) => (name, Some(value.to_string())),
            None => (arg.as_str(), None),
        };
        let mut value = |slot: &mut Option<String>| {
            *slot = Some(
                inline
                    .clone()
                    .or_else(|| rest.next().cloned())
                    .unwrap_or_default(),
            );
        };
        match name {
            "--basic" => out.characters = Charset::Basic,
            "--fenced" => out.fenced = true,
            "--help" | "-h" => out.print = Some(Print::Help),
            "--version" | "-v" => out.print = Some(Print::Version),
            "--comment" => value(&mut out.comment),
            "--output" | "-o" => value(&mut out.output),
            "--import" => value(&mut out.import),
            other => out.positional.push(other.to_string()),
        }
    }
    out
}

/// Opens the plugin's pane in the running herdr session.
///
/// herdr keybindings can only invoke plugin *actions* and there is no CLI wrapper
/// for arbitrary socket methods, so this speaks the raw newline-delimited JSON
/// protocol: one request per line, one response line back carrying the same id.
fn open_pane_command() {
    #[cfg(unix)]
    {
        use std::io::{BufRead, BufReader, Write};
        use std::os::unix::net::UnixStream;

        let Ok(socket_path) = std::env::var("HERDR_SOCKET_PATH") else {
            fail("HERDR_SOCKET_PATH is not set — run this through herdr");
        };
        let plugin_id =
            std::env::var("HERDR_PLUGIN_ID").unwrap_or_else(|_| "mpospirit.rsdia".to_string());
        let request = serde_json::json!({
            "id": format!("rsdia-{}", std::process::id()),
            "method": "plugin.pane.open",
            "params": {
                "plugin_id": plugin_id,
                "entrypoint": "canvas",
                "placement": "split",
                "direction": "right",
                "focus": true,
            },
        });
        let Ok(mut stream) = UnixStream::connect(&socket_path) else {
            fail("cannot reach herdr");
        };
        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
        if writeln!(stream, "{request}").is_err() {
            fail("cannot reach herdr");
        }
        let mut line = String::new();
        if BufReader::new(stream).read_line(&mut line).unwrap_or(0) == 0 {
            fail("unreadable response from herdr");
        }
        let Ok(response) = serde_json::from_str::<serde_json::Value>(&line) else {
            fail(&format!("unreadable response from herdr: {}", line.trim()));
        };
        if response.get("error").is_some() {
            fail(
                response["error"]["message"]
                    .as_str()
                    .unwrap_or("herdr refused to open the pane"),
            );
        }
    }
    #[cfg(not(unix))]
    fail("herdr plugin actions need a unix socket");
}

fn export_command(store: &DrawingStore, args: &[String]) {
    let parsed = parse_args(args);
    if parsed.print == Some(Print::Help) {
        println!("{}", help(store));
        return;
    }
    let Some(target) = parsed.positional.first() else {
        fail("export needs a drawing name or path");
    };
    let Some(path) = find_drawing(store, target) else {
        fail(&format!("no drawing named \"{target}\""));
    };
    let comment = parsed.comment.as_deref().unwrap_or("none");
    let wrapper = comment_aliases(comment);
    let Some(wrapper) = is_wrapper(wrapper) else {
        fail(&format!("unknown comment style \"{comment}\""));
    };
    let config = ExportConfig {
        characters: parsed.characters,
        wrapper,
        fenced: parsed.fenced,
    };
    let layer = match store.load(&path) {
        Ok((_, layer)) => layer,
        Err(e) => fail(&format!("can't read {}: {e}", path.display())),
    };
    let text = format!("{}\n", export_text(&layer, &config));
    match parsed.output.as_deref() {
        Some(output) => {
            if let Err(e) = std::fs::write(output, text) {
                fail(&format!("can't write {output}: {e}"));
            }
        }
        None => {
            let _ = std::io::stdout().write_all(text.as_bytes());
        }
    }
}

fn list_command(store: &DrawingStore) {
    for d in store.list() {
        println!("{}\t{} cells\t{}", d.name, d.size, d.path.display());
    }
}

fn is_tty() -> bool {
    use std::io::IsTerminal;
    std::io::stdin().is_terminal() && std::io::stdout().is_terminal()
}

const fn system_clipboard() -> Clipboard {
    Clipboard::new(true)
}

/// Everything the app turns on, in the order the terminal wants it off: mouse
/// reporting, the kitty keyboard flags, bracketed paste, then the alternate screen.
/// The signal handler and the panic hook both write exactly these bytes.
const RESTORE: &[u8] =
    b"\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1006l\x1b[<1u\x1b[?2004l\x1b[?1049l";

/// Drops those modes for anything that unwinds past the event loop — a bug in the
/// drawing code should not leave the user's shell echoing mouse escape codes.
/// `ratatui::init` installs its own restoring hook, so this one chains onto it.
fn install_panic_restore() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = std::io::stdout().write_all(RESTORE);
        previous(info);
    }));
}

/// Restores the terminal on SIGTERM/SIGHUP/SIGINT, which ctrl+q never sees.
#[cfg(unix)]
fn install_signal_restore() {
    extern "C" fn restore(_sig: libc::c_int) {
        // Only async-signal-safe writes.
        unsafe {
            libc::write(1, RESTORE.as_ptr().cast(), RESTORE.len());
        }
        unsafe { libc::_exit(0) };
    }
    for sig in [libc::SIGTERM, libc::SIGHUP, libc::SIGINT] {
        unsafe {
            libc::signal(sig, restore as *const () as libc::sighandler_t);
        }
    }
}

#[cfg(not(unix))]
fn install_signal_restore() {}

fn run_terminal(terminal: &mut DefaultTerminal, app: &mut App) -> std::io::Result<()> {
    loop {
        terminal.draw(|frame| app.paint(frame.buffer_mut()))?;
        if app.should_quit {
            return Ok(());
        }
        if event::poll(Duration::from_millis(33))? {
            match event::read()? {
                Event::Key(key) => app.on_key(&key),
                Event::Mouse(mouse) => app.on_mouse(&mouse),
                Event::Paste(text) => app.on_paste(&text),
                _ => {}
            }
        }
        app.tick();
    }
}

fn interactive(
    store: DrawingStore,
    config: Config,
    config_path: PathBuf,
    drawing: OpenDrawing,
) -> ! {
    let mut terminal = ratatui::init();
    let _ = execute!(
        std::io::stdout(),
        EnableMouseCapture,
        EnableBracketedPaste,
        PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
        )
    );
    install_signal_restore();
    install_panic_restore();

    // The `terminal` theme inherits the terminal's own colors, so ask before the
    // first frame; a terminal too slow to answer keeps the stand-in scheme.
    let term = if config.theme == ThemeName::Terminal {
        detect_terminal_colors(PALETTE_WAIT_MS)
    } else {
        None
    };

    let mut app = App::new(AppOptions {
        store,
        config,
        config_path,
        drawing,
        clipboard: system_clipboard(),
        term,
        autosave_ms: None,
    });
    let result = run_terminal(&mut terminal, &mut app);
    app.shutdown();
    let _ = execute!(
        std::io::stdout(),
        PopKeyboardEnhancementFlags,
        DisableBracketedPaste,
        DisableMouseCapture
    );
    ratatui::restore();
    match result {
        Ok(()) => std::process::exit(0),
        Err(e) => {
            eprintln!("rsdia: {e}");
            std::process::exit(1);
        }
    }
}

pub fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let store = DrawingStore::default();

    if argv.first().map(String::as_str) == Some("open-pane") {
        open_pane_command();
        return;
    }
    if argv.first().map(String::as_str) == Some("export") {
        export_command(&store, &argv[1..]);
        return;
    }
    if argv.first().map(String::as_str) == Some("list") {
        list_command(&store);
        return;
    }

    let parsed = parse_args(&argv);
    match parsed.print {
        Some(Print::Help) => {
            println!("{}", help(&store));
            return;
        }
        Some(Print::Version) => {
            println!("{}", env!("CARGO_PKG_VERSION"));
            return;
        }
        None => {}
    }
    if !is_tty() {
        fail("needs an interactive terminal (try `rsdia export`)");
    }

    let config_path = crate::storage::config::config_path();
    let mut config = load_config(&config_path);
    let drawing = match parsed.import.as_deref() {
        Some(file) => import_drawing(&store, file, parsed.positional.first().map(String::as_str)),
        None => parsed.positional.first().map_or_else(
            || {
                let last = config
                    .last_drawing
                    .clone()
                    .filter(|p| Path::new(p).exists());
                let name = last.unwrap_or_else(|| {
                    store
                        .list()
                        .first()
                        .map_or_else(|| "untitled".to_string(), |d| d.name.clone())
                });
                open_or_create(&store, &name)
            },
            |arg| open_or_create(&store, arg),
        ),
    }
    .unwrap_or_else(|e| fail(&e));
    config.last_drawing = Some(drawing.path.to_string_lossy().to_string());
    save_config(&config, &config_path);

    interactive(store, config, config_path, drawing)
}

#[cfg(test)]
mod tests {
    use super::{open_or_create, DrawingStore};

    fn store(name: &str) -> (DrawingStore, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("rsdia-cli-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a writable temp dir");
        (DrawingStore::new(dir.clone()), dir)
    }

    #[test]
    fn a_drawing_this_build_cannot_read_is_not_written_over() {
        let (store, dir) = store("unreadable");
        let path = store.path_for("future");
        // A drawing from a newer version, or a half-written one.
        let original = r#"{"version":99,"name":"future","cells":[]}"#;
        std::fs::write(&path, original).expect("writable");

        let Err(err) = open_or_create(&store, "future") else {
            panic!("an unreadable drawing must not be replaced");
        };
        assert!(err.starts_with("can't open"), "{err}");
        assert_eq!(
            std::fs::read_to_string(&path).expect("still there"),
            original
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn opening_a_free_name_creates_it() {
        let (store, dir) = store("fresh");
        let drawing = open_or_create(&store, "fresh one").expect("created");
        assert_eq!(drawing.name, "fresh one");
        assert!(drawing.path.exists());
        assert!(drawing.layer.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
