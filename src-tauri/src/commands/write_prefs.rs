use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Mutex;
use tauri::State;

#[derive(Debug, Hash, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct ProtectedKey {
    pub device_id: u32,
    pub object_type: String,
    pub instance: u32,
    pub property_id: String,
}

pub struct WriteProtection {
    protected: Mutex<HashSet<ProtectedKey>>,
}

impl WriteProtection {
    pub fn new() -> Self {
        Self {
            protected: Mutex::new(HashSet::new()),
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
        self.protected.lock().unwrap().insert(key);
    }

    pub fn remove_protection(&self, key: ProtectedKey) {
        self.protected.lock().unwrap().remove(&key);
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
