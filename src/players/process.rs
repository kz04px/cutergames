use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, Command, Stdio},
};

pub struct EngineProcess {
    child: Child,
    callback: &'static dyn Fn(&str),
    // stdin: ChildStdin,
    // stdout: ChildStdout,
}

impl EngineProcess {
    pub fn send(&mut self, msg: &str) {
        (self.callback)(msg);
        let stdin = self.child.stdin.as_mut().unwrap();
        let _ = stdin.write_all(msg.as_bytes());
    }

    pub fn wait(&mut self, msg: &str) {
        let mut child_out = BufReader::new(self.child.stdout.as_mut().unwrap());

        loop {
            let mut line = String::new();
            match child_out.read_line(&mut line) {
                Ok(nbytes) => {
                    (self.callback)(msg);
                    if nbytes == 0 || line == msg {
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

    pub fn wait_magic(&mut self, grr: &mut dyn FnMut(&str) -> bool) {
        let mut child_out = BufReader::new(self.child.stdout.as_mut().unwrap());

        loop {
            let mut line = String::new();
            match child_out.read_line(&mut line) {
                Ok(_) => {
                    (self.callback)(&line);
                    let ret = grr(&line);
                    if ret {
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
    pub fn new(path: &str, callback: &'static dyn Fn(&str)) -> Self {
        let child = Command::new(path)
            .args(["--game", "ataxx"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to launch engine process");

        // let stdin = child.stdin.as_mut().unwrap();

        // let stdout = child.stdout.as_mut().unwrap();

        Self {
            child,
            callback,
            // stdin,
            // stdout,
        }
    }
}

impl Drop for EngineProcess {
    fn drop(&mut self) {
        let _ = self.child.wait();
    }
}
