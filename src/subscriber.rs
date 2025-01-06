use crate::coordinator::Coordinator;
use std::{borrow::BorrowMut, collections::HashMap};

pub struct Subscriber {
    id: u64,
    name: String,
    coordinator: Coordinator,
    map_events: HashMap<String, rufutex::rufutex::SharedFutex>,
}

impl Subscriber {
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

        let ret = self.coordinator.add_event(event_name);
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
}
