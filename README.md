# mpevent
![mepevent workflow](https://github.com/yangosoft/mpevent/actions/workflows/rust.yml/badge.svg)
[![crates.io](https://img.shields.io/crates/v/mpevent.svg)](https://crates.io/crates/mpevent)
[![documentation](https://img.shields.io/badge/docs-live-brightgreen)](https://docs.rs/mpevent)


Rust multiprocess event manager (only for Linux)

This crate provides a very simple way of notifying processes using POSIX shared memory and futexes.

A `Coordinator` is instantiated to create or join the notification group.

Events can be added using the `Coordinator`.
A `Participant` can subscribe to events and/or publish them.

See the [examples](examples) folder for usage.

Event waiting

```
    // 
    let mut coordinator = Coordinator::new_clean("example1");
     let mut waitable = coordinator
        .add_event(fixed_participant_id, "test_event")
        .unwrap();

    let wait_time = libc::timespec {
        tv_sec: 15,
        tv_nsec: 0,
    };

    // Events can be waited with a timeout or completely block until they are recevied
    waitable.wait_with_timeout(0, wait_time);
    let has_timeout = waitable.get_futex_value() == 0;
    waitable.set_futex_value(0);
    println!("Event received. Has timeout? {}", has_timeout);
    let _ = coordinator.close(true);
```

Event publishing

```
    let mut coordinator = Coordinator::open_existing("example1");
    let participant_id = coordinator.add_participant("test_participant2").unwrap();

    // Directly manage the futex and notify up to 1000 waiters 
    let mut waitable = coordinator.add_event(participant_id, "test_event").unwrap();
    waitable.post_with_value(1, 1000);

    println!("Event test_event posted");

    // Use the Participant abstraction to trigger a new event
    let mut publisher = Participant::new("test_notifier", "example1");
    publisher
        .trigger_event("test_event", std::u32::MAX)
        .unwrap();
```





