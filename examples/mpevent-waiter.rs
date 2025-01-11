use mpevent::coordinator::Coordinator;
use mpevent::subscriber::Subscriber;

fn main() {
    let mut coordinator = Coordinator::new_clean("example1");
    let fixed_participant_id = coordinator.add_participant("test_participant").unwrap();

    let handle = std::thread::spawn(move || {
        let mut sub = Subscriber::new("test_subscriber", "example1");

        sub.set_on_create_participant_callback(move |participant_id: u64| {
            println!(
                " ######## New participant {} created and is not me!",
                participant_id
            );
        });
        //println!("Waiting for new participant event");
        sub.wait_on_new_participant().unwrap();
        println!("New participant event received");
    });

    let handle2 = std::thread::spawn(move || {
        let mut sub = Subscriber::new("test_subscriber", "example1");

        sub.set_on_create_event_callback(move |event_id: u64| {
            println!(" ######## New event id {} created and is not me!", event_id);
        });

        sub.wait_on_new_event().unwrap();
    });

    let mut waitable = coordinator
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

    handle.join().unwrap();
    handle2.join().unwrap();
    let _ = coordinator.close(true);
}
