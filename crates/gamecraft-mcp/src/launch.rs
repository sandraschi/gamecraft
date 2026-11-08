//! Starting, tracking and stopping games, for the CLI, the MCP server and the desktop app.
//!
//! Three runtimes start differently:
//!
//! * web (three.js): the static arcade server (`bun run serve.ts`, port 11258) must be up;
//!   the game is a URL, and `?autoplay=1` makes it play itself. A browser tab is not a
//!   process we own, so web games are never in the process table, only the server is.
//! * Godot: `godot --path <dir>`, with `GAMECRAFT_AUTOPLAY=1` for a demo.
//! * Bevy: the prebuilt binary named in `game.json`, with `--autoplay` for a demo.
//!
//! Native games and the arcade server we started are recorded in
//! `<root>/target/gamecraft/running.json` so a later `running` or `stop` (a different
//! process, perhaps a different front end) can find them. Dead pids are dropped on read.
//! A game started by the MCP server dies with that server (see [`crate::proc_guard`]).

#![allow(clippy::missing_errors_doc)]

use crate::catalog::{self, GameEntry, GameKind};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Port of the web arcade.
pub const WEB_PORT: u16 = 11_258;

/// How to start a game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Interactive.
    Play,
    /// The game plays itself (autoplay).
    Demo,
}

impl Mode {
    /// Name used in JSON and arguments.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Play => "play",
            Self::Demo => "demo",
        }
    }

    /// Parse `play` or `demo`.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        match name.trim().to_lowercase().as_str() {
            "play" => Some(Self::Play),
            "demo" => Some(Self::Demo),
            _ => None,
        }
    }
}

/// One native game process this machine started.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Process {
    /// Game slug.
    pub slug: String,
    /// `godot` or `bevy`.
    pub kind: String,
    /// OS process id.
    pub pid: u32,
    /// `play` or `demo`.
    pub mode: String,
    /// Unix seconds when it started.
    pub started: u64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Table {
    #[serde(default)]
    games: Vec<Process>,
    /// Pid of an arcade server we started (absent when it was already running).
    #[serde(default)]
    server: Option<u32>,
}

/// What a launch did.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Launched {
    /// Game slug.
    pub slug: String,
    /// Runtime name.
    pub kind: &'static str,
    /// `play` or `demo`.
    pub mode: &'static str,
    /// Process id of a native game (none for web games).
    pub pid: Option<u32>,
    /// URL of a web game.
    pub url: Option<String>,
    /// Was a browser opened on the URL?
    pub opened: bool,
    /// Pid of the arcade server when this call had to start it.
    pub server_started: Option<u32>,
    /// One honest sentence about what now exists.
    pub note: String,
}

fn table_path(root: &Path) -> PathBuf {
    root.join("target").join("gamecraft").join("running.json")
}

fn read_table(root: &Path) -> Table {
    std::fs::read_to_string(table_path(root))
        .ok()
        .and_then(|text| serde_json::from_str(text.as_str()).ok())
        .unwrap_or_default()
}

fn write_table(root: &Path, table: &Table) -> Result<(), String> {
    let path = table_path(root);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    }
    let text = serde_json::to_string_pretty(table).map_err(|e| e.to_string())?;
    std::fs::write(&path, text).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Is a process with this id running?
#[must_use]
pub fn pid_alive(pid: u32) -> bool {
    if cfg!(windows) {
        Command::new("tasklist")
            .args(["/FI", format!("PID eq {pid}").as_str(), "/NH", "/FO", "CSV"])
            .stdin(Stdio::null())
            .output()
            .is_ok_and(|o| {
                String::from_utf8_lossy(&o.stdout).contains(format!("\"{pid}\"").as_str())
            })
    } else {
        Command::new("kill")
            .args(["-0", pid.to_string().as_str()])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    }
}

/// Stop a process and its children. Returns whether the OS accepted the request.
#[must_use]
pub fn kill_pid(pid: u32) -> bool {
    let status = if cfg!(windows) {
        Command::new("taskkill")
            .args(["/PID", pid.to_string().as_str(), "/T", "/F"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
    } else {
        Command::new("kill")
            .args(["-TERM", pid.to_string().as_str()])
            .stdin(Stdio::null())
            .status()
    };
    status.is_ok_and(|s| s.success())
}

/// URL of a web game on the local arcade.
#[must_use]
pub fn web_url(slug: &str, mode: Mode) -> String {
    let suffix = if mode == Mode::Demo {
        "?autoplay=1"
    } else {
        ""
    };
    format!("http://127.0.0.1:{WEB_PORT}/games/{slug}/{suffix}")
}

/// True when whatever listens on the arcade port is the `GameCraft` arcade: it must
/// answer `/api/games` with the game list. A bare connect is not enough, since any
/// process holding the port would otherwise get a browser opened at it.
#[must_use]
pub fn probe_web_arcade() -> bool {
    http_get(WEB_PORT, "/api/games")
        .is_some_and(|(status, body)| status == 200 && body.contains("\"slug\""))
}

/// `GET path` from a local server (HTTP/1.0, so the server closes when done).
/// Returns the status code and the body (at most 256 KB).
fn http_get(port: u16, path: &str) -> Option<(u16, String)> {
    use std::io::{Read, Write};
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    let mut stream =
        std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(400)).ok()?;
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(1500)));
    let request = format!("GET {path} HTTP/1.0\r\nHost: 127.0.0.1:{port}\r\n\r\n");
    stream.write_all(request.as_bytes()).ok()?;
    let mut raw = Vec::new();
    let _ = stream.take(262_144).read_to_end(&mut raw);
    let text = String::from_utf8_lossy(&raw).into_owned();
    let (head, body) = text.split_once("\r\n\r\n")?;
    let status = head.split_whitespace().nth(1)?.parse().ok()?;
    Some((status, body.to_string()))
}

fn normalized(text: &str) -> String {
    text.replace("\r\n", "\n").trim().to_string()
}

/// Is the arcade on `port` serving this checkout's copy of the game? It compares the
/// `game.json` the server sends with the one on disk. An arcade started from another
/// checkout (a different worktree, an older clone) would otherwise happily show a stale
/// or missing game while we report success.
fn serves_checkout(port: u16, game: &GameEntry) -> Result<(), String> {
    let path = format!("/games/{}/game.json", game.slug);
    let ours = std::fs::read_to_string(game.dir.join("game.json")).map_err(|e| e.to_string())?;
    match http_get(port, path.as_str()) {
        Some((200, theirs)) if normalized(theirs.as_str()) == normalized(ours.as_str()) => Ok(()),
        Some((200, _)) => Err(format!(
            "the web arcade on port {port} serves a different copy of {} than {}: it was started \
             from another checkout. Stop it (end the bun process holding the port) and retry, or \
             use that checkout's own tools",
            game.slug,
            game.dir.display()
        )),
        Some((status, _)) => Err(format!(
            "the web arcade on port {port} answered {status} for {}: it was started from another \
             checkout that does not have this game. Stop it (end the bun process holding the port) and retry",
            game.slug
        )),
        None => Err(format!("the web arcade on port {port} did not answer")),
    }
}

/// Locate bun: `GAMECRAFT_BUN`, then the user install, then PATH.
#[must_use]
pub fn bun_binary() -> String {
    if let Ok(explicit) = std::env::var("GAMECRAFT_BUN") {
        if !explicit.trim().is_empty() {
            return explicit;
        }
    }
    if let Ok(home) = std::env::var(if cfg!(windows) { "USERPROFILE" } else { "HOME" }) {
        let candidate = Path::new(&home)
            .join(".bun")
            .join("bin")
            .join(format!("bun{}", std::env::consts::EXE_SUFFIX));
        if candidate.is_file() {
            return candidate.to_string_lossy().into_owned();
        }
    }
    "bun".to_string()
}

/// Make sure the arcade answers on its port. Returns the pid when this call started it.
fn ensure_web_server(root: &Path) -> Result<Option<u32>, String> {
    let port_open = std::net::TcpStream::connect(("127.0.0.1", WEB_PORT)).is_ok();
    if port_open {
        return if probe_web_arcade() {
            Ok(None)
        } else {
            Err(format!(
                "port {WEB_PORT} is held by something that is not the GameCraft web server"
            ))
        };
    }
    let web = root.join("web");
    if !web.join("serve.ts").is_file() {
        return Err(format!("{} has no serve.ts", web.display()));
    }
    let bun = bun_binary();
    let child = Command::new(bun.as_str())
        .arg("run")
        .arg("serve.ts")
        .current_dir(&web)
        // Loopback only: a 0.0.0.0 bind fails with EADDRINUSE while tailscaled serves
        // this port on the tailnet. An explicit HOST in the environment still wins.
        .env(
            "HOST",
            std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
        )
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("cannot start the web server ({bun}): {e}"))?;
    let pid = child.id();
    reap(child);
    for _ in 0..60 {
        if probe_web_arcade() {
            return Ok(Some(pid));
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    Err(format!(
        "started {bun} run serve.ts (pid {pid}) but nothing answered on port {WEB_PORT} within 6 s; \
         run `bun install` in web/ if node_modules is missing"
    ))
}

/// Wait for a child on a helper thread so it never lingers as a zombie.
fn reap(mut child: std::process::Child) {
    std::thread::spawn(move || {
        let _ = child.wait();
    });
}

/// Open a URL in the default browser.
pub fn open_browser(url: &str) -> Result<(), String> {
    let result = if cfg!(windows) {
        Command::new("cmd").args(["/C", "start", "", url]).spawn()
    } else if cfg!(target_os = "macos") {
        Command::new("open").arg(url).spawn()
    } else {
        Command::new("xdg-open").arg(url).spawn()
    };
    result
        .map(reap)
        .map_err(|e| format!("cannot open a browser: {e}"))
}

/// Start one game.
///
/// `open` decides whether a web game is also opened in the default browser (native games
/// always open their own window). `godot` overrides the Godot binary.
pub fn launch(
    game: &GameEntry,
    root: &Path,
    mode: Mode,
    open: bool,
    godot: Option<&str>,
) -> Result<Launched, String> {
    catalog::playable(game)?;
    match game.kind {
        GameKind::Web => {
            let server_started = ensure_web_server(root)?;
            if let Some(pid) = server_started {
                let mut table = read_table(root);
                table.server = Some(pid);
                write_table(root, &table)?;
            }
            serves_checkout(WEB_PORT, game)?;
            let url = web_url(game.slug.as_str(), mode);
            let opened = if open {
                open_browser(url.as_str())?;
                true
            } else {
                false
            };
            let note = if opened {
                format!("opened {url} in your default browser; close the tab to stop it")
            } else {
                format!("the arcade is serving {url}; open it in a browser")
            };
            Ok(Launched {
                slug: game.slug.clone(),
                kind: "web",
                mode: mode.as_str(),
                pid: None,
                url: Some(url),
                opened,
                server_started,
                note,
            })
        }
        GameKind::Godot | GameKind::Bevy => {
            let _ = stop(root, Some(game.slug.as_str()), false);
            let mut command = if game.kind == GameKind::Godot {
                let binary = godot
                    .map(str::to_string)
                    .or_else(|| std::env::var("GAMECRAFT_GODOT").ok())
                    .unwrap_or_else(|| "godot".to_string());
                let mut c = Command::new(binary);
                c.arg("--path").arg(&game.dir);
                if mode == Mode::Demo {
                    c.env("GAMECRAFT_AUTOPLAY", "1");
                }
                c
            } else {
                let exe = catalog::bevy_launch(game, root)?;
                let mut c = Command::new(exe);
                c.current_dir(root);
                if mode == Mode::Demo {
                    c.arg("--autoplay");
                }
                c
            };
            let child = command
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| format!("cannot start {}: {e}", game.slug))?;
            let pid = child.id();
            reap(child);
            let mut table = read_table(root);
            table.games.push(Process {
                slug: game.slug.clone(),
                kind: game.kind.as_str().to_string(),
                pid,
                mode: mode.as_str().to_string(),
                started: now_secs(),
            });
            write_table(root, &table)?;
            Ok(Launched {
                slug: game.slug.clone(),
                kind: game.kind.as_str(),
                mode: mode.as_str(),
                pid: Some(pid),
                url: None,
                opened: false,
                server_started: None,
                note: format!("started {} in its own window (pid {pid})", game.slug),
            })
        }
    }
}

/// What is running now.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Running {
    /// Native games we started that are still alive.
    pub games: Vec<Process>,
    /// Does the web arcade answer on its port?
    pub arcade_up: bool,
    /// Pid of the arcade server we started, if it is still alive.
    pub arcade_pid: Option<u32>,
}

/// Live native games and the arcade state. Dead entries are removed from the table.
#[must_use]
pub fn running(root: &Path) -> Running {
    let mut table = read_table(root);
    let before = (table.games.len(), table.server);
    table.games.retain(|p| pid_alive(p.pid));
    if table.server.is_some_and(|pid| !pid_alive(pid)) {
        table.server = None;
    }
    if before != (table.games.len(), table.server) {
        let _ = write_table(root, &table);
    }
    Running {
        games: table.games,
        arcade_up: probe_web_arcade(),
        arcade_pid: table.server,
    }
}

/// Result of a stop request.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Stopped {
    /// What was stopped, e.g. `maze (pid 4242)` or `web arcade (pid 17)`.
    pub what: String,
    /// Did the OS accept the kill?
    pub ok: bool,
}

/// Stop native games we started: the one named by `slug`, or all when `None`.
/// `include_server` also stops the arcade server when this machine started it.
#[must_use]
pub fn stop(root: &Path, slug: Option<&str>, include_server: bool) -> Vec<Stopped> {
    let mut table = read_table(root);
    let mut out = Vec::new();
    let mut kept = Vec::new();
    for p in table.games.drain(..) {
        if slug.is_none_or(|s| s == p.slug) {
            let ok = !pid_alive(p.pid) || kill_pid(p.pid);
            out.push(Stopped {
                what: format!("{} (pid {})", p.slug, p.pid),
                ok,
            });
        } else {
            kept.push(p);
        }
    }
    table.games = kept;
    if include_server {
        if let Some(pid) = table.server.take() {
            let ok = !pid_alive(pid) || kill_pid(pid);
            out.push(Stopped {
                what: format!("web arcade (pid {pid})"),
                ok,
            });
        }
    }
    let _ = write_table(root, &table);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("gamecraft-launch-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn sleeper() -> Command {
        if cfg!(windows) {
            let mut c = Command::new("powershell");
            c.args(["-NoProfile", "-Command", "Start-Sleep -Seconds 60"]);
            c
        } else {
            let mut c = Command::new("sleep");
            c.arg("60");
            c
        }
    }

    #[test]
    fn urls_and_modes() {
        assert_eq!(
            web_url("hoshi", Mode::Play),
            "http://127.0.0.1:11258/games/hoshi/"
        );
        assert!(web_url("hoshi", Mode::Demo).ends_with("/games/hoshi/?autoplay=1"));
        assert_eq!(Mode::parse(" Demo "), Some(Mode::Demo));
        assert_eq!(Mode::parse("watch"), None);
    }

    #[test]
    fn own_pid_is_alive_and_an_absurd_pid_is_not() {
        assert!(pid_alive(std::process::id()));
        assert!(!pid_alive(4_000_000_000));
    }

    #[test]
    fn table_tracks_stops_and_forgets_a_real_process() {
        let root = temp_root("table");
        let child = sleeper()
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        let Ok(child) = child else {
            return; // no sleeper on this machine: nothing to prove
        };
        let pid = child.id();
        let table = Table {
            games: vec![Process {
                slug: "maze".into(),
                kind: "godot".into(),
                pid,
                mode: "play".into(),
                started: now_secs(),
            }],
            server: None,
        };
        assert!(write_table(root.as_path(), &table).is_ok());
        assert_eq!(running(root.as_path()).games.len(), 1);
        let none = stop(root.as_path(), Some("other"), false);
        assert_eq!(none.len(), 0);
        let stopped = stop(root.as_path(), Some("maze"), false);
        assert_eq!(stopped.len(), 1);
        assert!(stopped.iter().all(|s| s.ok));
        reap(child);
        for _ in 0..50 {
            if !pid_alive(pid) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        assert!(!pid_alive(pid));
        assert_eq!(running(root.as_path()).games.len(), 0);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A one-shot local server that answers any request with `status` and `body`.
    fn fake_arcade(status: u16, body: &'static str) -> u16 {
        use std::io::{Read, Write};
        let Ok(listener) = std::net::TcpListener::bind(("127.0.0.1", 0)) else {
            return 0;
        };
        let port = listener.local_addr().map_or(0, |a| a.port());
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0_u8; 1024];
                let _ = stream.read(&mut buf);
                let reply =
                    format!("HTTP/1.0 {status} X\r\ncontent-type: application/json\r\n\r\n{body}");
                let _ = stream.write_all(reply.as_bytes());
            }
        });
        port
    }

    #[test]
    fn an_arcade_from_another_checkout_is_detected() {
        let root = temp_root("checkout");
        let dir = root.join("web").join("games").join("tiles");
        assert!(std::fs::create_dir_all(&dir).is_ok());
        assert!(std::fs::write(dir.join("game.json"), "{\"title\":\"Tiles\"}\r\n").is_ok());
        assert!(std::fs::write(dir.join("index.html"), "<html></html>").is_ok());
        let games = catalog::discover(root.as_path());
        let tiles = catalog::find(&games, "tiles", None);
        assert!(tiles.is_some());
        if let Some(tiles) = tiles {
            let same = fake_arcade(200, "{\"title\":\"Tiles\"}\n");
            assert!(serves_checkout(same, tiles).is_ok());
            let other = fake_arcade(200, "{\"title\":\"Old Tiles\"}");
            assert!(serves_checkout(other, tiles).is_err_and(|e| e.contains("another checkout")));
            let missing = fake_arcade(404, "not found");
            assert!(serves_checkout(missing, tiles).is_err_and(|e| e.contains("404")));
            assert!(serves_checkout(1, tiles).is_err());
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn dead_entries_are_dropped_on_read() {
        let root = temp_root("dead");
        let table = Table {
            games: vec![Process {
                slug: "ghost".into(),
                kind: "bevy".into(),
                pid: 4_000_000_001,
                mode: "demo".into(),
                started: 0,
            }],
            server: Some(4_000_000_002),
        };
        assert!(write_table(root.as_path(), &table).is_ok());
        let now = running(root.as_path());
        assert!(now.games.is_empty() && now.arcade_pid.is_none());
        assert_eq!(read_table(root.as_path()).games.len(), 0);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn unplayable_entries_are_refused_before_anything_starts() {
        let root = temp_root("refuse");
        let dir = root.join("web").join("games").join("case");
        assert!(std::fs::create_dir_all(&dir).is_ok());
        assert!(std::fs::write(dir.join("game.json"), r#"{"title":"Case"}"#).is_ok());
        let games = catalog::discover(root.as_path());
        let case = catalog::find(&games, "case", None);
        let refused = case.map(|g| launch(g, root.as_path(), Mode::Play, false, None));
        assert!(refused.is_some_and(|r| r.err().is_some_and(|e| e.contains("index.html"))));
        let _ = std::fs::remove_dir_all(&root);
    }
}
