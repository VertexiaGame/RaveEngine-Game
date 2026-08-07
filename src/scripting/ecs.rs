use bevy::prelude::*;
use serde::{Serialize, Deserialize};

fn default_true() -> bool {
    true
}

#[derive(Component, Reflect, Clone, Serialize, Deserialize)]
#[reflect(Component)]
pub struct ServerScript {
    pub code: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(skip)]
    pub started: bool,
    #[serde(skip)]
    pub running_code: String,
}

impl Default for ServerScript {
    fn default() -> Self {
        Self {
            code: "".to_string(),
            enabled: true,
            started: false,
            running_code: String::new(),
        }
    }
}

#[derive(Component, Reflect, Clone, Serialize, Deserialize)]
#[reflect(Component)]
pub struct LocalScript {
    pub code: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(skip)]
    pub started: bool,
    #[serde(skip)]
    pub running_code: String,
}

impl Default for LocalScript {
    fn default() -> Self {
        Self {
            code: "".to_string(),
            enabled: true,
            started: false,
            running_code: String::new(),
        }
    }
}

#[derive(Component, Reflect, Default, Clone, Serialize, Deserialize)]
#[reflect(Component)]
pub struct ModuleScript {
    pub code: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_defaults_are_ready_to_run() {
        let s = ServerScript::default();
        assert!(s.code.is_empty());
        assert!(s.enabled);
        assert!(!s.started);
        assert!(s.running_code.is_empty());

        let l = LocalScript::default();
        assert!(l.enabled && !l.started);
        assert!(ModuleScript::default().code.is_empty());
    }

    #[test]
    fn started_and_running_code_are_not_serialized() {
        let script = ServerScript {
            code: "print('hi')".to_string(),
            started: true,
            running_code: "old".to_string(),
            ..default()
        };
        let json = serde_json::to_string(&script).unwrap();
        assert!(json.contains("print"), "{json}");
        assert!(!json.contains("started"), "{json}");
        assert!(!json.contains("running_code"), "{json}");
    }

    #[test]
    fn serialized_scripts_default_to_enabled() {
        let json = r#"{"code":"print('x')"}"#;
        let script: ServerScript = serde_json::from_str(json).unwrap();
        assert!(script.enabled);
        assert_eq!(script.code, "print('x')");
    }
}
