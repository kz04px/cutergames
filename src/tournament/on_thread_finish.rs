pub fn on_thread_finish(id: usize, is_verbose: bool) {
    if is_verbose {
        println!("Finish thread {}", id);
    }
}
