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

static TEXT: ModuleManifest = ModuleManifest {
    id: "text",
    name: "Translator",
    nav: Some(NavEntry {
        order: 42,
        icon: "languages",
    }),
    commands: &[],
    context_actions: &[],
    settings_key: None,
    capabilities: &[],
};

pub struct TextModule;
#[async_trait]
impl Module for TextModule {
    fn manifest(&self) -> &ModuleManifest {
        &TEXT
    }
    async fn start(&self) -> Result<()> {
        Ok(())
    }
}

static PHONE: ModuleManifest = ModuleManifest {
    id: "phone",
    name: "Phone transfer",
    nav: Some(NavEntry {
        order: 44,
        icon: "smartphone",
    }),
    commands: &[],
    context_actions: &[],
    settings_key: None,
    capabilities: &[],
};

pub struct PhoneModule;
#[async_trait]
impl Module for PhoneModule {
    fn manifest(&self) -> &ModuleManifest {
        &PHONE
    }
    async fn start(&self) -> Result<()> {
        Ok(())
    }
}

static STARTUP: ModuleManifest = ModuleManifest {
    id: "startup",
    name: "Startup apps",
    nav: Some(NavEntry {
        order: 46,
        icon: "power",
    }),
    commands: &[],
    context_actions: &[],
    settings_key: None,
    capabilities: &[],
};

pub struct StartupModule;
#[async_trait]
impl Module for StartupModule {
    fn manifest(&self) -> &ModuleManifest {
        &STARTUP
    }
    async fn start(&self) -> Result<()> {
        Ok(())
    }
}

static ACTIONS: ModuleManifest = ModuleManifest {
    id: "actions",
    name: "My actions",
    nav: Some(NavEntry {
        order: 47,
        icon: "rocket",
    }),
    commands: &[],
    context_actions: &[],
    settings_key: None,
    capabilities: &[],
};

pub struct ActionsModule;
#[async_trait]
impl Module for ActionsModule {
    fn manifest(&self) -> &ModuleManifest {
        &ACTIONS
    }
    async fn start(&self) -> Result<()> {
        Ok(())
    }
}

static REMINDERS: ModuleManifest = ModuleManifest {
    id: "reminders",
    name: "Reminders",
    nav: Some(NavEntry {
        order: 48,
        icon: "bell",
    }),
    commands: &[],
    context_actions: &[],
    settings_key: None,
    capabilities: &[],
};

pub struct RemindersModule;
#[async_trait]
impl Module for RemindersModule {
    fn manifest(&self) -> &ModuleManifest {
        &REMINDERS
    }
    async fn start(&self) -> Result<()> {
        Ok(())
    }
}

static NOTES: ModuleManifest = ModuleManifest {
    id: "notes",
    name: "Notes",
    nav: Some(NavEntry {
        order: 49,
        icon: "pencil",
    }),
    commands: &[],
    context_actions: &[],
    settings_key: None,
    capabilities: &[],
};

pub struct NotesModule;
#[async_trait]
impl Module for NotesModule {
    fn manifest(&self) -> &ModuleManifest {
        &NOTES
    }
    async fn start(&self) -> Result<()> {
        Ok(())
    }
}

static CLOCKS: ModuleManifest = ModuleManifest {
    id: "clocks",
    name: "Time tools",
    nav: Some(NavEntry {
        order: 50,
        icon: "clock",
    }),
    commands: &[],
    context_actions: &[],
    settings_key: None,
    capabilities: &[],
};

pub struct ClocksModule;
#[async_trait]
impl Module for ClocksModule {
    fn manifest(&self) -> &ModuleManifest {
        &CLOCKS
    }
    async fn start(&self) -> Result<()> {
        Ok(())
    }
}

static TOOLBOX: ModuleManifest = ModuleManifest {
    id: "toolbox",
    name: "Text toolbox",
    nav: Some(NavEntry {
        order: 51,
        icon: "wrench",
    }),
    commands: &[],
    context_actions: &[],
    settings_key: None,
    capabilities: &[],
};

pub struct ToolboxModule;
#[async_trait]
impl Module for ToolboxModule {
    fn manifest(&self) -> &ModuleManifest {
        &TOOLBOX
    }
    async fn start(&self) -> Result<()> {
        Ok(())
    }
}

static MINI: ModuleManifest = ModuleManifest {
    id: "mini",
    name: "Mini tools",
    nav: Some(NavEntry {
        order: 52,
        icon: "wand",
    }),
    commands: &[],
    context_actions: &[],
    settings_key: None,
    capabilities: &[],
};

pub struct MiniModule;
#[async_trait]
impl Module for MiniModule {
    fn manifest(&self) -> &ModuleManifest {
        &MINI
    }
    async fn start(&self) -> Result<()> {
        Ok(())
    }
}

static INBOX: ModuleManifest = ModuleManifest {
    id: "inbox",
    name: "Notifications",
    nav: Some(NavEntry {
        order: 53,
        icon: "bell",
    }),
    commands: &[],
    context_actions: &[],
    settings_key: None,
    capabilities: &[],
};

pub struct InboxModule;
#[async_trait]
impl Module for InboxModule {
    fn manifest(&self) -> &ModuleManifest {
        &INBOX
    }
    async fn start(&self) -> Result<()> {
        Ok(())
    }
}

static RULES: ModuleManifest = ModuleManifest {
    id: "rules",
    name: "Folder rules",
    nav: Some(NavEntry {
        order: 54,
        icon: "wand",
    }),
    commands: &[],
    context_actions: &[],
    settings_key: None,
    capabilities: &[],
};

pub struct RulesModule;
#[async_trait]
impl Module for RulesModule {
    fn manifest(&self) -> &ModuleManifest {
        &RULES
    }
    async fn start(&self) -> Result<()> {
        Ok(())
    }
}

static SNIPPETS: ModuleManifest = ModuleManifest {
    id: "snippets",
    name: "Snippets",
    nav: Some(NavEntry {
        order: 55,
        icon: "file-text",
    }),
    commands: &[],
    context_actions: &[],
    settings_key: None,
    capabilities: &[],
};

pub struct SnippetsModule;
#[async_trait]
impl Module for SnippetsModule {
    fn manifest(&self) -> &ModuleManifest {
        &SNIPPETS
    }
    async fn start(&self) -> Result<()> {
        Ok(())
    }
}

static SHOTS: ModuleManifest = ModuleManifest {
    id: "shots",
    name: "Screenshots",
    nav: Some(NavEntry {
        order: 56,
        icon: "image",
    }),
    commands: &[],
    context_actions: &[],
    settings_key: None,
    capabilities: &[],
};

pub struct ShotsModule;
#[async_trait]
impl Module for ShotsModule {
    fn manifest(&self) -> &ModuleManifest {
        &SHOTS
    }
    async fn start(&self) -> Result<()> {
        Ok(())
    }
}

static GAMEMODE: ModuleManifest = ModuleManifest {
    id: "gamemode",
    name: "Game mode",
    nav: Some(NavEntry {
        order: 57,
        icon: "playtime",
    }),
    commands: &[],
    context_actions: &[],
    settings_key: None,
    capabilities: &[],
};

pub struct GameModeModule;
#[async_trait]
impl Module for GameModeModule {
    fn manifest(&self) -> &ModuleManifest {
        &GAMEMODE
    }
    async fn start(&self) -> Result<()> {
        Ok(())
    }
}

static QR: ModuleManifest = ModuleManifest {
    id: "qr",
    name: "QR codes",
    nav: Some(NavEntry {
        order: 58,
        icon: "layers",
    }),
    commands: &[],
    context_actions: &[],
    settings_key: None,
    capabilities: &[],
};

pub struct QrModule;
#[async_trait]
impl Module for QrModule {
    fn manifest(&self) -> &ModuleManifest {
        &QR
    }
    async fn start(&self) -> Result<()> {
        Ok(())
    }
}

static COMPARE: ModuleManifest = ModuleManifest {
    id: "compare",
    name: "Compare folders",
    nav: Some(NavEntry {
        order: 59,
        icon: "arrow",
    }),
    commands: &[],
    context_actions: &[],
    settings_key: None,
    capabilities: &[],
};

pub struct CompareModule;
#[async_trait]
impl Module for CompareModule {
    fn manifest(&self) -> &ModuleManifest {
        &COMPARE
    }
    async fn start(&self) -> Result<()> {
        Ok(())
    }
}

static ARCHIVES: ModuleManifest = ModuleManifest {
    id: "archives",
    name: "ZIP files",
    nav: Some(NavEntry {
        order: 60,
        icon: "package",
    }),
    commands: &[],
    context_actions: &[],
    settings_key: None,
    capabilities: &[],
};

pub struct ArchivesModule;
#[async_trait]
impl Module for ArchivesModule {
    fn manifest(&self) -> &ModuleManifest {
        &ARCHIVES
    }
    async fn start(&self) -> Result<()> {
        Ok(())
    }
}

static RECORDER: ModuleManifest = ModuleManifest {
    id: "recorder",
    name: "Screen recorder",
    nav: Some(NavEntry {
        order: 61,
        icon: "film",
    }),
    commands: &[],
    context_actions: &[],
    settings_key: None,
    capabilities: &[],
};

pub struct RecorderModule;
#[async_trait]
impl Module for RecorderModule {
    fn manifest(&self) -> &ModuleManifest {
        &RECORDER
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
