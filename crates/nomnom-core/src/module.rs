//! Module registration and lifecycle. A feature registers a `Module`; the shell derives
//! navigation, commands, settings and context-menu entries from its manifest.

use crate::{Error, Result};
use async_trait::async_trait;
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::OnceCell;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct NavEntry {
    pub order: u16,
    pub icon: &'static str,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct CommandInfo {
    pub id: &'static str,
    pub title: &'static str,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct ContextAction {
    pub id: &'static str,
    pub title: &'static str,
    pub extensions: &'static [&'static str],
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    Network,
    ReadFs,
    WriteFs,
    Spawn,
    Watch,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModuleManifest {
    pub id: &'static str,
    pub name: &'static str,
    pub nav: Option<NavEntry>,
    pub commands: &'static [CommandInfo],
    pub context_actions: &'static [ContextAction],
    pub settings_key: Option<&'static str>,
    pub capabilities: &'static [Capability],
}

#[async_trait]
pub trait Module: Send + Sync {
    fn manifest(&self) -> &ModuleManifest;
    /// Called lazily the first time the module is needed, never at boot.
    async fn start(&self) -> Result<()>;
    async fn stop(&self) -> Result<()> {
        Ok(())
    }
}

struct Entry {
    module: Arc<dyn Module>,
    started: OnceCell<()>,
}

#[derive(Default)]
pub struct ModuleRegistry {
    modules: BTreeMap<&'static str, Entry>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, module: Arc<dyn Module>) {
        let id = module.manifest().id;
        self.modules.insert(id, Entry { module, started: OnceCell::new() });
    }

    /// Manifests ordered by navigation order (modules without nav come last).
    pub fn manifests(&self) -> Vec<ModuleManifest> {
        let mut list: Vec<_> = self.modules.values().map(|e| e.module.manifest().clone()).collect();
        list.sort_by_key(|m| m.nav.map_or(u16::MAX, |n| n.order));
        list
    }

    /// Starts the module on first use. Concurrent callers wait for the same start.
    pub async fn ensure_started(&self, id: &str) -> Result<()> {
        let entry = self.modules.get(id).ok_or_else(|| Error::UnknownModule(id.to_string()))?;
        entry.started.get_or_try_init(|| entry.module.start()).await?;
        Ok(())
    }

    pub async fn stop_all(&self) {
        for entry in self.modules.values() {
            if entry.started.initialized() {
                if let Err(e) = entry.module.stop().await {
                    tracing::warn!("module stop failed: {e}");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static MANIFEST_B: ModuleManifest = ModuleManifest {
        id: "b",
        name: "B",
        nav: Some(NavEntry { order: 20, icon: "b" }),
        commands: &[],
        context_actions: &[],
        settings_key: None,
        capabilities: &[],
    };
    static MANIFEST_A: ModuleManifest = ModuleManifest {
        id: "a",
        name: "A",
        nav: Some(NavEntry { order: 10, icon: "a" }),
        commands: &[],
        context_actions: &[],
        settings_key: None,
        capabilities: &[],
    };

    struct Fake(&'static ModuleManifest, Arc<AtomicUsize>);
    #[async_trait]
    impl Module for Fake {
        fn manifest(&self) -> &ModuleManifest {
            self.0
        }
        async fn start(&self) -> Result<()> {
            self.1.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test]
    async fn orders_by_nav_and_starts_lazily_once() {
        let starts = Arc::new(AtomicUsize::new(0));
        let mut reg = ModuleRegistry::new();
        reg.register(Arc::new(Fake(&MANIFEST_B, starts.clone())));
        reg.register(Arc::new(Fake(&MANIFEST_A, starts.clone())));

        let ids: Vec<_> = reg.manifests().iter().map(|m| m.id).collect();
        assert_eq!(ids, ["a", "b"]);
        assert_eq!(starts.load(Ordering::SeqCst), 0, "nothing starts at registration");

        let (r1, r2) = tokio::join!(reg.ensure_started("a"), reg.ensure_started("a"));
        r1.unwrap();
        r2.unwrap();
        assert_eq!(starts.load(Ordering::SeqCst), 1);
        assert!(matches!(reg.ensure_started("zzz").await, Err(Error::UnknownModule(_))));
    }
}
