// This example demonstrates how to use the mpevent crate to create a simple event coordinator and a subscriber that listens for events.
// The coordinator creates an event and the subscriber listens for it.
// The subscriber is notified when a new participant is created.
// The subscriber is notified when a new event is created.
// The subscriber is notified when a new event is posted.

use mpevent::coordinator::Coordinator;
use mpevent::subscriber::Subscriber;

fn main() {
    let mut coordinator = Coordinator::open_existing("example1");
    let participant_id = coordinator.add_participant("test_participant2").unwrap();

    let mut waitable = coordinator.add_event(participant_id, "test_event").unwrap();
    waitable.post_with_value(1, 1000);

    println!("Event test_event posted");

    let mut publisher = Subscriber::new("test_notifier", "example1");
    publisher
        .trigger_event("test_event", std::u32::MAX)
        .unwrap();

    let _ = coordinator
        .add_event(participant_id, "test_event2")
        .unwrap();
}
