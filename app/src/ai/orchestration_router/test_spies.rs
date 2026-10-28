#[cfg(test)]
use std::sync::atomic::AtomicUsize;

#[cfg(test)]
pub static TEST_ACTION_DISPATCH_COUNT: AtomicUsize = AtomicUsize::new(0);

#[cfg(test)]
pub static TEST_EXECUTOR_COUNT: AtomicUsize = AtomicUsize::new(0);

#[cfg(test)]
pub static TEST_FOLLOWUP_REQUEST_COUNT: AtomicUsize = AtomicUsize::new(0);

#[cfg(test)]
pub static TEST_CHILD_START_COUNT: AtomicUsize = AtomicUsize::new(0);

#[cfg(test)]
pub static TEST_ROUTER_EFFECT_COUNT: AtomicUsize = AtomicUsize::new(0);
