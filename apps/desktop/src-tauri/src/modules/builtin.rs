use async_trait::async_trait;
use nomnom_core::module::{Module, ModuleManifest, NavEntry};
use nomnom_core::Result;

static HOME: ModuleManifest = ModuleManifest {
    id: "home",
    name: "Home",
    nav: Some(NavEntry {
        order: 0,
        icon: "home",
    }),
    commands: &[],
    context_actions: &[],
    settings_key: None,
    capabilities: &[],
};

static SETTINGS: ModuleManifest = ModuleManifest {
    id: "settings",
    name: "Settings",
    nav: Some(NavEntry {
        order: 1000,
        icon: "settings",
    }),
    commands: &[],
    context_actions: &[],
    settings_key: None,
    capabilities: &[],
};

pub struct Home;
#[async_trait]
impl Module for Home {
    fn manifest(&self) -> &ModuleManifest {
        &HOME
    }
    async fn start(&self) -> Result<()> {
        Ok(())
    }
}

pub struct Settings;
#[async_trait]
impl Module for Settings {
    fn manifest(&self) -> &ModuleManifest {
        &SETTINGS
    }
    async fn start(&self) -> Result<()> {
        Ok(())
    }
}
