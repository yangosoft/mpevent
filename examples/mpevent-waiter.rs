use mpevent::coordinator::Coordinator;
use mpevent::subscriber::Subscriber;

fn wait_on_event(participant_id: u64, event_name: &str) {
    let mut coordinator = Coordinator::new("example1");
    coordinator.add_participant("test_participant").unwrap();
    let mut waitable = coordinator.add_event(participant_id, event_name).unwrap();

    waitable.wait(0);
    println!("Event {} received", event_name);
}

fn main() {
    let mut coordinator = Coordinator::new_clean("example1");
    let fixed_participant_id = coordinator.add_participant("test_participant").unwrap();

    let handle3 = std::thread::spawn(move || {
        let mut sub = Subscriber::new("test_subscriber", "example1");

        sub.set_on_create_participant_callback(move |participant_id: u64| {
            println!("New participant {} created and is not me!", participant_id);
        });
        //println!("Waiting for new participant event");
        sub.wait_on_new_participant().unwrap();
        println!("New participant event received");
    });

    /*let mut waitable = coordinator
            .add_event(fixed_participant_id, "test_event")
            .unwrap();

        let wait_time = libc::timespec {
            tv_sec: 15,
            tv_nsec: 0,
        };

        waitable.wait_with_timeout(0, wait_time);
        let has_timeout = waitable.get_futex_value() == 0;
        waitable.set_futex_value(0);
        println!("Event received. Has timeout? {}", has_timeout);
    */
    handle3.join().unwrap();
    let _ = coordinator.close(true);
}
