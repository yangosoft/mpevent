use mpevent::coordinator::Coordinator;

fn wait_on_event(event_name: &str) {
    let mut coordinator = Coordinator::new("example1");
    coordinator.add_participant("test_participant").unwrap();
    let mut waitable = coordinator.add_event(event_name).unwrap();

    waitable.wait(0);
    println!("Event {} received", event_name);
}

fn main() {
    let mut coordinator = Coordinator::new("example1");
    coordinator.add_participant("test_participant").unwrap();
    let mut waitable = coordinator.add_event("test_event").unwrap();

    // spawn a thread to wait on the futex
    let handle = std::thread::spawn(move || {
        wait_on_event("test_event");
    });

    waitable.wait(0);
    println!("Event received");
    handle.join().unwrap();
    let _ = coordinator.close(true);
}
