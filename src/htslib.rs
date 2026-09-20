//! Raw bindings to the HTSlib sources bundled with this package.
#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case)]
#![allow(clippy::all, improper_ctypes)]
// bindgen 0.72 emits integer transmutes for C bitfield accessors.
#![allow(unnecessary_transmutes)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
