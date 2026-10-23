//! msqu-tool: the msqu tools in one program (run as msqu-TOOL).
//!
//! Usage: msqu-tool TOOL [args]   tools: stopwatch, drink, alarm, timer, repeat, countdown, clock, pick

use msqu_ui::tool;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        Some("stopwatch") => tool::stopwatch::main(&args[1..]),
        Some("drink") => tool::drink::main(&args[1..]),
        Some("alarm") => tool::alarm::main(&args[1..]),
        Some("timer") => tool::timer::main(&args[1..]),
        Some("repeat") => tool::repeat::main(&args[1..]),
        Some("countdown") => tool::countdown::main(&args[1..]),
        Some("clock") => tool::clock::main(&args[1..]),
        Some("pick") => tool::pick::main(&args[1..]),
        _ => {
            eprintln!("usage: msqu-tool TOOL [args]   tools: stopwatch, drink, alarm, timer, repeat, countdown, clock, pick");
            2
        }
    };
    std::process::exit(code);
}
