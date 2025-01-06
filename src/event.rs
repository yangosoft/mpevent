use crate::MAX_EVENT_NAME_SIZE;

use rufutex::rufutex::SharedFutex;
use rushm::posixaccessor::POSIXShm;

// C representation
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Event {
    id: u64,
    name: [u8; MAX_EVENT_NAME_SIZE],
}

impl Event {
    pub fn new() -> Self {
        Event {
            id: 0,
            name: [0; MAX_EVENT_NAME_SIZE],
        }
    }

    pub fn get_name(&self) -> String {
        let vname: Vec<u8> = self.name.iter().take_while(|&&c| c != 0).cloned().collect();
        let name = String::from_utf8(vname).unwrap();
        name
    }

    pub fn get_id(&self) -> u64 {
        self.id
    }

    pub fn set_id(&mut self, id: u64) {
        self.id = id;
    }

    pub fn set_name(&mut self, name: &str) -> Result<(), &'static str> {
        if name.len() > MAX_EVENT_NAME_SIZE {
            return Err("Name too long");
        }
        let bytes = name.as_bytes();
        for (i, byte) in bytes.iter().enumerate() {
            self.name[i] = *byte;
        }
        Ok(())
    }

    pub fn get_waitable(&self) -> Option<SharedFutex> {
        let name = self.get_name();
        if name.is_empty() {
            return None;
        }

        let mut shm = POSIXShm::<i64>::new(name.to_string(), std::mem::size_of::<i64>());
        unsafe {
            let ret = shm.open();
            if ret.is_err() {
                return None;
            }
        }
        let ptr_shm = shm.get_cptr_mut();
        let shared_futex = SharedFutex::new(ptr_shm);
        Some(shared_futex)
    }
}

#[cfg(test)]
#[test]
fn test_event() {
    let mut event = Event::new();
    assert_eq!(event.get_id(), 0);
    assert_eq!(event.get_name(), "");

    event.set_id(42);
    assert_eq!(event.get_id(), 42);

    let ret = event.set_name("test_event");
    assert!(ret.is_ok());
    assert_eq!(event.get_name(), "test_event");

    let shared_futex = event.get_waitable();
    assert!(shared_futex.is_some());

    let mut shared_futex = shared_futex.unwrap();
    shared_futex.wait(1);
    shared_futex.post(0);
}

#[test]
fn test_events() {
    let mut event = Event::new();
    assert_eq!(event.get_id(), 0);
    assert_eq!(event.get_name(), "");

    event.set_id(42);
    assert_eq!(event.get_id(), 42);

    let ret = event.set_name("test_event2");
    assert!(ret.is_ok());
    assert_eq!(event.get_name(), "test_event2");

    let shared_futex = event.get_waitable();
    assert!(shared_futex.is_some());

    let mut shared_futex = shared_futex.unwrap();

    // spawn a thread to wait on the futex
    let handle = std::thread::spawn(move || {
        let mut event = Event::new();
        assert_eq!(event.get_id(), 0);
        assert_eq!(event.get_name(), "");

        event.set_id(42);
        assert_eq!(event.get_id(), 42);

        let ret = event.set_name("test_event2");
        assert!(ret.is_ok());
        assert_eq!(event.get_name(), "test_event2");

        let shared_futex2 = event.get_waitable();
        assert!(shared_futex2.is_some());

        let mut shared_futex2 = shared_futex2.unwrap();
        println!("Waiting on futex for 42");
        shared_futex2.wait(42);
        println!("Posting on futex for 1");
        shared_futex2.post_with_value(1, 32);
    });

    //Sleep for a while to let the thread start
    std::thread::sleep(std::time::Duration::from_millis(300));

    println!("Posting on futex for 42");
    shared_futex.post_with_value(42, 32);
    println!("Waiting on futex for 1");
    shared_futex.wait(1);
    println!("Wakeup on futex from 1");
    println!("Posting on futex for 42");
    shared_futex.post_with_value(42, 32);

    handle.join().unwrap();
}
