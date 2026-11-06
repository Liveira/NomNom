//! NOMNOM's entry in the Windows 11 right-click menu.
//!
//! Explorer loads this DLL into its own process, so it does as little as possible: no file reads
//! except the small `integration.json`, no network, and the NomNom program is only started when the
//! user picks an entry. What the menu offers comes from `nomnom_win::shell_menu::TABLE`.
#![allow(non_snake_case)]
#![allow(linker_messages)]

mod launch;

pub use launch::{integration_path, read_integration, reset_cache, start_with, Integration};

use nomnom_win::shell_menu::{entries_for, Entry, MENU_TITLE, SHELL_CLSID};
use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::path::PathBuf;
use windows::core::{implement, w, Error, Interface, Result, GUID, HRESULT, PWSTR};
use windows::Win32::Foundation::{
    BOOL, CLASS_E_CLASSNOTAVAILABLE, CLASS_E_NOAGGREGATION, E_FAIL, E_NOTIMPL, E_POINTER, S_FALSE,
    S_OK,
};
use windows::Win32::System::Com::{IBindCtx, IClassFactory, IClassFactory_Impl};
use windows::Win32::UI::Shell::{
    IEnumExplorerCommand, IEnumExplorerCommand_Impl, IExplorerCommand, IExplorerCommand_Impl,
    IShellItemArray, SHStrDupW, ECF_DEFAULT, ECF_HASSUBCOMMANDS, ECS_ENABLED, ECS_HIDDEN,
    SIGDN_FILESYSPATH,
};

const CLSID: GUID = GUID::from_u128(0x7C2D5B1E_3A64_4F0B_9A6E_5D1C8B7F2E40);

/// The extensions and paths of what is selected, read without touching the files.
pub struct Selected {
    pub paths: Vec<PathBuf>,
    pub exts: Vec<String>,
}

fn selected(items: Option<&IShellItemArray>) -> Result<Selected> {
    let items = items.ok_or_else(|| Error::from(E_POINTER))?;
    let count = unsafe { items.GetCount()? };
    let mut paths = Vec::new();
    let mut exts = Vec::new();
    for i in 0..count {
        let item = unsafe { items.GetItemAt(i)? };
        // Items that aren't files (a library, This PC) have no file system path: nothing to offer.
        let Ok(name) = (unsafe { item.GetDisplayName(SIGDN_FILESYSPATH) }) else {
            return Ok(Selected {
                paths: vec![],
                exts: vec![],
            });
        };
        let path = PathBuf::from(unsafe { name.to_string() }.unwrap_or_default());
        unsafe { windows::Win32::System::Com::CoTaskMemFree(Some(name.0 as *const c_void)) };
        match path.extension().and_then(|e| e.to_str()) {
            Some(e) => exts.push(e.to_ascii_lowercase()),
            // A folder or a file with no extension: NOMNOM has nothing for it.
            None => {
                return Ok(Selected {
                    paths: vec![],
                    exts: vec![],
                })
            }
        }
        paths.push(path);
    }
    Ok(Selected { paths, exts })
}

fn dup(text: &str) -> Result<PWSTR> {
    let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe { SHStrDupW(windows::core::PCWSTR(wide.as_ptr())) }
}

// ---- the NOMNOM entry with its submenu ---------------------------------------------------------

#[implement(IExplorerCommand)]
struct Root {
    /// Chosen in `GetState`, which Explorer calls first; `EnumSubCommands` has no selection of its own.
    entries: RefCell<Vec<&'static Entry>>,
}

impl IExplorerCommand_Impl for Root_Impl {
    fn GetTitle(&self, _items: Option<&IShellItemArray>) -> Result<PWSTR> {
        dup(MENU_TITLE)
    }
    fn GetIcon(&self, _items: Option<&IShellItemArray>) -> Result<PWSTR> {
        match launch::read_integration(&launch::integration_path()).exe {
            Some(exe) => dup(&format!("{},0", exe.display())),
            None => Err(E_NOTIMPL.into()),
        }
    }
    fn GetToolTip(&self, _items: Option<&IShellItemArray>) -> Result<PWSTR> {
        Err(E_NOTIMPL.into())
    }
    fn GetCanonicalName(&self) -> Result<GUID> {
        Ok(CLSID)
    }
    fn GetState(&self, items: Option<&IShellItemArray>, _slow: BOOL) -> Result<u32> {
        let enabled = launch::menu_enabled();
        let sel = selected(items).unwrap_or(Selected {
            paths: vec![],
            exts: vec![],
        });
        let entries = if enabled {
            entries_for(&sel.exts, sel.paths.len())
        } else {
            vec![]
        };
        let hidden = entries.is_empty();
        *self.entries.borrow_mut() = entries;
        Ok(if hidden {
            ECS_HIDDEN.0 as u32
        } else {
            ECS_ENABLED.0 as u32
        })
    }
    fn Invoke(&self, _items: Option<&IShellItemArray>, _bind: Option<&IBindCtx>) -> Result<()> {
        // Choosing the parent entry itself does nothing; the work is in the submenu.
        Ok(())
    }
    fn GetFlags(&self) -> Result<u32> {
        Ok(ECF_HASSUBCOMMANDS.0 as u32)
    }
    fn EnumSubCommands(&self) -> Result<IEnumExplorerCommand> {
        let commands: Vec<IExplorerCommand> = self
            .entries
            .borrow()
            .iter()
            .map(|e| Sub { entry: e }.into())
            .collect();
        Ok(Subs {
            commands,
            pos: Cell::new(0),
        }
        .into())
    }
}

#[implement(IExplorerCommand)]
struct Sub {
    entry: &'static Entry,
}

impl IExplorerCommand_Impl for Sub_Impl {
    fn GetTitle(&self, _items: Option<&IShellItemArray>) -> Result<PWSTR> {
        dup(self.entry.title)
    }
    fn GetIcon(&self, _items: Option<&IShellItemArray>) -> Result<PWSTR> {
        Err(E_NOTIMPL.into())
    }
    fn GetToolTip(&self, _items: Option<&IShellItemArray>) -> Result<PWSTR> {
        Err(E_NOTIMPL.into())
    }
    fn GetCanonicalName(&self) -> Result<GUID> {
        // Not used by Explorer; a stable value derived from the entry id keeps it unique.
        let mut h: u128 = CLSID.to_u128();
        for b in self.entry.id.bytes() {
            h = h.rotate_left(5) ^ u128::from(b);
        }
        Ok(GUID::from_u128(h))
    }
    fn GetState(&self, items: Option<&IShellItemArray>, _slow: BOOL) -> Result<u32> {
        let sel = selected(items).unwrap_or(Selected {
            paths: vec![],
            exts: vec![],
        });
        let show = launch::menu_enabled()
            && nomnom_win::shell_menu::entry_matches(self.entry, &sel.exts, sel.paths.len());
        Ok(if show {
            ECS_ENABLED.0 as u32
        } else {
            ECS_HIDDEN.0 as u32
        })
    }
    fn Invoke(&self, items: Option<&IShellItemArray>, _bind: Option<&IBindCtx>) -> Result<()> {
        let sel = selected(items)?;
        if !nomnom_win::shell_menu::entry_matches(self.entry, &sel.exts, sel.paths.len()) {
            return Err(E_FAIL.into());
        }
        launch::start(self.entry.action, &sel.paths).map_err(|_| Error::from(E_FAIL))
    }
    fn GetFlags(&self) -> Result<u32> {
        Ok(ECF_DEFAULT.0 as u32)
    }
    fn EnumSubCommands(&self) -> Result<IEnumExplorerCommand> {
        Err(E_NOTIMPL.into())
    }
}

#[implement(IEnumExplorerCommand)]
struct Subs {
    commands: Vec<IExplorerCommand>,
    pos: Cell<usize>,
}

impl IEnumExplorerCommand_Impl for Subs_Impl {
    fn Next(&self, celt: u32, out: *mut Option<IExplorerCommand>, fetched: *mut u32) -> HRESULT {
        if out.is_null() {
            return E_POINTER;
        }
        let mut n = 0usize;
        while n < celt as usize && self.pos.get() < self.commands.len() {
            // SAFETY: Explorer gives room for `celt` entries.
            unsafe {
                out.add(n)
                    .write(Some(self.commands[self.pos.get()].clone()))
            };
            self.pos.set(self.pos.get() + 1);
            n += 1;
        }
        if !fetched.is_null() {
            // SAFETY: checked for null; the caller owns the location.
            unsafe { fetched.write(n as u32) };
        }
        if n == celt as usize {
            S_OK
        } else {
            S_FALSE
        }
    }
    fn Skip(&self, celt: u32) -> Result<()> {
        self.pos
            .set((self.pos.get() + celt as usize).min(self.commands.len()));
        Ok(())
    }
    fn Reset(&self) -> Result<()> {
        self.pos.set(0);
        Ok(())
    }
    fn Clone(&self) -> Result<IEnumExplorerCommand> {
        Ok(Subs {
            commands: self.commands.clone(),
            pos: Cell::new(self.pos.get()),
        }
        .into())
    }
}

// ---- the COM entry points -----------------------------------------------------------------------

#[implement(IClassFactory)]
struct Factory;

impl IClassFactory_Impl for Factory_Impl {
    fn CreateInstance(
        &self,
        outer: Option<&windows::core::IUnknown>,
        iid: *const GUID,
        object: *mut *mut c_void,
    ) -> Result<()> {
        if outer.is_some() {
            return Err(CLASS_E_NOAGGREGATION.into());
        }
        let root: IExplorerCommand = Root {
            entries: RefCell::new(Vec::new()),
        }
        .into();
        // SAFETY: Explorer passes valid pointers.
        unsafe { root.query(iid, object).ok() }
    }
    fn LockServer(&self, _lock: BOOL) -> Result<()> {
        Ok(())
    }
}

/// # Safety
/// Called by COM with valid pointers.
#[no_mangle]
pub unsafe extern "system" fn DllGetClassObject(
    clsid: *const GUID,
    iid: *const GUID,
    object: *mut *mut c_void,
) -> HRESULT {
    if clsid.is_null() || iid.is_null() || object.is_null() {
        return E_POINTER;
    }
    if *clsid != CLSID {
        return CLASS_E_CLASSNOTAVAILABLE;
    }
    let factory: IClassFactory = Factory.into();
    factory.query(iid, object)
}

/// Explorer decides when to unload; this DLL keeps no global state that would make it unsafe.
#[no_mangle]
pub extern "system" fn DllCanUnloadNow() -> HRESULT {
    S_FALSE
}

#[allow(dead_code)]
fn _keep(_: &str) {
    let _ = (SHELL_CLSID, w!(""));
}
