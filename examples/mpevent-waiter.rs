use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use mpevent::coordinator::Coordinator;
use mpevent::subscriber::Subscriber;

fn wait_on_event(event_name: &str) {
    let mut coordinator = Coordinator::new("example1");
    coordinator.add_participant("test_participant").unwrap();
    let mut waitable = coordinator.add_event(event_name).unwrap();

    waitable.wait(0);
    println!("Event {} received", event_name);
}

fn main() {
    static MUST_RUN: AtomicBool = AtomicBool::new(true);

    let mut coordinator = Coordinator::new_clean("example1");
    coordinator.add_participant("test_participant").unwrap();

    let mut waitable = coordinator.add_event("test_event").unwrap();

    let must_run = Arc::new(&MUST_RUN).clone();
    // spawn a thread to wait on the futex
    let handle = std::thread::spawn(move || {
        wait_on_event("test_event");
        must_run.store(false, std::sync::atomic::Ordering::SeqCst);
    });

    let must_run = Arc::new(&MUST_RUN).clone();
    let handle2 = std::thread::spawn(move || {
        let mut sub = Subscriber::new("test_subscriber", "example1");

        while must_run.load(std::sync::atomic::Ordering::SeqCst) {
            sub.wait_on_event(mpevent::BUILTIN_EVENT_NEW_PARTICIPANT)
                .unwrap();
            println!("Notification of new participant added");
        }
    });

    let must_run = Arc::new(&MUST_RUN).clone();
    let handle3 = std::thread::spawn(move || {
        let mut sub = Subscriber::new("test_subscriber", "example1");

        while must_run.load(std::sync::atomic::Ordering::SeqCst) {
            sub.wait_on_event(mpevent::BUILTIN_EVENT_NEW_EVENT).unwrap();
            println!("Notification of new event created");
        }
    });

    let wait_time = libc::timespec {
        tv_sec: 5,
        tv_nsec: 0,
    };

    waitable.wait_with_timeout(0, wait_time);
    waitable.set_futex_value(0);
    println!("Event received");
    handle.join().unwrap();
    handle2.join().unwrap();
    handle3.join().unwrap();
    let _ = coordinator.close(true);
}
