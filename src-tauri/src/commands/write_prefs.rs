use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

#[derive(Debug, Hash, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct ProtectedKey {
    pub device_id: u32,
    pub object_type: String,
    pub instance: u32,
    pub property_id: String,
}

fn rules_file_path() -> PathBuf {
    let mut path = if let Some(data_dir) = dirs::data_dir() {
        data_dir
    } else {
        PathBuf::from(".")
    };
    path.push("baccy");
    let _ = fs::create_dir_all(&path);
    path.push("write_protection.json");
    path
}

fn load_rules() -> HashSet<ProtectedKey> {
    let path = rules_file_path();
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_rules(rules: &HashSet<ProtectedKey>) {
    let path = rules_file_path();
    if let Ok(json) = serde_json::to_string_pretty(rules) {
        let _ = fs::write(&path, json);
    }
}

pub struct WriteProtection {
    protected: Mutex<HashSet<ProtectedKey>>,
}

impl WriteProtection {
    pub fn new() -> Self {
        let rules = load_rules();
        Self {
            protected: Mutex::new(rules),
        }
    }

    pub fn is_protected(&self, key: &ProtectedKey) -> bool {
        let protected = self.protected.lock().unwrap();
        protected.iter().any(|rule| {
            (rule.device_id == 0 || rule.device_id == key.device_id)
                && (rule.object_type.is_empty() || rule.object_type == key.object_type)
                && (rule.instance == 0 || rule.instance == key.instance)
                && rule.property_id == key.property_id
        })
    }

    pub fn add_protection(&self, key: ProtectedKey) {
        let mut protected = self.protected.lock().unwrap();
        protected.insert(key);
        save_rules(&protected);
    }

    pub fn remove_protection(&self, key: ProtectedKey) {
        let mut protected = self.protected.lock().unwrap();
        protected.remove(&key);
        save_rules(&protected);
    }

    pub fn get_all(&self) -> Vec<ProtectedKey> {
        self.protected.lock().unwrap().iter().cloned().collect()
    }
}

#[tauri::command]
pub fn is_write_protected(key: ProtectedKey, state: State<'_, AppState>) -> Result<bool, String> {
    Ok(state.write_protection.is_protected(&key))
}

#[tauri::command]
pub fn set_write_protection(
    key: ProtectedKey,
    protected: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if protected {
        state.write_protection.add_protection(key);
    } else {
        state.write_protection.remove_protection(key);
    }
    Ok(())
}

#[tauri::command]
pub fn get_all_write_protections(state: State<'_, AppState>) -> Result<Vec<ProtectedKey>, String> {
    Ok(state.write_protection.get_all())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        let wp = WriteProtection::new();
        let key = ProtectedKey {
            device_id: 123,
            object_type: "AnalogOutput".to_string(),
            instance: 1,
            property_id: "PresentValue".to_string(),
        };
        wp.add_protection(key.clone());

        assert!(wp.is_protected(&key));
        
        let different_key = ProtectedKey {
            device_id: 123,
            object_type: "AnalogOutput".to_string(),
            instance: 1,
            property_id: "Description".to_string(),
        };
        assert!(!wp.is_protected(&different_key));
    }

    #[test]
    fn test_device_wildcard() {
        let wp = WriteProtection::new();
        wp.add_protection(ProtectedKey {
            device_id: 0,
            object_type: "AnalogOutput".to_string(),
            instance: 1,
            property_id: "PresentValue".to_string(),
        });

        let key = ProtectedKey {
            device_id: 999,
            object_type: "AnalogOutput".to_string(),
            instance: 1,
            property_id: "PresentValue".to_string(),
        };
        assert!(wp.is_protected(&key));
    }

    #[test]
    fn test_instance_wildcard() {
        let wp = WriteProtection::new();
        wp.add_protection(ProtectedKey {
            device_id: 123,
            object_type: "AnalogOutput".to_string(),
            instance: 0,
            property_id: "PresentValue".to_string(),
        });

        let key = ProtectedKey {
            device_id: 123,
            object_type: "AnalogOutput".to_string(),
            instance: 456,
            property_id: "PresentValue".to_string(),
        };
        assert!(wp.is_protected(&key));
    }

    #[test]
    fn test_device_and_instance_wildcard() {
        let wp = WriteProtection::new();
        wp.add_protection(ProtectedKey {
            device_id: 0,
            object_type: "AnalogOutput".to_string(),
            instance: 0,
            property_id: "PresentValue".to_string(),
        });

        let key = ProtectedKey {
            device_id: 555,
            object_type: "AnalogOutput".to_string(),
            instance: 777,
            property_id: "PresentValue".to_string(),
        };
        assert!(wp.is_protected(&key));
    }
}
