//! Monitor for object synchronization (monitorenter/monitorexit).

/// Monitor for object synchronization
#[derive(Debug, Clone)]
pub struct Monitor {
    pub owner: Option<u32>,
    pub count: u32,
    pub waiters: Vec<u32>,
}

impl Monitor {
    pub fn new() -> Self {
        Self {
            owner: None,
            count: 0,
            waiters: Vec::new(),
        }
    }

    pub fn enter(&mut self, thread_id: u32) -> bool {
        if let Some(owner) = self.owner {
            if owner == thread_id {
                self.count += 1;
                true
            } else {
                self.waiters.push(thread_id);
                false
            }
        } else {
            self.owner = Some(thread_id);
            self.count = 1;
            true
        }
    }

    pub fn exit(&mut self, thread_id: u32) -> bool {
        if let Some(owner) = self.owner {
            if owner == thread_id {
                self.count -= 1;
                if self.count == 0 {
                    self.owner = None;
                    if let Some(next_thread) = self.waiters.first() {
                        let next_thread = *next_thread;
                        self.waiters.remove(0);
                        self.owner = Some(next_thread);
                        self.count = 1;
                    }
                }
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn is_owned_by(&self, thread_id: u32) -> bool {
        self.owner.map_or(false, |owner| owner == thread_id)
    }

    pub fn wait(&mut self, thread_id: u32) -> bool {
        if let Some(owner) = self.owner {
            if owner == thread_id {
                self.waiters.push(thread_id);
                let old_count = self.count;
                self.owner = None;
                self.count = 0;

                if let Some(next_thread) = self.waiters.first() {
                    let next_thread = *next_thread;
                    self.waiters.remove(0);
                    self.owner = Some(next_thread);
                    self.count = 1;
                }

                old_count > 0
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn notify(&mut self) -> bool {
        if self.waiters.is_empty() {
            false
        } else {
            if let Some(waiter) = self.waiters.first() {
                let waiter = *waiter;
                self.waiters.remove(0);
                self.waiters.push(waiter);
                true
            } else {
                false
            }
        }
    }

    pub fn notify_all(&mut self) -> usize {
        let count = self.waiters.len();
        if count > 0 {
            self.waiters.rotate_right(1);
        }
        count
    }
}

impl Default for Monitor {
    fn default() -> Self {
        Self::new()
    }
}
