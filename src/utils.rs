use std::process::Command;

pub fn get_suitable_terminal() -> Option<String> {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .to_uppercase();

    if desktop.contains("GNOME") {
        if command_exists("ptyxis") {
            return Some("ptyxis -x ".to_string());
        } else if command_exists("gnome-terminal") {
            return Some("gnome-terminal -- ".to_string());
        }
    } else if desktop.contains("KDE") || desktop.contains("PLASMA") {
        if command_exists("konsole") {
            return Some("konsole -e ".to_string());
        }
    } else if desktop.contains("XFCE") {
        if command_exists("xfce4-terminal") {
            return Some("xfce4-terminal -e ".to_string());
        }
    }

    for term in &["alacritty", "kitty", "xterm"] {
        if command_exists(term) {
            return Some(format!("{term} -e "));
        }
    }

    None
}

pub fn open_terminal_with_command(command: &str) {
    let terminal_cmd = match get_suitable_terminal() {
        Some(t) => t,
        None => {
            eprintln!("Error: No suitable terminal emulator found.");
            return;
        }
    };

    let escaped = command.replace('\'', "'\"'\"'");
    let full_cmd = format!(
        "{terminal_cmd} bash -c '{escaped}; echo; echo Press ENTER to close...; read'"
    );

    if let Err(e) = Command::new("sh").arg("-c").arg(&full_cmd).spawn() {
        eprintln!("Error opening terminal: {e}");
    }
}

fn command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
