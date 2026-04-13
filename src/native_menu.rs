//! macOS ネイティブメニューバー（muda クレート使用）
//! このファイルは macOS 専用。他 OS からは `#[cfg(target_os = "macos")]` で除外される。
#![cfg(target_os = "macos")]

use muda::{
    accelerator::{Accelerator, Code, Modifiers},
    CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu,
};
use objc2_app_kit::{NSAppearance, NSAppearanceNameAqua, NSAppearanceNameDarkAqua, NSApplication};
use objc2_foundation::MainThreadMarker;
use std::cell::RefCell;

// ── アクション ────────────────────────────────────────────────────

/// ネイティブメニューから発行されるアクション
pub enum MenuAction {
    FileOpen,
    FileImportPng,
    FileSave,
    FileSaveAs,
    EditUndo,
    ViewDarkMode(bool),   // トグル後の値
    PaletteOpenPal,
    PaletteOpenDat,
    PaletteSaveDat,
    PaletteReset,
}

// ── メニューアイテムハンドル ──────────────────────────────────────

struct MenuHandles {
    file_open:        MenuItem,
    file_import_png:  MenuItem,
    file_save:        MenuItem,
    file_save_as:     MenuItem,
    edit_undo:        MenuItem,
    view_dark_mode:   CheckMenuItem,
    palette_open_pal: MenuItem,
    palette_open_dat: MenuItem,
    palette_save_dat: MenuItem,
    palette_reset:    MenuItem,
}

thread_local! {
    static HANDLES: RefCell<Option<MenuHandles>> = const { RefCell::new(None) };
}

// ── 初期化 ────────────────────────────────────────────────────────

/// ウィンドウ枠（タイトルバー）を含む macOS アプリ全体の外観を設定する
pub fn set_app_appearance(dark: bool) {
    let Some(mtm) = MainThreadMarker::new() else { return };
    let app = NSApplication::sharedApplication(mtm);
    let name = if dark {
        unsafe { NSAppearanceNameDarkAqua }
    } else {
        unsafe { NSAppearanceNameAqua }
    };
    let appearance = NSAppearance::appearanceNamed(name);
    unsafe { app.setAppearance(appearance.as_deref()); }
}

/// `eframe::run_native()` の前に一度だけ呼ぶ
pub fn init() {
    let cmd       = Modifiers::META;
    let cmd_shift = Modifiers::META | Modifiers::SHIFT;

    let h = MenuHandles {
        file_open:        MenuItem::new("開く…",          true,  Some(Accelerator::new(Some(cmd),       Code::KeyO))),
        file_import_png:  MenuItem::new("PNG をインポート…", true, None),
        file_save:        MenuItem::new("保存",            false, Some(Accelerator::new(Some(cmd),       Code::KeyS))),
        file_save_as:     MenuItem::new("別名で保存…",     true,  Some(Accelerator::new(Some(cmd_shift), Code::KeyS))),
        edit_undo:        MenuItem::new("元に戻す",         false, Some(Accelerator::new(Some(cmd),       Code::KeyZ))),
        view_dark_mode:   CheckMenuItem::new("ダークモード", true, true, None),
        palette_open_pal: MenuItem::new("PAL ファイルを開く…",              true, None),
        palette_open_dat: MenuItem::new("DAT ファイルを開く…",              true, None),
        palette_save_dat: MenuItem::new("DAT ファイルを保存…",              true, None),
        palette_reset:    MenuItem::new("マスターパレットをリセット (NES 標準)", true, None),
    };

    // ── ファイル
    let file = Submenu::new("ファイル", true);
    file.append(&h.file_open).unwrap();
    file.append(&h.file_import_png).unwrap();
    file.append(&PredefinedMenuItem::separator()).unwrap();
    file.append(&h.file_save).unwrap();
    file.append(&h.file_save_as).unwrap();

    // ── 編集
    let edit = Submenu::new("編集", true);
    edit.append(&h.edit_undo).unwrap();

    // ── 表示
    let view = Submenu::new("表示", true);
    view.append(&h.view_dark_mode).unwrap();

    // ── パレット
    let palette = Submenu::new("パレット", true);
    palette.append(&h.palette_open_pal).unwrap();
    palette.append(&h.palette_open_dat).unwrap();
    palette.append(&PredefinedMenuItem::separator()).unwrap();
    palette.append(&h.palette_save_dat).unwrap();
    palette.append(&PredefinedMenuItem::separator()).unwrap();
    palette.append(&h.palette_reset).unwrap();

    // ── macOS: 先頭はアプリ名メニュー（省略するとファイルメニューがそこに入る）
    let app_menu = Submenu::new("R-CHR", true);
    app_menu.append(&PredefinedMenuItem::about(None, None)).unwrap();
    app_menu.append(&PredefinedMenuItem::separator()).unwrap();
    app_menu.append(&PredefinedMenuItem::quit(None)).unwrap();

    // ── ルートメニューに追加してアプリのメニューバーへ
    let menu = Menu::new();
    menu.append(&app_menu).unwrap();
    menu.append(&file).unwrap();
    menu.append(&edit).unwrap();
    menu.append(&view).unwrap();
    menu.append(&palette).unwrap();
    menu.init_for_nsapp();

    HANDLES.with(|slot| *slot.borrow_mut() = Some(h));
}

// ── 公開 API ──────────────────────────────────────────────────────

/// 未処理のメニューイベントを 1 件取り出して MenuAction に変換する。
/// キューが空なら None。
pub fn try_recv_action() -> Option<MenuAction> {
    let event = MenuEvent::receiver().try_recv().ok()?;
    HANDLES.with(|slot| {
        let borrow = slot.borrow();
        let h = borrow.as_ref()?;
        let id = &event.id;
        if      id == h.file_open.id()        { Some(MenuAction::FileOpen) }
        else if id == h.file_import_png.id()  { Some(MenuAction::FileImportPng) }
        else if id == h.file_save.id()        { Some(MenuAction::FileSave) }
        else if id == h.file_save_as.id()     { Some(MenuAction::FileSaveAs) }
        else if id == h.edit_undo.id()        { Some(MenuAction::EditUndo) }
        else if id == h.view_dark_mode.id()   { Some(MenuAction::ViewDarkMode(h.view_dark_mode.is_checked())) }
        else if id == h.palette_open_pal.id() { Some(MenuAction::PaletteOpenPal) }
        else if id == h.palette_open_dat.id() { Some(MenuAction::PaletteOpenDat) }
        else if id == h.palette_save_dat.id() { Some(MenuAction::PaletteSaveDat) }
        else if id == h.palette_reset.id()    { Some(MenuAction::PaletteReset) }
        else { None }
    })
}

/// enabled / checked 状態をアプリ側の状態に合わせて更新する。毎フレーム呼ぶ。
pub fn sync_state(can_save: bool, can_undo: bool, dark_mode: bool) {
    HANDLES.with(|slot| {
        let borrow = slot.borrow();
        let Some(h) = borrow.as_ref() else { return };
        h.file_save.set_enabled(can_save);
        h.edit_undo.set_enabled(can_undo);
        if h.view_dark_mode.is_checked() != dark_mode {
            h.view_dark_mode.set_checked(dark_mode);
        }
    });
}