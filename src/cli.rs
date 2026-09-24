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
use crate::storage::drawings::{now_iso, DrawingStore, FILE_EXT};
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
    arg.contains('/') || arg.ends_with(FILE_EXT) || arg.ends_with(".json")
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
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| arg.to_string());
    base.strip_suffix(FILE_EXT)
        .or_else(|| base.strip_suffix(".json"))
        .unwrap_or(&base)
        .to_string()
}

fn open_or_create(store: &DrawingStore, arg: &str) -> OpenDrawing {
    if let Some(existing) = find_drawing(store, arg) {
        if let Ok((file, layer)) = store.load(&existing) {
            return OpenDrawing {
                path: existing,
                name: file.name,
                layer,
                created_at: file.created_at,
            };
        }
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
    let created_at = now_iso();
    if let Err(e) = store.save(&path, &name, &Layer::new(), &created_at) {
        fail(&format!("can't write {}: {e}", path.display()));
    }
    OpenDrawing {
        path,
        name,
        layer: Layer::new(),
        created_at,
    }
}

/// Every flag both commands understand, with no parser dependency. Value-taking
/// flags accept `--flag value` and `--flag=value`; anything else is a positional.
#[derive(Default)]
struct Args {
    positional: Vec<String>,
    basic: bool,
    fenced: bool,
    help: bool,
    version: bool,
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
            "--basic" => out.basic = true,
            "--fenced" => out.fenced = true,
            "--help" | "-h" => out.help = true,
            "--version" | "-v" => out.version = true,
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
        let Ok(stream) = UnixStream::connect(&socket_path) else {
            fail("cannot reach herdr");
        };
        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
        let mut stream = stream;
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
    if parsed.help {
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
        characters: if parsed.basic {
            Charset::Basic
        } else {
            Charset::Extended
        },
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

fn system_clipboard() -> Clipboard {
    Clipboard::new(true)
}

/// Restores the terminal on SIGTERM/SIGHUP/SIGINT, which ctrl+q never sees.
#[cfg(unix)]
fn install_signal_restore() {
    extern "C" fn restore(_sig: libc::c_int) {
        // Only async-signal-safe writes: leave the alternate screen, then drop the
        // mouse/paste modes the app turned on.
        const EXIT: &[u8] = b"\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1006l\x1b[?1049l";
        unsafe {
            libc::write(1, EXIT.as_ptr() as *const libc::c_void, EXIT.len());
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
    if parsed.help {
        println!("{}", help(&store));
        return;
    }
    if parsed.version {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return;
    }
    if !is_tty() {
        fail("needs an interactive terminal (try `rsdia export`)");
    }

    let config_path = crate::storage::config::config_path();
    let mut config = load_config(&config_path);
    let drawing = match parsed.import.as_deref() {
        Some(file) => match std::fs::read_to_string(file) {
            Ok(text) => {
                let base = parsed.positional.first().cloned().unwrap_or_else(|| {
                    Path::new(file)
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| "imported".to_string())
                });
                let name = store.unique_name(&base);
                let layer = text_to_layer(&text, crate::core::vector::Pos::default());
                let path = store.path_for(&name);
                let created_at = now_iso();
                if let Err(e) = store.save(&path, &name, &layer, &created_at) {
                    fail(&format!("can't write {}: {e}", path.display()));
                }
                OpenDrawing {
                    path,
                    name,
                    layer,
                    created_at,
                }
            }
            Err(_) => fail(&format!("can't read {file}")),
        },
        None => match parsed.positional.first() {
            Some(arg) => open_or_create(&store, arg),
            None => match config
                .last_drawing
                .clone()
                .filter(|p| Path::new(p).exists())
            {
                Some(last) => open_or_create(&store, &last),
                None => {
                    let name = store
                        .list()
                        .first()
                        .map(|d| d.name.clone())
                        .unwrap_or_else(|| "untitled".to_string());
                    open_or_create(&store, &name)
                }
            },
        },
    };
    config.last_drawing = Some(drawing.path.to_string_lossy().to_string());
    save_config(&config, &config_path);

    interactive(store, config, config_path, drawing)
}
