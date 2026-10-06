// Compile the actual job module independently of renderer/audio dependencies.
//! A scope cannot borrow a local destroyed at the end of its body:
//! ```compile_fail,E0597
//! use job_runtime_tests::job::JobSystem;
//! let system = JobSystem::new(1, 1, 0).unwrap();
//! unsafe { system.handle().scope(|scope| {
//!     let mut local = 0;
//!     scope.spawn(&scope.group(), || { local = 1; }).unwrap();
//! }); }
//! ```
//! The scope itself cannot escape:
//! ```compile_fail
//! use job_runtime_tests::job::JobSystem;
//! let system = JobSystem::new(1, 1, 0).unwrap();
//! let escaped = unsafe { system.handle().scope(|scope| scope) };
//! ```
//! Owned jobs cannot borrow caller data:
//! ```compile_fail,E0373
//! use job_runtime_tests::job::{JobSystem, JobGroup};
//! let system = JobSystem::new(1, 1, 0).unwrap();
//! let local = 42;
//! system.handle().spawn(&JobGroup::new(), || { println!("{local}"); });
//! ```
#[path = "../mod.rs"]
pub mod job;
