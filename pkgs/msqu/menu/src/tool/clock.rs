//! msqu clock: the time somewhere else, up to MSQU_SLOTS at once, kept
//! across reboots.
//!
//!   msqu clock add ZONE [LABEL [SLOT]]   ZONE: "Asia/Tokyo"
//!   msqu clock remove SLOT   (middle-clicking its block)
//!   msqu clock middle-click SLOT   hides its block
//!   msqu clock running       exit 0 if there's a clock
//!
//! state file per slot ("1", "2", ...): "zone ZONE LABEL"

use super::{fields, State};
use crate::model::{zone_city, zone_time, zones};

pub fn main(args: &[String]) -> i32 {
    let mut st = State::open("clock");
    let arg = |i: usize| args.get(i).map(String::as_str).unwrap_or("");
    match arg(0) {
        "add" | "pin" => {
            let zone = arg(1);
            if !zones().iter().any(|z| z == zone) || zone_time(zone, 0).is_none() {
                eprintln!("msqu clock: no time zone {zone:?} (like Asia/Tokyo)");
                return 2;
            }
            // there already: leave it
            let slots: Vec<String> = (1..=9).map(|i| i.to_string()).collect();
            if arg(3).is_empty() && slots.iter().any(|s| fields(&st.read(s), 3)[1] == zone) {
                return 0;
            }
            if !arg(3).is_empty() {
                if let Err(code) = st.use_slot(arg(3)) {
                    return code;
                }
            } else {
                let Some(free) = st.free_slot() else {
                    let n = std::env::var("MSQU_SLOTS").unwrap_or_default();
                    eprintln!("{n} clocks already: remove one first");
                    return 1;
                };
                st.slot = free;
            }
            let label = if arg(2).is_empty() { zone_city(zone) } else { arg(2).to_string() };
            st.put(&st.slot, &format!("zone {zone} {label}"));
        }
        "remove" | "cancel" | "right-click" => {
            if let Err(code) = st.use_slot(arg(1)) {
                return code;
            }
            st.remove(&st.slot);
        }
        "running" => return if st.any_slot() { 0 } else { 1 },
        "middle-click" => {
            if let Err(code) = st.use_slot(arg(1)) {
                return code;
            }
            crate::model::set_mark("hide", st.tool, &st.slot, true);
        }
        "click" => {}
        _ => {
            eprintln!("usage: msqu clock [add ZONE [LABEL [SLOT]]|remove SLOT|running]");
            return 2;
        }
    }
    0
}
