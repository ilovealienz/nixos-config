//! The monitors, from sway (swaymsg -t get_outputs).

use std::process::Command;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Output {
    pub name: String,
    pub width: u32,
    pub height: u32,
    /// Hz, rounded
    pub refresh: u32,
    pub focused: bool,
}

impl Output {
    /// "DP-1 · 1920×1080 · 144Hz"
    pub fn label(&self) -> String {
        format!("{} · {}×{} · {}Hz", self.name, self.width, self.height, self.refresh)
    }
}

/// the active monitors, focused first; empty outside sway
pub fn list() -> Vec<Output> {
    Command::new("swaymsg")
        .args(["-r", "-t", "get_outputs"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| parse(&String::from_utf8_lossy(&o.stdout)))
        .unwrap_or_default()
}

pub fn parse(json: &str) -> Vec<Output> {
    let Ok(serde_json::Value::Array(items)) = serde_json::from_str(json) else {
        return Vec::new();
    };
    let mut out: Vec<Output> = items
        .iter()
        .filter(|o| o["active"].as_bool() != Some(false))
        .filter_map(|o| {
            let mode = &o["current_mode"];
            Some(Output {
                name: o["name"].as_str()?.to_string(),
                width: mode["width"].as_u64()? as u32,
                height: mode["height"].as_u64()? as u32,
                // sway gives mHz
                refresh: ((mode["refresh"].as_u64().unwrap_or(0) + 500) / 1000) as u32,
                focused: o["focused"].as_bool().unwrap_or(false),
            })
        })
        .collect();
    out.sort_by_key(|o| !o.focused);
    out
}

/// moves the focus there, so gamescope opens on it
pub fn focus(name: &str) {
    if !name.is_empty() {
        let _ = Command::new("swaymsg")
            .args(["-q", "focus", "output", name])
            .status();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_sway() {
        let j = r#"[
          {"name":"DP-2","active":true,"focused":false,"current_mode":{"width":1920,"height":1080,"refresh":165000}},
          {"name":"DP-1","active":true,"focused":true,"adaptive_sync_status":"disabled","current_mode":{"width":1920,"height":1080,"refresh":143855}},
          {"name":"HDMI-A-3","active":false,"current_mode":{"width":0,"height":0,"refresh":0}}
        ]"#;
        let o = parse(j);
        assert_eq!(o.len(), 2);
        assert_eq!(o[0].label(), "DP-1 · 1920×1080 · 144Hz");
        assert_eq!(o[1].refresh, 165);
        assert!(parse("nope").is_empty());
    }
}
