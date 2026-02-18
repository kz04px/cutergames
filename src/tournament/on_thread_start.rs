pub fn on_thread_start(id: usize, is_verbose: bool) {
    if is_verbose {
        println!("Start thread {}", id);
    }
}
