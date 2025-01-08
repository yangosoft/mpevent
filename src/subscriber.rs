use crate::coordinator::Coordinator;
use std::collections::HashMap;

pub struct Subscriber<'a> {
    id: u64,
    name: String,
    coordinator: Coordinator,
    map_events: HashMap<String, rufutex::rufutex::SharedFutex>,
    on_new_event: Box<dyn FnMut(u64) + 'a>,
    on_new_participant: Box<dyn FnMut(u64) + 'a>,
}

impl<'a> Subscriber<'a> {
    pub fn new(name: &str, mem_path: &str) -> Self {
        let mut coordinator = Coordinator::new(mem_path);
        let ret = coordinator.add_participant(name);
        if ret.is_err() {
            panic!("Failed to add participant");
        }

        let map_events = HashMap::new();

        Subscriber {
            id: ret.unwrap(),
            name: name.to_string(),
            coordinator,
            map_events,
            on_new_event: Box::new(|_| {}),
            on_new_participant: Box::new(|_| {}),
        }
    }

    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    pub fn get_id(&self) -> u64 {
        self.id
    }

    pub fn set_id(&mut self, id: u64) {
        self.id = id;
    }

    pub fn get_coordinator(&self) -> &Coordinator {
        &self.coordinator
    }

    fn get_or_create_event(
        &mut self,
        event_name: &str,
    ) -> Result<&mut rufutex::rufutex::SharedFutex, String> {
        if self.map_events.contains_key(event_name) {
            return Ok(self.map_events.get_mut(event_name).unwrap());
        }

        let ret = self.coordinator.add_event(self.id, event_name);
        if ret.is_err() {
            return Err(String::from("Error adding event"));
        }
        let event = ret.unwrap();
        self.map_events.insert(event_name.to_string(), event);
        Ok(self.map_events.get_mut(event_name).unwrap())
    }

    pub fn trigger_event(
        &mut self,
        event_name: &str,
        number_of_waiters: u32,
    ) -> Result<(), String> {
        let ret = self.get_or_create_event(event_name);
        if ret.is_err() {
            return Err(String::from("Error getting or creating event"));
        }

        let value = 1;
        let event: &mut rufutex::rufutex::SharedFutex = ret.unwrap();
        event.post_with_value(value, number_of_waiters);

        Ok(())
    }

    pub fn wait_on_event(&mut self, event_name: &str) -> Result<(), String> {
        let ret = self.get_or_create_event(event_name);
        if ret.is_err() {
            return Err(String::from("Error getting or creating event"));
        }

        let event: &mut rufutex::rufutex::SharedFutex = ret.unwrap();
        event.wait(0);
        let v = event.get_futex_value();
        if v == 0 {
            //It was spurious wake up
            return Err(String::from("Error waiting on event"));
        }
        event.set_futex_value(0);

        Ok(())
    }

    pub fn wait_on_event_timeout(
        &mut self,
        event_name: &str,
        timeout: std::time::Duration,
    ) -> Result<(), String> {
        let ret = self.get_or_create_event(event_name);
        if ret.is_err() {
            return Err(String::from("Error getting or creating event"));
        }
        let timeout_spec = libc::timespec {
            tv_sec: timeout.as_secs() as i64,
            tv_nsec: timeout.subsec_nanos() as i64,
        };

        let event: &mut rufutex::rufutex::SharedFutex = ret.unwrap();
        event.wait_with_timeout(0, timeout_spec);
        event.set_futex_value(0);

        Ok(())
    }

    pub fn set_on_create_event_callback(&mut self, c: impl FnMut(u64) + 'a) {
        self.on_new_event = Box::new(c);
    }

    pub fn set_on_create_participant_callback(&mut self, c: impl FnMut(u64) + 'a) {
        self.on_new_participant = Box::new(c);
    }

    pub fn wait_on_new_event(&mut self) -> Result<(), String> {
        loop {
            //Check which was the last event
            let last_event_id = self.coordinator.get_last_event_id();

            let ret = self.wait_on_event(crate::BUILTIN_EVENT_NEW_EVENT);
            if ret.is_err() {
                //Spurious wake up
                continue;
            }
            let current_event_id = self.coordinator.get_last_event_id();
            if current_event_id != last_event_id {
                if current_event_id.is_none() {
                    continue;
                }
                //Check if the event was triggered by the subscriber itself
                let event = self
                    .coordinator
                    .get_participant_id_by_event_id(current_event_id.unwrap());
                match event {
                    Some(id) => {
                        if id == self.id {
                            continue;
                        }
                    }
                    None => {
                        continue;
                    }
                }
                self.on_new_event.as_mut()(current_event_id.unwrap());
                break;
            }
        }

        Ok(())
    }

    pub fn wait_on_new_participant(&mut self) -> Result<(), String> {
        loop {
            //Check which was the last event
            let last_participant_id = self.coordinator.get_last_participant_id();

            let ret = self.wait_on_event(crate::BUILTIN_EVENT_NEW_PARTICIPANT);
            if ret.is_err() {
                //Spurious wake up
                continue;
            }
            let current_participant_id = self.coordinator.get_last_participant_id();
            if current_participant_id != last_participant_id {
                if current_participant_id.is_none() {
                    continue;
                }
                //Check if the event was triggered by the subscriber itself
                let id = current_participant_id.unwrap();
                if id == self.id {
                    continue;
                }
                self.on_new_participant.as_mut()(id);
                break;
            }
        }
        Ok(())
    }

    pub fn close(&mut self) -> Result<(), String> {
        self.coordinator.close(true)
    }
}

#[cfg(test)]
#[test]
fn test_subscriber() {
    let mut subscriber = Subscriber::new("test_subscriber", "test_mem_path");
    assert_eq!(subscriber.get_id(), 0);
    assert_eq!(subscriber.get_name(), "test_subscriber");

    // spawn a thread to wait on the futex
    let handle = std::thread::spawn(move || {
        let mut subscriber = Subscriber::new("test_subscriber2", "test_mem_path");
        assert_eq!(subscriber.get_id(), 0);
        assert_eq!(subscriber.get_name(), "test_subscriber2");
        let ret = subscriber.wait_on_event("test_subscribers");
        assert!(ret.is_ok());
        let ret = subscriber.wait_on_event("test_subscribers");
        assert!(ret.is_ok());
    });

    // Sleep for a bit to allow the thread to start
    std::thread::sleep(std::time::Duration::from_millis(100));
    let ret = subscriber.trigger_event("test_subscribers", u32::max_value());
    assert!(ret.is_ok());
    std::thread::sleep(std::time::Duration::from_millis(100));
    let ret = subscriber.trigger_event("test_subscribers", u32::max_value());
    assert!(ret.is_ok());
    handle.join().unwrap();
    let _ = subscriber.close();
}
