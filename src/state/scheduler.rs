use crate::protocol::{Clock, LeaderSchedule};

pub struct Scheduler {

}

impl Scheduler {
    pub fn new() -> Self {
        Self {}
    }
    pub async fn start(self) {
        let clock = Clock::new();
        let leader_schedule = LeaderSchedule::empty();
    }
}

