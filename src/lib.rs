const MAX_PARTICIPANTS: usize = 64;
const MAX_EVENT_NAME_SIZE: usize = 256;
const MAX_EVENTS: usize = 64;
const MAX_PARTICIPANT_NAME_SIZE: usize = 64;

pub mod coordinator;
pub mod event;
pub mod subscriber;
