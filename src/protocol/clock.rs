use std::time::SystemTime;

use super::Slot;

/// Represents a clock that manages and emits the current slot number based on system time.
pub struct Clock {
    /// The last slot number that was emitted by the clock.
    last_emitted: Slot,
}

impl Clock {
    /// Constructs a new `Clock` instance.
    pub fn new() -> Self {
        Self {
            last_emitted: Slot::zero(),
        }
    }

    /// Returns the current slot number, never returning the same slot twice.
    pub async fn tick(&mut self) -> Slot {
        let now = SystemTime::now();
        let cur_slot = Slot::from_system_time(now);

        // If the current slot is greater than the last emitted, update and return it.
        if self.last_emitted < cur_slot {
            self.last_emitted = cur_slot;
            return cur_slot;
        }

        // Calculate the next slot and the time duration until it starts.
        let next_slot = cur_slot.next();
        let til_next = next_slot.to_system_time().duration_since(now).unwrap();

        // Sleep until the next slot time is reached.
        tokio::time::sleep(til_next).await;

        // Update the last emitted slot to the next slot and return it.
        self.last_emitted = next_slot;
        next_slot
    }
}
