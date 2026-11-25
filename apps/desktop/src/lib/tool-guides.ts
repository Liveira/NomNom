/** Complete, plain-English how-to guides for every NomNom tool and core workflow. */
import type { ToolCategoryId } from './tools';

export interface GuideEntry {
  id: string;
  title: string;
  icon: string;
  category: 'core' | ToolCategoryId;
  summary: string;
  steps: string[];
  tips: string[];
  shortcut?: string;
  requires?: string;
  /** Module route id to open directly from the guide, if applicable. */
  route?: string;
}

export const CORE_GUIDES: GuideEntry[] = [
  {
    id: 'guide-launcher',
    title: 'Global Search & Launcher',
    icon: 'search',
    category: 'core',
    summary: 'Open apps, files, NomNom tools, snippets, unit conversions and quick math from anywhere on your computer.',
    shortcut: 'Ctrl+K',
    steps: [
      'Press Ctrl+K anywhere (even outside NomNom or while it is minimized to the tray) to open the floating search bar.',
      'Type an app name, folder, recent file, NomNom tool, math expression (like 145*12), unit conversion (like 12 kg to lb), or a reminder (like "in 20m stretch").',
      'Press Enter to open or run the highlighted result, Ctrl+Enter to reveal a file in its folder, or Esc to close.',
    ],
    tips: [
      'Inside NomNom, Ctrl+K toggles the built-in command palette right away.',
      'You can switch themes, open clipboard history or start screen OCR directly from Ctrl+K.',
      'Change or disable the global shortcut anytime in Settings → Shortcuts.',
    ],
    route: 'settings',
  },
  {
    id: 'guide-themes',
    title: 'Themes & Full-App Layouts',
    icon: 'palette',
    category: 'core',
    summary: 'Switching themes changes both the colour palette and the layout geometry of the whole app.',
    steps: [
      'Open Settings → Appearance to browse the 17 built-in themes plus System automatic mode.',
      'Each theme applies one of 6 layouts: Classic Studio, Floating Island, Top Bar · Glass, Icon Rail · Bento, Right HUD · Sharp, or Terminal TUI.',
      'Choose Motion (Full, Calm or Off) to tune background effects and animations.',
    ],
    tips: [
      'In Top Bar · Glass (Aurora, Ocean, Frost), scroll your mouse wheel over the top nav bar to reach overflowing tools.',
      'Quickly test themes from anywhere by pressing Ctrl+K and typing "Theme:".',
    ],
    route: 'settings',
  },
  {
    id: 'guide-sets',
    title: 'Tool Sets & Custom Sidebar',
    icon: 'tools',
    category: 'core',
    summary: 'Keep the sidebar focused by choosing a preset Tool Set or toggling individual categories.',
    steps: [
      'Open Tools from the sidebar.',
      'Click any Tool Set (Essentials, Creator & Media, Developer & Power, Gamer & Focus, Work & Study, Cleanup & Storage, Phone & Wireless, or All tools) to swap your sidebar in one click.',
      'Fine-tune by toggling single tools or clicking Show group / Hide group on any category.',
    ],
    tips: [
      'When 6 or more tools are pinned, the vertical sidebar automatically organizes them under category labels.',
      'Hidden tools are never disabled — you can always open them from Tools or via Ctrl+K.',
    ],
    route: 'tools',
  },
  {
    id: 'guide-updates',
    title: 'In-App Updates & Tray',
    icon: 'update',
    category: 'core',
    summary: 'Download verified releases directly inside NomNom without hunting for installers.',
    steps: [
      'NomNom checks GitHub Releases automatically and shows an Update card in the sidebar when a new version is published.',
      'Open Settings → Updates to check manually, read what changed and click Update now.',
      'NomNom verifies the SHA-256 checksum automatically and installs the new build.',
    ],
    tips: [
      'Close-to-tray keeps Playtime, Reminders, Folder rules, Clipboard and Ctrl+K running quietly in the background.',
    ],
    route: 'settings',
  },
];

export const TOOL_GUIDES: Record<string, Omit<GuideEntry, 'id' | 'title' | 'icon' | 'category' | 'route'>> = {
  downloads: {
    summary: 'Download videos, playlists and audio tracks locally with format selection, queues and schedules.',
    shortcut: 'Ctrl+Alt+D',
    requires: 'yt-dlp & FFmpeg (1-click install inside the app)',
    steps: [
      'Paste any video or audio link (or press Ctrl+Alt+D anywhere with a link copied to open Quick Download).',
      'Wait a moment for NomNom to list available resolutions and audio formats, or keep your saved defaults.',
      'Click Download to add it to the queue; track speed, pause/resume or open the finished folder directly.',
    ],
    tips: [
      'Use Subscriptions and Scheduled tabs inside Downloader to check favourite channels or run downloads later.',
      'Connect the NomNom browser extension in Settings → Browser to send links with one click.',
    ],
  },
  activity: {
    summary: 'Tracks active foreground hours for your daily apps, creative software and games on Windows and Linux.',
    steps: [
      'Open Playtime to see today, weekly, monthly and yearly active hours for every app you actively use.',
      'Click any app card to inspect daily charts, streaks, focus history and category assignment.',
      'Set daily or weekly time goals and start Pomodoro Focus sessions from the Focus card.',
    ],
    tips: [
      'Only foreground active time is counted — idle and background apps are paused automatically.',
      'Adjust the idle timeout in Settings → Playtime.',
    ],
  },
  mobile: {
    summary: 'Complete Android-to-PC hub over USB or Wi-Fi: screen mirror, 4K/120fps Webcam Studio, Virtual Display, UHID gaming, files, notifications and more.',
    requires: 'Android phone with USB or Wireless debugging + 1-click ADB/scrcpy install',
    steps: [
      'Click Install Android tools once if prompted (downloads official ADB and scrcpy automatically).',
      'Connect by USB cable or click Connect Wi-Fi (QR) and scan the QR code in Settings → Developer options → Wireless debugging on Android 11+ (or pair once with the 6-digit code).',
      'Switch across Home, Webcam, Display, Messages, Calls, Photos, Files, Apps, System, Console and Deck to control every part of your phone.',
    ],
    tips: [
      'Already on USB? Click Go Wireless in the top bar to switch to Wi-Fi and unplug the cable.',
      'In Webcam Studio, use the Wi-Fi 1080p60 preset + smoothing buffer for smooth wireless calls, or OBS Virtual Camera / Linux v4l2loopback for Discord and Zoom.',
      'In Display, launch an independent Virtual Display or enable UHID Keyboard/Mouse/Gamepad for low-latency PC gaming.',
    ],
  },
  storage: {
    summary: 'Fast disk analyzer with visual treemap, sunburst chart, duplicate finder, history snapshots and safe cleanup.',
    steps: [
      'Pick a drive or folder and click Scan to map every subfolder and file size.',
      'Click rectangles in the treemap or rows in the table to drill down; right-click any item for reveal, batch rename or safe delete.',
      'Open the Duplicates, Cleanup spots and History tabs to reclaim space and compare disk growth over time.',
    ],
    tips: [
      'Protected system folders are guarded automatically so critical OS files cannot be deleted by mistake.',
      'Set up scheduled snapshots in Settings to track what fills your drive over weeks.',
    ],
  },
  filetools: {
    summary: 'Convert images, audio, video and Office documents locally, plus PDF tools and batch renaming.',
    requires: 'FFmpeg for audio/video (1-click install); LibreOffice for Office → PDF',
    steps: [
      'Drop files or click Add files, then pick a target format or preset (for example WebP, MP3, MP4 or PDF).',
      'Adjust quality, resolution or trimming options if needed and click Convert.',
      'Use the PDF tools tab to merge, split or compress PDFs, and Batch rename to rename many files with preview and undo.',
    ],
    tips: [
      'Every conversion runs offline on your computer — nothing is uploaded anywhere.',
      'Right-click files in Explorer / Dolphin / Nautilus when the shell menu is enabled to jump straight here.',
    ],
  },
  organizer: {
    summary: 'Sorts messy folders like Downloads into clean subfolders by file type with preview and full one-click undo.',
    steps: [
      'Choose a source folder (for example Downloads) and click Preview.',
      'Review every planned move — code repositories, installers and app folders are skipped automatically.',
      'Click Organize to apply, or click Undo on any past batch in History to move every file right back.',
    ],
    tips: [
      'Enable Auto-organize in Settings → Organizer if you want finished downloads sorted automatically.',
    ],
  },
  backup: {
    summary: 'Scheduled local folder backups with version history, file browser, restore and integrity checks.',
    steps: [
      'Click New plan, pick the source folder to protect and a destination folder (such as an external drive).',
      'Choose a schedule and how many versions to keep, then click Back up now.',
      'Browse any saved version to restore a single file or whole folder, or run Verify to check every stored file hash.',
    ],
    tips: [
      'Unchanged files are reused across versions so incremental backups stay fast and compact.',
    ],
  },
  clipboard: {
    summary: 'Searchable local clipboard history with pinned entries and instant paste from anywhere.',
    shortcut: 'Ctrl+Alt+V',
    steps: [
      'Switch Clipboard history on once to start remembering text and links you copy.',
      'Press Ctrl+Alt+V in any app to open the popup, filter by typing and press Enter to paste.',
      'Pin frequent items (emails, addresses, code snippets) so they never expire.',
    ],
    tips: [
      'Everything stays locally in SQLite on your computer and cleans up older items after your retention window.',
    ],
  },
  text: {
    summary: 'Translate selected text in any app in place, read it in a floating card, or extract text from screen images (OCR).',
    shortcut: 'Ctrl+Alt+R / Ctrl+Alt+Y / Ctrl+Alt+O',
    steps: [
      'Enable Translator and add your DeepL API key (free tier works great) and preferred target language.',
      'Highlight text in any program and press Ctrl+Alt+Y for a floating translation popup, or Ctrl+Alt+R to replace it in place.',
      'Enable Screen OCR and press Ctrl+Alt+O to drag a box over any image or video frame to copy its text.',
    ],
    tips: [
      'On Linux, OCR uses tesseract and automatically falls back to English if a language pack is not installed.',
    ],
  },
  phone: {
    summary: 'Zero-setup Wi-Fi file transfer between any phone (Android or iPhone) and your PC using a QR code.',
    steps: [
      'Click Start sharing to start a temporary local-network server and display a QR code.',
      'Scan the QR code with your phone camera (both devices must be on the same Wi-Fi).',
      'Upload photos/files from your phone browser to this PC, or click Add files here to download them on your phone.',
    ],
    tips: [
      'Nothing leaves your local Wi-Fi and sharing shuts off automatically after the idle timer expires.',
    ],
  },
  startup: {
    summary: 'Shows programs that launch when your computer boots and lets you turn off the ones you do not need.',
    steps: [
      'Open Startup apps to inspect user startup entries and their status.',
      'Toggle off any entry you do not want launching at login.',
      'Turn an entry back on at any time — NomNom remembers disabled items safely.',
    ],
    tips: [
      'System-critical entries are kept read-only so you never break your boot environment.',
    ],
  },
  actions: {
    summary: 'Create your own quick shortcuts for programs, folders, URLs and shell commands.',
    shortcut: 'Ctrl+K',
    steps: [
      'Click New action and choose Program, Folder, Link or Command.',
      'Optionally add placeholders like {query} for prompts, pick a global shortcut and tick Show in Ctrl+K.',
      'Trigger the action anytime from Ctrl+K, your custom hotkey, or the My actions page.',
    ],
    tips: [
      'Great for opening project workspaces, running scripts or searching sites with a single keystroke.',
    ],
  },
  reminders: {
    summary: 'Natural-language reminders that pop up on time even when the main window is closed.',
    steps: [
      'Type naturally, such as "in 20 minutes stretch", "tomorrow at 9 call dentist" or "friday 18:00 submit report".',
      'Check the parsed date/time preview and press Enter to save.',
      'Use Snooze or Done on active reminders, or add reminders directly from Ctrl+K.',
    ],
    tips: [
      'Your next upcoming reminder also shows right on the Home page At a glance strip.',
    ],
  },
  notes: {
    summary: 'Fast scratchpad notes saved locally with pinning and search.',
    shortcut: 'Ctrl+Alt+M',
    steps: [
      'Click New note (or press Ctrl+Alt+M anywhere when Notes is in the sidebar) to capture a thought.',
      'Pin important notes to keep them at the top and filter quickly with the search box.',
      'You can also type "note: buy milk" in Ctrl+K to save a quick note without opening the window.',
    ],
    tips: [
      'Changes save automatically as you type.',
    ],
  },
  clocks: {
    summary: 'World clocks, time-zone converter, stopwatch and background countdown timers.',
    steps: [
      'Add cities in World clocks to track teammates or friends across time zones.',
      'Use the Converter tab to check meeting times across zones at a glance.',
      'Start a Timer or Stopwatch — timers keep counting and ring even if you close the main window to the tray.',
    ],
    tips: [
      'Drag the converter slider to preview any hour of the day across all added cities.',
    ],
  },
  toolbox: {
    summary: 'Offline developer & text utilities: JSON formatter, Base64, URL encode/decode, hashes, UUID, regex tester, timestamps, diff and colours.',
    steps: [
      'Pick a utility tab along the top of Text toolbox.',
      'Paste or type your input — output updates live locally with zero network calls.',
      'Click Copy to send the formatted result straight to your clipboard.',
    ],
    tips: [
      'Regex testing runs in a background worker so complex patterns never freeze the interface.',
    ],
  },
  mini: {
    summary: 'Four instant desktop helpers: Always-on-top window pin, screen colour picker, paste plain text and link tracker cleaner.',
    shortcut: 'Ctrl+Alt+P / C / Shift+V / L',
    requires: 'Window pin, colour picker & plain paste require Windows; link cleaner works everywhere',
    steps: [
      'Press Ctrl+Alt+P while any window is focused to pin or unpin it above all other windows.',
      'Press Ctrl+Alt+C to magnify under the cursor and copy any pixel hex colour.',
      'Press Ctrl+Alt+Shift+V to paste without formatting, or Ctrl+Alt+L to strip tracking parameters from a copied URL.',
    ],
    tips: [
      'Enable automatic link cleaning inside Mini tools to strip utm_ and fbclid trackers silently when you copy URLs.',
    ],
  },
  inbox: {
    summary: 'History of notifications NomNom has shown so you never lose an alert that faded away.',
    steps: [
      'Open Notifications to see recent completion alerts, warnings and reminders.',
      'Click an entry to jump straight to the tool that raised it.',
      'Dismiss single notices or clear the list once reviewed.',
    ],
    tips: [
      'Switch between styled corner toast cards and native OS notices in Settings → General.',
    ],
  },
  rules: {
    summary: 'Automatic folder watchers that move, rename or convert new files to WebP as soon as they appear.',
    steps: [
      'Click New rule, select a watched folder and optionally filter by file extensions or name.',
      'Choose the action (Move, Rename pattern or Convert image to WebP) and click Preview to verify.',
      'Enable the rule — every move is logged in the Activity log with one-click Undo.',
    ],
    tips: [
      'Rules never delete or overwrite existing files, and only act on newly arriving files unless you click Run on existing.',
    ],
  },
  snippets: {
    summary: 'Reusable text templates with dynamic placeholders like {date}, {time} and {clipboard}.',
    shortcut: 'Ctrl+K',
    steps: [
      'Click New snippet, give it a short name and write the template body.',
      'Use {date}, {time} or {clipboard} wherever you want live values inserted automatically.',
      'Press Ctrl+K anywhere, type the snippet name and hit Enter — NomNom pastes it right where your cursor is.',
    ],
    tips: [
      'Ideal for email replies, bug report templates, markdown headers and support macros.',
    ],
  },
  shots: {
    summary: 'Region screenshot capture with built-in annotation editor (arrows, boxes, step numbers, text and blur/redact).',
    shortcut: 'Ctrl+Alt+S',
    requires: 'Windows',
    steps: [
      'Press Ctrl+Alt+S (or click Capture on the Screenshots page) and drag over any region of the screen.',
      'Annotate in the editor using Arrow, Box, Step number, Text, Freehand or Redact blur.',
      'Click Copy to put the marked-up image on your clipboard, or Save to store a PNG.',
    ],
    tips: [
      'Use Ctrl+Z inside the editor to undo strokes one by one.',
    ],
  },
  gamemode: {
    summary: 'Automatically activates when a game is in the foreground: quiets notifications, switches to High Performance power plan and closes chosen background apps.',
    steps: [
      'Open Game mode and switch it on.',
      'Choose whether to hold non-urgent notifications, boost the Windows power plan and optionally list background apps to close while playing.',
      'Play normally — when you exit the game, NomNom restores your previous power plan and delivers held notices.',
    ],
    tips: [
      'NomNom remembers your original power plan on disk, so even a crash or power cut restores it on next start.',
    ],
  },
  qr: {
    summary: 'Generate crisp QR codes for links, text or Wi-Fi credentials, or scan a QR code already visible on your screen.',
    steps: [
      'Pick Text/Link or Wi-Fi mode, enter the content and watch the QR code update live.',
      'Click Save SVG / PNG or Copy to share it.',
      'Use Scan from screen / image to decode any QR code without picking up your phone.',
    ],
    tips: [
      'Wi-Fi QR codes let guests connect to your router just by pointing their phone camera.',
    ],
  },
  compare: {
    summary: 'Compares two folders side by side and copies selected differences safely without ever deleting files.',
    steps: [
      'Select Left folder and Right folder and click Compare (enable Exact byte check if you want hash comparison).',
      'Filter by Only on left, Only on right or Different and choose copy directions per file.',
      'Click Apply — any overwritten file is preserved automatically inside a .nomnom-replaced backup folder.',
    ],
    tips: [
      'Great for syncing USB drives, photo archives or project backups with full visibility before copying.',
    ],
  },
  archives: {
    summary: 'Pack files and folders into clean ZIP archives or inspect and safely unpack existing ZIP files.',
    steps: [
      'In Make a ZIP, add files or folders and click Make the ZIP to save the archive.',
      'In Open a ZIP, pick any .zip file to preview its contents and uncompressed size first.',
      'Click Unpack into a new folder — path-traversal entries are blocked automatically for safety.',
    ],
    tips: [
      'Unpacking always targets a fresh folder so existing files are never mixed or overwritten.',
    ],
  },
  recorder: {
    summary: 'Lightweight area screen recorder that saves clean MP4 videos or compact animated GIFs.',
    shortcut: 'Ctrl+Alt+Shift+R',
    requires: 'Windows + FFmpeg (1-click install)',
    steps: [
      'Open Screen recorder or press Ctrl+Alt+Shift+R to drag the capture rectangle.',
      'Choose Video (MP4) or GIF before starting.',
      'Press Ctrl+Alt+Shift+R again (or click Stop) when finished to open the saved clip.',
    ],
    tips: [
      'Recordings cap safely at 10 minutes and save inside Videos/NomNom.',
    ],
  },
  paths: {
    summary: 'Inspects your Windows PATH variable, flags duplicates and missing folders, and saves an automatic backup before any edit.',
    requires: 'Windows',
    steps: [
      'Open PATH editor to inspect User and Machine PATH entries.',
      'Spot red (missing) or amber (duplicate) folders immediately, reorder or add new folders safely.',
      'Click Save changes — NomNom stores a snapshot so you can roll back with one click.',
    ],
    tips: [
      'Machine PATH is shown for reference while User PATH can be edited without administrator prompts.',
    ],
  },
  monitor: {
    summary: 'Real-time CPU, memory, disk I/O, network throughput and top running processes.',
    steps: [
      'Open System Monitor for live charts and per-process CPU and RAM usage.',
      'Inspect mounted disks and active network interfaces at the bottom.',
      'Optionally enable live stats in the tray tooltip or low-disk alerts in Settings.',
    ],
    tips: [
      'Click column headers in the process list to sort by CPU or memory instantly.',
    ],
  },
};
