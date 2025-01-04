
use rushm::posixaccessor;

// C representation
#[repr(C)]
struct CentralData {
    first: i16,
    second: i8,
    third: i32
}

#[derive(Debug)]
pub(crate) struct Coordinator {
    mem_path: String,
}


impl Coordinator {
    pub(crate) fn new() -> Self {
        Coordinator {
            mem_path: String::new(),
        }
    }

    pub fn open_existing(&mut self, mem_path: &str) -> Result<(), String>
    {

        Ok(())
    }
}