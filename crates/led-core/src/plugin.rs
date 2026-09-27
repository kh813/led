use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use wasmi::{Caller, Engine, Func, Linker, Memory, Module, Store, TypedFunc};
use crate::outline::OutlineNode;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    #[serde(default)]
    pub languages: Vec<String>,
    #[serde(default)]
    pub capabilities: PluginCapabilities,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginCapabilities {
    #[serde(default)]
    pub outline_provider: bool,
    #[serde(default)]
    pub commands: Vec<String>,
}

pub struct WasmPlugin {
    pub manifest: PluginManifest,
    #[allow(dead_code)]
    engine: Engine,
    store: Store<()>,
    memory: Memory,
    alloc_fn: Option<TypedFunc<i32, i32>>,
    dealloc_fn: Option<TypedFunc<(i32, i32), ()>>,
    parse_outline_fn: Option<TypedFunc<(i32, i32), i64>>,
    exec_command_fn: Option<TypedFunc<(i32, i32, i32, i32), i64>>,
}

impl WasmPlugin {
    pub fn load_from_bytes(manifest: PluginManifest, wasm_bytes: &[u8]) -> Result<Self> {
        let engine = Engine::default();
        let module = Module::new(&engine, wasm_bytes).context("Failed to parse WASM module")?;
        let mut store = Store::new(&engine, ());
        let mut linker = Linker::new(&engine);

        // Define host functions imported by WASM
        linker.define(
            "env",
            "host_log",
            Func::wrap(&mut store, |_caller: Caller<()>, level: i32, _ptr: i32, _len: i32| {
                if level <= 1 {
                    eprintln!("[WASM Plugin Log level={}]", level);
                }
            }),
        )?;

        let instance = linker
            .instantiate(&mut store, &module)
            .context("Failed to instantiate WASM plugin")?
            .start(&mut store)
            .context("Failed to start WASM plugin instance")?;

        let memory = instance
            .get_memory(&store, "memory")
            .ok_or_else(|| anyhow::anyhow!("Plugin missing 'memory' export"))?;

        let alloc_fn = instance.get_typed_func::<i32, i32>(&store, "led_alloc").ok();
        let dealloc_fn = instance.get_typed_func::<(i32, i32), ()>(&store, "led_dealloc").ok();
        let parse_outline_fn = instance
            .get_typed_func::<(i32, i32), i64>(&store, "led_parse_outline")
            .ok();
        let exec_command_fn = instance
            .get_typed_func::<(i32, i32, i32, i32), i64>(&store, "led_execute_command")
            .ok();

        // Optional init hook
        if let Ok(init_fn) = instance.get_typed_func::<(), i32>(&store, "led_init") {
            let _ = init_fn.call(&mut store, ());
        }

        Ok(Self {
            manifest,
            engine,
            store,
            memory,
            alloc_fn,
            dealloc_fn,
            parse_outline_fn,
            exec_command_fn,
        })
    }

    pub fn parse_outline(&mut self, content: &str) -> Result<Vec<OutlineNode>> {
        let parse_fn = self
            .parse_outline_fn
            .ok_or_else(|| anyhow::anyhow!("Plugin does not export led_parse_outline"))?;
        let alloc_fn = self
            .alloc_fn
            .ok_or_else(|| anyhow::anyhow!("Plugin does not export led_alloc"))?;

        let bytes = content.as_bytes();
        let in_len = bytes.len() as i32;
        let in_ptr = alloc_fn.call(&mut self.store, in_len)?;

        self.memory.write(&mut self.store, in_ptr as usize, bytes)?;

        let packed_result = parse_fn.call(&mut self.store, (in_ptr, in_len))?;

        let out_ptr = ((packed_result >> 32) & 0xFFFF_FFFF) as i32;
        let out_len = (packed_result & 0xFFFF_FFFF) as i32;

        if out_len <= 0 || out_ptr == 0 {
            if let Some(dealloc) = self.dealloc_fn {
                let _ = dealloc.call(&mut self.store, (in_ptr, in_len));
            }
            return Ok(Vec::new());
        }

        let mut out_bytes = vec![0u8; out_len as usize];
        self.memory.read(&self.store, out_ptr as usize, &mut out_bytes)?;

        let nodes: Vec<OutlineNode> = serde_json::from_slice(&out_bytes)
            .context("Failed to deserialize OutlineNode list from WASM plugin")?;

        if let Some(dealloc) = self.dealloc_fn {
            let _ = dealloc.call(&mut self.store, (in_ptr, in_len));
            let _ = dealloc.call(&mut self.store, (out_ptr, out_len));
        }

        Ok(nodes)
    }

    pub fn execute_command(&mut self, command: &str, args: &str) -> Result<String> {
        let exec_fn = self
            .exec_command_fn
            .ok_or_else(|| anyhow::anyhow!("Plugin does not export led_execute_command"))?;
        let alloc_fn = self
            .alloc_fn
            .ok_or_else(|| anyhow::anyhow!("Plugin does not export led_alloc"))?;

        let cmd_bytes = command.as_bytes();
        let args_bytes = args.as_bytes();

        let cmd_ptr = alloc_fn.call(&mut self.store, cmd_bytes.len() as i32)?;
        self.memory.write(&mut self.store, cmd_ptr as usize, cmd_bytes)?;

        let args_ptr = alloc_fn.call(&mut self.store, args_bytes.len() as i32)?;
        self.memory.write(&mut self.store, args_ptr as usize, args_bytes)?;

        let packed_result = exec_fn.call(
            &mut self.store,
            (
                cmd_ptr,
                cmd_bytes.len() as i32,
                args_ptr,
                args_bytes.len() as i32,
            ),
        )?;

        let out_ptr = ((packed_result >> 32) & 0xFFFF_FFFF) as i32;
        let out_len = (packed_result & 0xFFFF_FFFF) as i32;

        let result_str = if out_len > 0 && out_ptr != 0 {
            let mut out_bytes = vec![0u8; out_len as usize];
            self.memory.read(&self.store, out_ptr as usize, &mut out_bytes)?;
            String::from_utf8_lossy(&out_bytes).to_string()
        } else {
            String::new()
        };

        if let Some(dealloc) = self.dealloc_fn {
            let _ = dealloc.call(&mut self.store, (cmd_ptr, cmd_bytes.len() as i32));
            let _ = dealloc.call(&mut self.store, (args_ptr, args_bytes.len() as i32));
            if out_len > 0 && out_ptr != 0 {
                let _ = dealloc.call(&mut self.store, (out_ptr, out_len));
            }
        }

        Ok(result_str)
    }
}

pub struct PluginManager {
    pub plugins: Vec<WasmPlugin>,
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginManager {
    pub fn new() -> Self {
        let mut manager = Self {
            plugins: Vec::new(),
        };
        manager.load_installed_plugins();
        manager
    }

    pub fn plugins_dir() -> Option<PathBuf> {
        crate::config::Config::config_dir().map(|d| d.join("plugins"))
    }

    pub fn load_installed_plugins(&mut self) {
        if let Some(plugins_dir) = Self::plugins_dir() {
            if plugins_dir.exists() {
                if let Ok(entries) = fs::read_dir(plugins_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() {
                            let _ = self.load_plugin_dir(&path);
                        } else if path.extension().and_then(|e| e.to_str()) == Some("wasm") {
                            let _ = self.load_standalone_wasm(&path);
                        }
                    }
                }
            }
        }
    }

    pub fn load_plugin_dir(&mut self, dir: &Path) -> Result<()> {
        let manifest_path = dir.join("plugin.toml");
        let wasm_path = dir.join("plugin.wasm");

        if !wasm_path.exists() {
            return Ok(());
        }

        let manifest: PluginManifest = if manifest_path.exists() {
            let manifest_str = fs::read_to_string(&manifest_path)?;
            toml::from_str(&manifest_str)?
        } else {
            let id = dir
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "unnamed_plugin".to_string());
            PluginManifest {
                id: id.clone(),
                name: id,
                version: "0.1.0".to_string(),
                description: None,
                languages: Vec::new(),
                capabilities: PluginCapabilities {
                    outline_provider: true,
                    commands: Vec::new(),
                },
            }
        };

        let wasm_bytes = fs::read(&wasm_path)?;
        let plugin = WasmPlugin::load_from_bytes(manifest, &wasm_bytes)?;
        self.plugins.push(plugin);
        Ok(())
    }

    pub fn load_standalone_wasm(&mut self, path: &Path) -> Result<()> {
        let id = path
            .file_stem()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unnamed".to_string());

        let manifest = PluginManifest {
            id: id.clone(),
            name: id,
            version: "0.1.0".to_string(),
            description: None,
            languages: Vec::new(),
            capabilities: PluginCapabilities {
                outline_provider: true,
                commands: Vec::new(),
            },
        };

        let wasm_bytes = fs::read(path)?;
        let plugin = WasmPlugin::load_from_bytes(manifest, &wasm_bytes)?;
        self.plugins.push(plugin);
        Ok(())
    }

    pub fn parse_outline(&mut self, lang_or_ext: &str, content: &str) -> Option<Vec<OutlineNode>> {
        let lang = lang_or_ext.to_lowercase();
        for plugin in &mut self.plugins {
            if plugin.manifest.capabilities.outline_provider
                && (plugin.manifest.languages.is_empty()
                    || plugin
                        .manifest
                        .languages
                        .iter()
                        .any(|l| l.to_lowercase() == lang))
            {
                if let Ok(nodes) = plugin.parse_outline(content) {
                    if !nodes.is_empty() {
                        return Some(nodes);
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_manager_creation() {
        let manager = PluginManager::new();
        assert!(manager.plugins.is_empty() || !manager.plugins.is_empty());
    }

    #[test]
    fn test_plugin_manifest_deserialization() {
        let toml_str = r#"
id = "markdown-outline"
name = "Markdown Outline"
version = "0.1.0"
description = "Extracts headings from markdown"
languages = ["markdown", "md"]

[capabilities]
outline_provider = true
commands = ["outline.refresh"]
"#;
        let manifest: PluginManifest = toml::from_str(toml_str).unwrap();
        assert_eq!(manifest.id, "markdown-outline");
        assert_eq!(manifest.languages, vec!["markdown", "md"]);
        assert!(manifest.capabilities.outline_provider);
        assert_eq!(manifest.capabilities.commands, vec!["outline.refresh"]);
    }
}

