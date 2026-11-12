#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Command {
    Teleport(f32, f32),
    Speed(f32),
    Fov(f32),
    OpenAll,
    CloseAll,
    Win,
    Restart,
    Help,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SettingChange {
    Speed(f32),
    Fov(f32),
}

pub fn parse_command(input: &str) -> Result<Command, String> {
    let mut words = input.split_whitespace();
    let name = words.next().ok_or_else(|| "enter a command".to_owned())?;
    let args: Vec<_> = words.collect();
    let expected = |usage: &str| Err(format!("usage: {usage}"));
    match name {
        "tp" => {
            if args.len() != 2 {
                return expected("tp <x> <y>");
            }
            let x = args[0]
                .parse::<f32>()
                .map_err(|_| "tp coordinates must be numbers".to_owned())?;
            let y = args[1]
                .parse::<f32>()
                .map_err(|_| "tp coordinates must be numbers".to_owned())?;
            if !x.is_finite() || !y.is_finite() {
                return Err("tp coordinates must be finite numbers".to_owned());
            }
            Ok(Command::Teleport(x, y))
        }
        "speed" => {
            if args.len() != 1 {
                return expected("speed <n>");
            }
            let value = args[0]
                .parse::<f32>()
                .map_err(|_| "speed must be a number".to_owned())?;
            if !value.is_finite() || !(0.1..=10.0).contains(&value) {
                return Err("speed must be between 0.1 and 10".to_owned());
            }
            Ok(Command::Speed(value))
        }
        "fov" => {
            if args.len() != 1 {
                return expected("fov <degrees>");
            }
            let value = args[0]
                .parse::<f32>()
                .map_err(|_| "FOV must be a number".to_owned())?;
            if !value.is_finite() || !(30.0..=120.0).contains(&value) {
                return Err("FOV must be between 30 and 120 degrees".to_owned());
            }
            Ok(Command::Fov(value))
        }
        "open_all" if args.is_empty() => Ok(Command::OpenAll),
        "close_all" if args.is_empty() => Ok(Command::CloseAll),
        "win" if args.is_empty() => Ok(Command::Win),
        "restart" if args.is_empty() => Ok(Command::Restart),
        "help" if args.is_empty() => Ok(Command::Help),
        "open_all" | "close_all" | "win" | "restart" | "help" => {
            Err(format!("{name} takes no arguments"))
        }
        _ => Err(format!("unknown command: {name}")),
    }
}

#[cfg(test)]
mod tests {
    use super::{Command, parse_command};

    #[test]
    fn parses_every_command_kind() {
        assert_eq!(parse_command("tp 4 7"), Ok(Command::Teleport(4.0, 7.0)));
        assert_eq!(parse_command("speed 2.5"), Ok(Command::Speed(2.5)));
        assert_eq!(parse_command("fov 90"), Ok(Command::Fov(90.0)));
        assert_eq!(parse_command("open_all"), Ok(Command::OpenAll));
        assert_eq!(parse_command("close_all"), Ok(Command::CloseAll));
        assert_eq!(parse_command("win"), Ok(Command::Win));
        assert_eq!(parse_command("restart"), Ok(Command::Restart));
        assert_eq!(parse_command("help"), Ok(Command::Help));
    }

    #[test]
    fn rejects_missing_non_numeric_and_unknown_commands() {
        assert!(parse_command("tp 2").unwrap_err().contains("usage"));
        assert!(parse_command("speed fast").unwrap_err().contains("number"));
        assert!(parse_command("fov ninety").unwrap_err().contains("number"));
        assert!(parse_command("tp 1 2 3").unwrap_err().contains("usage"));
        assert!(parse_command("warp 2 3").unwrap_err().contains("unknown"));
        assert!(
            parse_command("win now")
                .unwrap_err()
                .contains("no arguments")
        );
        assert!(parse_command("speed 0").unwrap_err().contains("between"));
    }
}
