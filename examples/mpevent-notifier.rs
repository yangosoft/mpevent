use mpevent::coordinator::Coordinator;

fn main() {
    let mut coordinator = Coordinator::new("example1");
    let participant_id = coordinator.add_participant("test_participant2").unwrap();

    let mut waitable = coordinator.add_event(participant_id, "test_event").unwrap();
    waitable.post_with_value(1, 1000);

    println!("Event test_event posted");
}
