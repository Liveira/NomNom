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

static CLIPBOARD: ModuleManifest = ModuleManifest {
    id: "clipboard",
    name: "Clipboard",
    nav: Some(NavEntry {
        order: 38,
        icon: "clipboard",
    }),
    commands: &[],
    context_actions: &[],
    settings_key: None,
    capabilities: &[],
};

pub struct ClipboardModule;
#[async_trait]
impl Module for ClipboardModule {
    fn manifest(&self) -> &ModuleManifest {
        &CLIPBOARD
    }
    async fn start(&self) -> Result<()> {
        Ok(())
    }
}

static TOOLS: ModuleManifest = ModuleManifest {
    id: "tools",
    name: "Tools",
    nav: Some(NavEntry {
        order: 5,
        icon: "tools",
    }),
    commands: &[],
    context_actions: &[],
    settings_key: None,
    capabilities: &[],
};

pub struct ToolsHub;
#[async_trait]
impl Module for ToolsHub {
    fn manifest(&self) -> &ModuleManifest {
        &TOOLS
    }
    async fn start(&self) -> Result<()> {
        Ok(())
    }
}

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

static DOWNLOADS: ModuleManifest = ModuleManifest {
    id: "downloads",
    name: "NomNom Downloader",
    nav: Some(NavEntry {
        order: 10,
        icon: "cat",
    }),
    commands: &[],
    context_actions: &[],
    settings_key: Some("downloads"),
    capabilities: &[
        nomnom_core::module::Capability::Network,
        nomnom_core::module::Capability::WriteFs,
        nomnom_core::module::Capability::Spawn,
    ],
};

pub struct Downloads;
#[async_trait]
impl Module for Downloads {
    fn manifest(&self) -> &ModuleManifest {
        &DOWNLOADS
    }
    async fn start(&self) -> Result<()> {
        Ok(())
    }
}

static ORGANIZER: ModuleManifest = ModuleManifest {
    id: "organizer",
    name: "File Organizer",
    nav: Some(NavEntry {
        order: 20,
        icon: "organizer",
    }),
    commands: &[],
    context_actions: &[],
    settings_key: Some("organizer"),
    capabilities: &[
        nomnom_core::module::Capability::ReadFs,
        nomnom_core::module::Capability::WriteFs,
        nomnom_core::module::Capability::Watch,
    ],
};

pub struct Organizer;
#[async_trait]
impl Module for Organizer {
    fn manifest(&self) -> &ModuleManifest {
        &ORGANIZER
    }
    async fn start(&self) -> Result<()> {
        Ok(())
    }
}

static FILETOOLS: ModuleManifest = ModuleManifest {
    id: "filetools",
    name: "File Tools",
    nav: Some(NavEntry {
        order: 30,
        icon: "convert",
    }),
    commands: &[],
    context_actions: &[],
    settings_key: None,
    capabilities: &[
        nomnom_core::module::Capability::ReadFs,
        nomnom_core::module::Capability::WriteFs,
        nomnom_core::module::Capability::Spawn,
    ],
};

pub struct FileTools;
#[async_trait]
impl Module for FileTools {
    fn manifest(&self) -> &ModuleManifest {
        &FILETOOLS
    }
    async fn start(&self) -> Result<()> {
        Ok(())
    }
}

static STORAGE: ModuleManifest = ModuleManifest {
    id: "storage",
    name: "Disk Analyzer",
    nav: Some(NavEntry {
        order: 25,
        icon: "storage",
    }),
    commands: &[],
    context_actions: &[],
    settings_key: None,
    capabilities: &[
        nomnom_core::module::Capability::ReadFs,
        nomnom_core::module::Capability::WriteFs,
    ],
};

pub struct Storage;
#[async_trait]
impl Module for Storage {
    fn manifest(&self) -> &ModuleManifest {
        &STORAGE
    }
    async fn start(&self) -> Result<()> {
        Ok(())
    }
}

static ACTIVITY: ModuleManifest = ModuleManifest {
    id: "activity",
    name: "Playtime",
    nav: Some(NavEntry {
        order: 15,
        icon: "playtime",
    }),
    commands: &[],
    context_actions: &[],
    settings_key: Some("activity"),
    capabilities: &[],
};

static BACKUP: ModuleManifest = ModuleManifest {
    id: "backup",
    name: "Backup",
    nav: Some(NavEntry {
        order: 35,
        icon: "backup",
    }),
    commands: &[],
    context_actions: &[],
    settings_key: None,
    capabilities: &[],
};

pub struct Backup;
#[async_trait]
impl Module for Backup {
    fn manifest(&self) -> &ModuleManifest {
        &BACKUP
    }
    async fn start(&self) -> Result<()> {
        Ok(())
    }
}

static MONITOR: ModuleManifest = ModuleManifest {
    id: "monitor",
    name: "System Monitor",
    nav: Some(NavEntry {
        order: 40,
        icon: "activity",
    }),
    commands: &[],
    context_actions: &[],
    settings_key: Some("monitor"),
    capabilities: &[],
};

pub struct Monitor;
#[async_trait]
impl Module for Monitor {
    fn manifest(&self) -> &ModuleManifest {
        &MONITOR
    }
    async fn start(&self) -> Result<()> {
        Ok(())
    }
}

pub struct Activity;
#[async_trait]
impl Module for Activity {
    fn manifest(&self) -> &ModuleManifest {
        &ACTIVITY
    }
    async fn start(&self) -> Result<()> {
        Ok(())
    }
}
