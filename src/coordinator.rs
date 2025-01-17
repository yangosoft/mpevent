use crate::event::Event;
use log::{debug, error};
use rufutex::rufutex::SharedFutex;
use rushm::posixaccessor;

use std::fs;
use std::path::Path;

use crate::MAX_EVENTS;
use crate::MAX_PARTICIPANTS;
use crate::MAX_PARTICIPANT_NAME_SIZE;
use crate::{BUILTIN_EVENT_NEW_EVENT, BUILTIN_EVENT_NEW_PARTICIPANT};

// C representation
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Participant {
    id: u64,
    name: [u8; MAX_PARTICIPANT_NAME_SIZE],
}

impl Participant {
    pub fn new() -> Self {
        Participant {
            id: 0,
            name: [0; MAX_PARTICIPANT_NAME_SIZE],
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
    last_participant_id: u64,
    last_event_id: u64,
    participants: [Participant; MAX_PARTICIPANTS],
    events: [Event; MAX_EVENTS],
    events_owners: [u64; MAX_EVENTS],
}

impl Directory {
    pub fn new() -> Self {
        Directory {
            last_participant_id: 0,
            last_event_id: 0,
            participants: [Participant {
                id: 0,
                name: [0; MAX_PARTICIPANT_NAME_SIZE],
            }; MAX_PARTICIPANTS],
            events: [Event::new(); MAX_EVENTS],
            events_owners: [0; MAX_EVENTS],
        }
    }
}

fn clean_shared_files(path: &str, mem_path: &str) {
    // First unlink all the files under /dev/shm/
    let shm_path = Path::new(path);
    if let Ok(entries) = fs::read_dir(shm_path) {
        for entry in entries {
            if let Ok(entry) = entry {
                if let Ok(file_name) = entry.file_name().into_string() {
                    if file_name.contains(mem_path) {
                        let file_path = shm_path.join(file_name);
                        let _ = fs::remove_file(file_path);
                    }
                }
            }
        }
    }
}

pub struct Coordinator {
    mem_path: String,
    directory: *mut Directory,
    shm: posixaccessor::POSIXShm<Directory>,
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
            error!("Error opening shared memory");
            panic!("Error opening shared memory");
        }

        let ptr_data: *mut Directory = tmp_shm.get_as_mut();
        if ptr_data.is_null() {
            error!("Error getting pointer to shared memory");
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
                error!("Error opening shared memory for mutex");
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
            mutex: shared_futex,
        }
    }

    pub fn new_clean(mem_path: &str) -> Self {
        clean_shared_files("/dev/shm", mem_path);

        Coordinator::new(mem_path)
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
            mutex: shared_futex,
        }
    }

    pub fn close(&mut self, unlink: bool) -> Result<(), String> {
        let ret: Result<(), String>;

        // Send fake events to possibly unblock waiters
        // Notify with internal event
        let _ = self.notify_builtin(BUILTIN_EVENT_NEW_PARTICIPANT);
        let _ = self.notify_builtin(BUILTIN_EVENT_NEW_EVENT);

        unsafe {
            ret = self.shm.close(unlink);
        }

        if ret.is_err() {
            return Err(String::from("Error closing shared memory"));
        }

        Ok(())
    }

    pub fn get_number_of_participants(&self) -> u64 {
        unsafe { (*self.directory).last_participant_id }
    }

    pub fn get_number_of_events(&self) -> u64 {
        unsafe { (*self.directory).last_event_id }
    }

    pub fn get_path(&self) -> String {
        self.mem_path.clone()
    }

    fn notify_builtin(&mut self, event_name: &str) -> Result<(), String> {
        let event_name = self.mem_path.to_string() + "_" + event_name;
        debug!("   |-> Notifying builtin event. {}", event_name);
        let mut event = Event::new();

        let ret = event.set_name(event_name.as_str());
        if ret.is_err() {
            return Err(String::from("Error setting event name"));
        }
        event.set_id(42);

        let waitable = event.get_waitable();
        if waitable.is_none() {
            return Err(String::from("Error creating waitable"));
        }
        let mut waitable = waitable.unwrap();
        debug!(
            "   |-> Notifying builtin event {}. Old value {}",
            event_name,
            waitable.get_futex_value()
        );

        waitable.set_futex_value(0);
        waitable.post_with_value(1, u32::max_value());

        Ok(())
    }

    pub fn add_participant(&mut self, name: &str) -> Result<u64, String> {
        debug!("Creating new participant '{}'", name);
        let mut participant = Participant::new();
        self.mutex.lock();

        let max_id = unsafe { (*self.directory).last_participant_id };
        if max_id >= MAX_PARTICIPANTS as u64 {
            self.mutex.unlock(1);
            log::error!("Max number of participants reached");
            return Err(String::from("Max number of participants reached"));
        }

        // Check if participant already exists
        for i in 0..max_id {
            let p = unsafe { (*self.directory).participants[i as usize] };
            let p_name = p.get_name();
            if p_name == name {
                self.mutex.unlock(1);
                log::error!("Participant already exists");
                return Err(String::from("Participant already exists"));
            }
        }

        unsafe {
            participant.id = (*self.directory).last_participant_id;
            let name_bytes = name.as_bytes();
            for i in 0..name_bytes.len() {
                participant.name[i] = name_bytes[i];
            }
            (*self.directory).participants[participant.id as usize] = participant;
            (*self.directory).last_participant_id += 1;
            debug!(
                " |-> Participant created with id {}. Next id: {}",
                participant.id,
                (*self.directory).last_participant_id
            );
        }
        self.mutex.unlock(1);

        // Notify with internal event
        debug!(" |-> Notifying new participant");
        let _ = self.notify_builtin(BUILTIN_EVENT_NEW_PARTICIPANT);

        Ok(participant.id)
    }

    pub fn add_event(&mut self, participant_id: u64, name: &str) -> Result<SharedFutex, String> {
        self.mutex.lock();

        let max_id = unsafe { (*self.directory).last_event_id };

        if max_id >= MAX_EVENTS as u64 {
            self.mutex.unlock(1);
            return Err(String::from("Max number of events reached"));
        }

        // Prepend coordinator name to event name
        let name = self.mem_path.to_string() + "_" + name;
        debug!("|-> Creating new event '{}'", name);

        let mut exists = false;
        // Check if event already exists
        for i in 0..max_id {
            let e = unsafe { (*self.directory).events[i as usize] };
            let e_name = e.get_name();
            if e_name == name {
                exists = true;
                break;
            }
        }

        let mut event = Event::new();

        unsafe {
            let new_id = (*self.directory).last_event_id;
            event.set_id(new_id);
            let ret = event.set_name(name.as_str());
            if ret.is_err() {
                self.mutex.unlock(1);
                return Err(String::from("Error setting event name"));
            }

            if !exists {
                (*self.directory).events[new_id as usize] = event;
                (*self.directory).events_owners[new_id as usize] = participant_id as u64;
                (*self.directory).last_event_id += 1;
            }
        }
        self.mutex.unlock(1);
        let waitable = event.get_waitable();
        if waitable.is_none() {
            return Err(String::from("Error creating waitable"));
        }
        // Notify with internal event
        let _ = self.notify_builtin(BUILTIN_EVENT_NEW_EVENT);

        Ok(waitable.unwrap())
    }

    pub fn get_participant(&self, id: u64) -> Option<Participant> {
        if id > MAX_PARTICIPANTS as u64 {
            return None;
        }

        unsafe {
            let participant = (*self.directory).participants[id as usize];
            Some(participant)
        }
    }

    pub fn get_last_event_id(&mut self) -> Option<u64> {
        self.mutex.lock();
        let current_id = unsafe { (*self.directory).last_event_id };
        if current_id == 0 {
            self.mutex.unlock(1);
            return None;
        }
        self.mutex.unlock(1);
        Some(current_id - 1)
    }

    pub fn get_last_participant_id(&mut self) -> Option<u64> {
        self.mutex.lock();
        let current_id = unsafe { (*self.directory).last_participant_id };
        if current_id == 0 {
            self.mutex.unlock(1);
            return None;
        }
        self.mutex.unlock(1);
        Some(current_id - 1)
    }

    pub fn get_participant_id_by_event_id(&self, event_id: u64) -> Option<u64> {
        if event_id > MAX_EVENTS as u64 {
            return None;
        }

        unsafe {
            let participant_id = (*self.directory).events_owners[event_id as usize];
            Some(participant_id)
        }
    }
}

#[cfg(test)]
use std::ffi::CString;

#[test]
fn test_shared_memory_write_read() {
    let mut coordinator = Coordinator::new("test_shared_memory_write_read");
    let mut coordinator2 = Coordinator::open_existing("test_shared_memory_write_read");

    let ret = coordinator.add_participant("test_participant");
    assert!(ret.is_ok());
    let num_participants = coordinator2.get_number_of_participants();
    assert_eq!(num_participants, 1);
    let s = coordinator2.get_participant(0).unwrap();
    assert_eq!(s.id, 0);

    let vname: Vec<u8> = s.name.iter().take_while(|&&c| c != 0).cloned().collect();
    let name = CString::new(vname.to_vec()).unwrap();
    assert_eq!(name.to_str().unwrap(), "test_participant");

    let s = coordinator.get_participant(0).unwrap();
    assert_eq!(s.id, 0);
    let name = s.get_name();
    assert_eq!(name, "test_participant");

    let ret = coordinator2.close(false);
    assert!(ret.is_ok());
    let _ret = coordinator.close(true);
    // Dont check for error, since the shared memory is already unlinked
}

#[test]
fn test_events() {
    let mut coordinator = Coordinator::new("test_events");
    let mut coordinator2 = Coordinator::open_existing("test_events");
    let participant_id = coordinator
        .add_participant("test_shared_memory_participant")
        .unwrap();
    let ret = coordinator.add_event(participant_id, "test_event");
    assert!(ret.is_ok());
    let num_events = coordinator2.get_number_of_events();
    assert_eq!(num_events, 1);
    let ret = coordinator2.add_event(participant_id, "test_event");
    assert!(ret.is_ok());
    let num_events = coordinator2.get_number_of_events();
    assert_eq!(num_events, 1);
    let ret = coordinator2.add_event(participant_id, "test_event");
    assert!(ret.is_ok());
    let num_events = coordinator2.get_number_of_events();
    assert_eq!(num_events, 1);

    let ret = coordinator2.close(false);
    assert!(ret.is_ok());
    let _ret = coordinator.close(true);
    // Dont check for error, since the shared memory is already unlinked
}

//pidfd = syscall(SYS_pidfd_open, shm_base->prod_pid, 0);
//event_fd = syscall(SYS_pidfd_getfd, pidfd, shm_base->event_fd, 0);
