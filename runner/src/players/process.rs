use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, Command, Stdio},
    sync::{
        Arc, LazyLock, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

#[cfg(unix)]
use std::os::unix::process::CommandExt;

// Tracks every currently running engine's stdin so quit_all_engines() can
// reach them from outside the (per-worker-thread, !Send) Player trait
// objects that actually own them.
static NEXT_ENGINE_ID: AtomicU64 = AtomicU64::new(0);
static ENGINE_STDINS: LazyLock<Mutex<HashMap<u64, Arc<Mutex<ChildStdin>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

// Asks every currently running engine to stop and quit, abandoning
// whatever games are in progress. Used when the user wants those games
// ended right away (e.g. a second Ctrl-C) rather than left to finish
// naturally.
pub fn quit_all_engines() {
    let stdins = ENGINE_STDINS.lock().expect("ENGINE_STDINS lock poisoned");
    for stdin in stdins.values() {
        if let Ok(mut stdin) = stdin.lock() {
            let _ = stdin.write_all(b"stop\n");
            let _ = stdin.write_all(b"quit\n");
        }
    }
}

pub struct EngineProcess {
    id: u64,
    child: Child,
    // Kept across calls rather than rebuilt per wait()/wait_magic(): a fresh
    // BufReader can read ahead into the pipe, and any bytes it buffered
    // beyond the line we were waiting for would otherwise be discarded when
    // it's dropped at the end of the call.
    stdout: BufReader<std::process::ChildStdout>,
    stdin: Option<Arc<Mutex<ChildStdin>>>,
    callback: Box<dyn Fn(&str)>,
}

impl EngineProcess {
    pub fn send(&mut self, msg: &str) {
        (self.callback)(msg);
        if let Some(stdin) = &self.stdin
            && let Ok(mut stdin) = stdin.lock()
        {
            let _ = stdin.write_all(msg.as_bytes());
        }
    }

    pub fn set_option(&mut self, name: &str, value: &str) {
        self.send(&format!("setoption name {} value {}\n", name, value));
    }

    pub fn wait(&mut self, msg: &str) {
        loop {
            let mut line = String::new();
            match self.stdout.read_line(&mut line) {
                Ok(0) => return,
                Ok(_) => {
                    (self.callback)(&line);
                    if line.trim_end() == msg.trim_end() {
                        return;
                    }
                }
                Err(err) => {
                    println!("Error: {}", err);
                    return;
                }
            }
        }
    }

    pub fn wait_magic(&mut self, matches: &mut dyn FnMut(&str) -> bool) {
        loop {
            let mut line = String::new();
            match self.stdout.read_line(&mut line) {
                Ok(0) => return,
                Ok(_) => {
                    (self.callback)(&line);
                    if matches(&line) {
                        return;
                    }
                }
                Err(err) => {
                    println!("Error: {}", err);
                    return;
                }
            }
        }
    }
}

impl EngineProcess {
    pub fn new(path: &str, parameters: &str, callback: Box<dyn Fn(&str)>) -> Self {
        let mut command = Command::new(path);
        command
            .args(parameters.split_whitespace())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped());

        // Put the engine in its own process group so a terminal Ctrl-C
        // (delivered to the whole foreground process group) doesn't kill
        // it out from under an in-progress game -- only cutergames' own
        // Ctrl-C handler should decide when to stop.
        #[cfg(unix)]
        command.process_group(0);

        let mut child = command
            .spawn()
            .unwrap_or_else(|err| panic!("Failed to launch engine process at {path:?}: {err}"));

        let stdin = Arc::new(Mutex::new(
            child.stdin.take().expect("engine stdin should be piped"),
        ));
        let stdout = BufReader::new(child.stdout.take().expect("engine stdout should be piped"));
        let id = NEXT_ENGINE_ID.fetch_add(1, Ordering::Relaxed);
        ENGINE_STDINS
            .lock()
            .expect("ENGINE_STDINS lock poisoned")
            .insert(id, stdin.clone());

        Self {
            id,
            child,
            stdout,
            stdin: Some(stdin),
            callback,
        }
    }
}

impl Drop for EngineProcess {
    fn drop(&mut self) {
        ENGINE_STDINS
            .lock()
            .expect("ENGINE_STDINS lock poisoned")
            .remove(&self.id);

        // Drop our own reference to the engine's stdin; if the registry
        // above held the only other one, this closes the pipe before we
        // wait for the child to exit below.
        drop(self.stdin.take());
        let _ = self.child.wait();
    }
}
