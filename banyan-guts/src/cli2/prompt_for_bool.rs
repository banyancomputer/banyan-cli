use std::io::Read;

/// Prompt the user for a y/n answer
pub fn prompt_for_bool(msg: &str) -> bool {
    info!("{msg} y/n");
    loop {
        let mut input = [0];
        let _ = std::io::stdin().read(&mut input);
        match input[0] as char {
            'y' | 'Y' => return true,
            'n' | 'N' => return false,
            _ => info!("y/n only please."),
        }
    }
}
