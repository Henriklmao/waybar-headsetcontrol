use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KeyConfig {
    pub dec1: char,
    pub dec10: char,
    pub inc1: char,
    pub inc10: char,
    pub full: char,
    pub none: char,
    pub quit: char,
    pub config: char,
    pub verbose: char,
}

impl Default for KeyConfig {
    fn default() -> Self {
        Self {
            dec1: 'a',
            dec10: 's',
            inc1: 'd',
            inc10: 'w',
            full: 'f',
            none: 'e',
            quit: 'q',
            config: 'c',
            verbose: 'v',
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct ConfigFile {
    keys: KeyConfig,
    #[serde(default)]
    default_sidetone: u8,
}

pub fn get_config_path() -> PathBuf {
    let mut path = dirs::home_dir().unwrap_or_default();
    path.push(".config/wb-headsetcontrol/config.toml");
    path
}

pub fn load_config() -> KeyConfig {
    let path = get_config_path();
    
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(config_file) = toml::from_str::<ConfigFile>(&content) {
                return config_file.keys;
            }
        }
    }
    
    // Auto-create config with defaults if missing
    let config = KeyConfig::default();
    let _ = save_config_full(&config, 0);
    config
}

pub fn load_default_sidetone() -> u8 {
    let path = get_config_path();
    
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(config_file) = toml::from_str::<ConfigFile>(&content) {
                return config_file.default_sidetone;
            }
        }
    }
    
    0
}

pub fn save_config(keys: &KeyConfig) -> std::io::Result<()> {
    save_config_full(keys, 0)
}

fn save_config_full(keys: &KeyConfig, default_sidetone: u8) -> std::io::Result<()> {
    let path = get_config_path();
    
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    
    let config_file = ConfigFile {
        keys: keys.clone(),
        default_sidetone,
    };
    
    let content = toml::to_string_pretty(&config_file)
        .unwrap_or_else(|_| String::new());
    
    fs::write(&path, content)?;
    Ok(())
}

pub fn interactive_config() -> std::io::Result<()> {
    use std::io::{self, Write};
    
    let mut config = load_config();
    let mut default_sidetone = load_default_sidetone();
    
    println!("\n=== Headset Control Configuration ===\n");
    
    // Configure default sidetone
    loop {
        print!("Default sidetone level (0-128): [{}] ", default_sidetone);
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        let trimmed = input.trim();
        if trimmed.is_empty() {
            break;
        }
        if let Ok(level) = trimmed.parse::<u8>() {
            if level <= 128 {
                default_sidetone = level;
                break;
            } else {
                println!("Please enter a value between 0 and 128");
            }
        } else {
            println!("Invalid input");
        }
    }
    
    println!("\n=== Key Bindings ===\n");
    
    let keys_to_configure = vec![
        ("dec1", "Decrease sidetone by 1", &mut config.dec1),
        ("dec10", "Decrease sidetone by 10", &mut config.dec10),
        ("inc1", "Increase sidetone by 1", &mut config.inc1),
        ("inc10", "Increase sidetone by 10", &mut config.inc10),
        ("full", "Set sidetone to full (128)", &mut config.full),
        ("none", "Set sidetone to none (0)", &mut config.none),
        ("config", "Open configuration menu", &mut config.config),
        ("verbose", "Toggle verbose mode", &mut config.verbose),
        ("quit", "Quit the application", &mut config.quit),
    ];
    
    for (key_name, description, key_ref) in keys_to_configure {
        print!("{} ({}): [{}] ", description, key_name, key_ref);
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        let trimmed = input.trim();
        if !trimmed.is_empty() {
            if let Some(c) = trimmed.chars().next() {
                *key_ref = c.to_lowercase().next().unwrap_or(c);
            }
        }
    }
    
    // Ask to save
    println!("\nSave changes? (w - write, esc - discard)");
    loop {
        print!("> ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let trimmed = input.trim();
        
        if trimmed.to_lowercase() == "w" {
            save_config_full(&config, default_sidetone)?;
            println!("Configuration saved to {:?}", get_config_path());
            return Ok(());
        } else if trimmed.is_empty() || trimmed.to_lowercase() == "esc" {
            println!("Changes discarded");
            return Ok(());
        }
    }
}
