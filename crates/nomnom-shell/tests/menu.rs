//! Loads the menu the way Explorer does (through COM) against a throwaway folder and a stand-in
//! for nomnom.exe that records its arguments. One test, because it changes process-wide settings.

use nomnom_shell::{reset_cache, DllGetClassObject};
use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use windows::core::{Interface, GUID, HSTRING, PCWSTR};
use windows::Win32::Foundation::CLASS_E_CLASSNOTAVAILABLE;
use windows::Win32::System::Com::{
    CoInitializeEx, CoTaskMemFree, IClassFactory, COINIT_APARTMENTTHREADED,
};
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
use windows::Win32::UI::Shell::{
    IExplorerCommand, IShellItemArray, SHCreateShellItemArrayFromIDLists, SHParseDisplayName,
};

const CLSID: GUID = GUID::from_u128(0x7C2D5B1E_3A64_4F0B_9A6E_5D1C8B7F2E40);

fn array(paths: &[PathBuf]) -> IShellItemArray {
    let mut pidls: Vec<*mut ITEMIDLIST> = Vec::new();
    for p in paths {
        let mut pidl = std::ptr::null_mut();
        unsafe {
            SHParseDisplayName(&HSTRING::from(p.as_os_str()), None, &mut pidl, 0, None).unwrap()
        };
        pidls.push(pidl);
    }
    let consts: Vec<*const ITEMIDLIST> = pidls.iter().map(|p| *p as *const _).collect();
    let arr = unsafe { SHCreateShellItemArrayFromIDLists(&consts).unwrap() };
    for p in pidls {
        unsafe { CoTaskMemFree(Some(p as *const c_void)) };
    }
    arr
}

fn text(p: windows::core::PWSTR) -> String {
    let s = unsafe { p.to_string().unwrap() };
    unsafe { CoTaskMemFree(Some(p.0 as *const c_void)) };
    s
}

fn titles(cmd: &IExplorerCommand, sel: &IShellItemArray) -> Vec<String> {
    let flags = unsafe { cmd.GetFlags().unwrap() };
    assert_eq!(flags & 1, 1, "the NOMNOM entry must open a submenu");
    let en = unsafe { cmd.EnumSubCommands().unwrap() };
    let mut out = Vec::new();
    loop {
        let mut item: [Option<IExplorerCommand>; 1] = [None];
        let mut got = 0u32;
        let hr = unsafe { en.Next(&mut item, Some(&mut got)) };
        if got == 0 {
            let _ = hr;
            break;
        }
        let sub = item[0].take().unwrap();
        assert_eq!(
            unsafe { sub.GetState(sel, false).unwrap() },
            0,
            "a listed entry is enabled"
        );
        out.push(text(unsafe { sub.GetTitle(sel).unwrap() }));
    }
    out
}

fn sub(cmd: &IExplorerCommand, title: &str) -> IExplorerCommand {
    let en = unsafe { cmd.EnumSubCommands().unwrap() };
    loop {
        let mut item: [Option<IExplorerCommand>; 1] = [None];
        let mut got = 0u32;
        let _ = unsafe { en.Next(&mut item, Some(&mut got)) };
        assert!(got > 0, "no entry called {title}");
        let c = item[0].take().unwrap();
        if text(unsafe { c.GetTitle(None).unwrap() }) == title {
            return c;
        }
    }
}

fn wait_for(path: &Path) -> String {
    let t = Instant::now();
    while t.elapsed() < Duration::from_secs(10) {
        if let Ok(s) = std::fs::read_to_string(path) {
            if !s.is_empty() {
                return s;
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("the stand-in program never wrote {}", path.display());
}

#[test]
fn the_menu_works_the_way_explorer_uses_it() {
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap() };
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    // Everything NOMNOM-related is looked up through these, so nothing real is read or written.
    std::env::set_var("LOCALAPPDATA", root);
    std::env::set_var("TEMP", root);
    std::env::set_var("TMP", root);
    let argdump = root.join("argdump.txt");
    std::env::set_var("NOMNOM_ARGDUMP", &argdump);

    let app = root.join("app");
    std::fs::create_dir_all(&app).unwrap();
    let exe = app.join("nomnom.exe");
    std::fs::copy(env!("CARGO_BIN_EXE_nomnom-argdump"), &exe).unwrap();
    let write_integration = |on: bool| {
        let dir = root.join("NOMNOM");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("integration.json"),
            serde_json::json!({ "exe": exe, "shellMenu": on }).to_string(),
        )
        .unwrap();
        reset_cache();
    };
    write_integration(true);

    let pics = root.join("pics");
    std::fs::create_dir_all(&pics).unwrap();
    let file = |name: &str| {
        let p = pics.join(name);
        std::fs::write(&p, b"x").unwrap();
        p
    };
    let png = file("a b.png");
    let jpg = file("c.jpg");

    // A factory for our class, and a refusal for any other.
    let mut factory: *mut c_void = std::ptr::null_mut();
    let hr = unsafe { DllGetClassObject(&CLSID, &IClassFactory::IID, &mut factory) };
    assert!(hr.is_ok());
    let factory = unsafe { IClassFactory::from_raw(factory) };
    let mut nothing: *mut c_void = std::ptr::null_mut();
    let other = GUID::from_u128(1);
    assert_eq!(
        unsafe { DllGetClassObject(&other, &IClassFactory::IID, &mut nothing) },
        CLASS_E_CLASSNOTAVAILABLE
    );

    let cmd: IExplorerCommand = unsafe { factory.CreateInstance(None).unwrap() };
    assert_eq!(text(unsafe { cmd.GetTitle(None).unwrap() }), "NOMNOM");

    // One picture: its own conversions.
    let one = array(std::slice::from_ref(&png));
    assert_eq!(unsafe { cmd.GetState(&one, false).unwrap() }, 0);
    assert_eq!(
        titles(&cmd, &one),
        [
            "Convert to WebP",
            "Convert to JPG",
            "Convert to PDF",
            "More options…"
        ]
    );

    // Two pictures of different types: only what fits both, and combining becomes possible.
    let two = array(&[png.clone(), jpg.clone()]);
    assert_eq!(unsafe { cmd.GetState(&two, false).unwrap() }, 0);
    assert_eq!(
        titles(&cmd, &two),
        [
            "Convert to WebP",
            "Convert to PDF",
            "Combine into one PDF",
            "More options…"
        ]
    );

    // Choosing an entry starts NomNom with every file, quoted, once.
    let webp = sub(&cmd, "Convert to WebP");
    unsafe { webp.Invoke(&two, None).unwrap() };
    let args: Vec<String> = wait_for(&argdump).lines().map(String::from).collect();
    assert_eq!(
        args,
        [
            "--shell-action",
            "convert:webp",
            "--files",
            &png.to_string_lossy(),
            &jpg.to_string_lossy()
        ]
    );
    std::fs::remove_file(&argdump).unwrap();

    // A big selection goes through a list file inside NOMNOM's own temp folder.
    let many: Vec<PathBuf> = (0..40).map(|i| file(&format!("p{i}.png"))).collect();
    let big = array(&many);
    assert_eq!(unsafe { cmd.GetState(&big, false).unwrap() }, 0);
    let more = sub(&cmd, "More options…");
    unsafe { more.Invoke(&big, None).unwrap() };
    let args: Vec<String> = wait_for(&argdump).lines().map(String::from).collect();
    assert_eq!(&args[..3], ["--shell-action", "choose", "--files-from"]);
    let list = PathBuf::from(&args[3]);
    assert!(list.starts_with(root.join("nomnom")), "{}", list.display());
    assert_eq!(std::fs::read_to_string(&list).unwrap().lines().count(), 40);
    std::fs::remove_file(&argdump).unwrap();

    // Nothing for programs, folders, or a mix.
    let exe_file = file("setup.exe");
    for sel in [
        array(std::slice::from_ref(&exe_file)),
        array(std::slice::from_ref(&pics)),
        array(&[png.clone(), exe_file.clone()]),
    ] {
        assert_eq!(unsafe { cmd.GetState(&sel, false).unwrap() }, 2, "hidden");
    }

    // Switched off in the app: gone at once, and an old submenu can't run anything.
    write_integration(false);
    assert_eq!(unsafe { cmd.GetState(&one, false).unwrap() }, 2);
    assert!(unsafe { webp.GetState(&two, false).unwrap() } == 2);

    // Pointing at some other program is refused.
    std::fs::write(
        root.join("NOMNOM").join("integration.json"),
        serde_json::json!({ "exe": root.join("argdump.txt"), "shellMenu": true }).to_string(),
    )
    .unwrap();
    reset_cache();
    assert_eq!(unsafe { cmd.GetState(&one, false).unwrap() }, 2);
    let _ = PCWSTR::null();
}
