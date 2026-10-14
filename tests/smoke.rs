//! Opt-in, unchecked crash-hunting sweep over the API generated for this build.
//! Run only in a disposable process; this deliberately violates native preconditions.

#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/smoke/smoke.rs"));
