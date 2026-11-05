//! A very cautious file organizer. Plan first, show the plan, then move only what is still exactly
//! as planned, record every step so it can be undone, and do nothing when unsure.

pub mod classify;
pub mod exec;
pub mod journal;
pub mod model;
pub mod pathutil;
pub mod safety;
pub mod scan;
pub mod settle;
pub mod winfs;

pub use model::{
    Category, ConflictPolicy, Fingerprint, Move, Options, OrganizerError, Plan, Skip, SkipCode,
};
