use rufutex::rufutex::SharedFutex;
use rushm::posixaccessor;

const MAX_SUBSCRIBERS: usize = 64;
const MAX_EVENT_NAME_SIZE: usize = 64;
const MAX_EVENTS: usize = 64;
const MAX_SUBSCRIBER_NAME_SIZE: usize = 64;

// C representation
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Subscriber {
    id: u64,
    name: [u8; MAX_SUBSCRIBER_NAME_SIZE],
}

impl Subscriber {
    pub fn new() -> Self {
        Subscriber {
            id: 0,
            name: [0; MAX_SUBSCRIBER_NAME_SIZE],
        }
    }

    pub fn get_name(&self) -> String {
        let vname: Vec<u8> = self.name.iter().take_while(|&&c| c != 0).cloned().collect();
        let name = String::from_utf8(vname).unwrap();
        name
    }
}

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
}

// C representation
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Directory {
    last_subscriber_id: u64,
    last_event_id: u64,
    subscribers: [Subscriber; MAX_SUBSCRIBERS],
    events: [Event; MAX_EVENTS],
}

impl Directory {
    pub fn new() -> Self {
        Directory {
            last_subscriber_id: 0,
            last_event_id: 0,
            subscribers: [Subscriber {
                id: 0,
                name: [0; MAX_SUBSCRIBER_NAME_SIZE],
            }; MAX_SUBSCRIBERS],
            events: [Event {
                id: 0,
                name: [0; MAX_EVENT_NAME_SIZE],
            }; MAX_EVENTS],
        }
    }
}

pub struct Coordinator {
    mem_path: String,
    directory: *mut Directory,
    shm: posixaccessor::POSIXShm<Directory>,
    shm_mutex: posixaccessor::POSIXShm<i32>,
    mutex: rufutex::rufutex::SharedFutex,
}

impl Coordinator {
    pub fn new(mem_path: &str) -> Self {
        let mut tmp_shm = posixaccessor::POSIXShm::<Directory>::new(
            mem_path.to_string(),
            std::mem::size_of::<Directory>(),
        );
        let ret: Result<(), String>;

        unsafe {
            ret = tmp_shm.open();
        }

        if ret.is_err() {
            panic!("Error opening shared memory");
        }

        let ptr_data: *mut Directory = tmp_shm.get_as_mut();
        if ptr_data.is_null() {
            panic!("Error getting pointer to shared memory");
        }

        unsafe {
            *ptr_data = Directory::new();
        }

        let mutex_path = mem_path.to_string() + "_mutex";

        let mut shm_mutex =
            posixaccessor::POSIXShm::<i32>::new(mutex_path, std::mem::size_of::<i32>());
        unsafe {
            let ret = shm_mutex.open();
            if ret.is_err() {
                panic!("Error opening shared memory for mutex");
            }
        }

        let ptr_shm = shm_mutex.get_cptr_mut();
        let shared_futex = SharedFutex::new(ptr_shm);

        Coordinator {
            mem_path: mem_path.to_string(),
            directory: ptr_data,
            shm: posixaccessor::POSIXShm::new(
                mem_path.to_string(),
                std::mem::size_of::<Directory>(),
            ),
            shm_mutex,
            mutex: shared_futex,
        }
    }

    pub fn open_existing(mem_path: &str) -> Self {
        let mut tmp_shm = posixaccessor::POSIXShm::<Directory>::new(
            mem_path.to_string(),
            std::mem::size_of::<Directory>(),
        );
        let ret: Result<(), String>;

        unsafe {
            ret = tmp_shm.open();
        }

        if ret.is_err() {
            return Coordinator::new(mem_path);
        }

        let ptr_data: *mut Directory = tmp_shm.get_as_mut();

        let mutex_path = mem_path.to_string() + "_mutex";
        let mut shm_mutex =
            posixaccessor::POSIXShm::<i32>::new(mutex_path, std::mem::size_of::<i32>());
        unsafe {
            let ret = shm_mutex.open();
            if ret.is_err() {
                return Coordinator::new(mem_path);
            }
        }

        let ptr_shm = shm_mutex.get_cptr_mut();
        let shared_futex = SharedFutex::new(ptr_shm);

        Coordinator {
            mem_path: mem_path.to_string(),
            directory: ptr_data,
            shm: tmp_shm,
            shm_mutex,
            mutex: shared_futex,
        }
    }

    pub fn close(&mut self, unlink: bool) -> Result<(), String> {
        let ret: Result<(), String>;
        unsafe {
            ret = self.shm.close(unlink);
        }

        if ret.is_err() {
            return Err(String::from("Error closing shared memory"));
        }

        Ok(())
    }

    pub fn add_subscriber(&mut self, name: &str) -> Result<(), String> {
        let mut subscriber = Subscriber::new();
        self.mutex.lock();

        unsafe {
            (*self.directory).last_subscriber_id += 1;
            subscriber.id = (*self.directory).last_subscriber_id;
            let name_bytes = name.as_bytes();
            for i in 0..name_bytes.len() {
                subscriber.name[i] = name_bytes[i];
            }
            (*self.directory).subscribers[subscriber.id as usize] = subscriber;
        }
        self.mutex.unlock(0);
        Ok(())
    }

    pub fn add_event(&mut self, name: &str) -> Result<(), String> {
        self.mutex.lock();
        let mut event = Event::new();

        unsafe {
            (*self.directory).last_event_id += 1;
            event.id = (*self.directory).last_event_id;
            let name_bytes = name.as_bytes();
            for i in 0..name_bytes.len() {
                event.name[i] = name_bytes[i];
            }
            (*self.directory).events[event.id as usize] = event;
        }
        self.mutex.unlock(0);
        Ok(())
    }

    pub fn get_subscriber(&self, id: u64) -> Option<Subscriber> {
        if id > MAX_SUBSCRIBERS as u64 {
            return None;
        }

        unsafe {
            let subscriber = (*self.directory).subscribers[id as usize];
            Some(subscriber)
        }
    }
}

#[cfg(test)]
use std::ffi::CString;

#[test]
fn test_shared_memory_write_read() {
    let mut coordinator = Coordinator::new("test_shared_memory_coordinator");
    let mut coordinator2 = Coordinator::open_existing("test_shared_memory_coordinator");

    let ret = coordinator.add_subscriber("test_subscriber");
    assert!(ret.is_ok());
    let s = coordinator2.get_subscriber(1).unwrap();
    assert_eq!(s.id, 1);

    let vname: Vec<u8> = s.name.iter().take_while(|&&c| c != 0).cloned().collect();
    let name = CString::new(vname.to_vec()).unwrap();
    assert_eq!(name.to_str().unwrap(), "test_subscriber");

    let s = coordinator.get_subscriber(1).unwrap();
    assert_eq!(s.id, 1);
    let name = s.get_name();
    assert_eq!(name, "test_subscriber");

    let ret = coordinator2.close(false);
    assert!(ret.is_ok());
    let _ret = coordinator.close(true);
    // Dont check for error, since the shared memory is already unlinked
}

//pidfd = syscall(SYS_pidfd_open, shm_base->prod_pid, 0);
//event_fd = syscall(SYS_pidfd_getfd, pidfd, shm_base->event_fd, 0);
