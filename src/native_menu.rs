//! macOS ネイティブメニューバー（muda クレート使用）
//! このファイルは macOS 専用。他 OS からは `#[cfg(target_os = "macos")]` で除外される。
#![cfg(target_os = "macos")]

use muda::{
    accelerator::{Accelerator, Code, Modifiers},
    CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu,
};
use objc2_app_kit::{NSAlert, NSAlertStyle, NSAppearance, NSAppearanceNameAqua, NSAppearanceNameDarkAqua, NSApplication};
use objc2_foundation::{MainThreadMarker, NSString};
use std::cell::RefCell;

// ── アクション ────────────────────────────────────────────────────

/// ネイティブメニューから発行されるアクション
pub enum MenuAction {
    About,
    FileNew,
    FileOpen,
    FileImportPng,
    FileSave,
    FileSaveAs,
    EditUndo,
    EditCopy,
    EditPaste,
    ViewDarkMode(bool),
    LangEnglish(bool),    // true = English, false = 日本語
    PaletteOpenPal,
    PaletteOpenDat,
    PaletteSaveDat,
    PaletteReset,
}

// ── メニューアイテムハンドル ──────────────────────────────────────

struct MenuHandles {
    about:            MenuItem,
    lang_english:     CheckMenuItem,
    file_new:         MenuItem,
    file_open:        MenuItem,
    file_import_png:  MenuItem,
    file_save:        MenuItem,
    file_save_as:     MenuItem,
    edit_undo:        MenuItem,
    edit_copy:        MenuItem,
    edit_paste:       MenuItem,
    view_dark_mode:   CheckMenuItem,
    palette_open_pal: MenuItem,
    palette_open_dat: MenuItem,
    palette_save_dat: MenuItem,
    palette_reset:    MenuItem,
    // サブメニュー（言語切替時に set_text するため保持）
    sub_file:         Submenu,
    sub_edit:         Submenu,
    sub_view:         Submenu,
    sub_palette:      Submenu,
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

    use crate::editor::i18n::{self, Lang};
    let s = i18n::t(Lang::Ja);

    let mut h = MenuHandles {
        about:            MenuItem::new(s.about,           true,  None),
        lang_english:     CheckMenuItem::new(s.lang_english, true, false, None),
        file_new:         MenuItem::new(s.file_new,        true,  Some(Accelerator::new(Some(cmd),       Code::KeyN))),
        file_open:        MenuItem::new(s.file_open,       true,  Some(Accelerator::new(Some(cmd),       Code::KeyO))),
        file_import_png:  MenuItem::new(s.file_import_png, true, None),
        file_save:        MenuItem::new(s.file_save,       false, Some(Accelerator::new(Some(cmd),       Code::KeyS))),
        file_save_as:     MenuItem::new(s.file_save_as,    true,  Some(Accelerator::new(Some(cmd_shift), Code::KeyS))),
        edit_undo:        MenuItem::new(s.edit_undo,       false, Some(Accelerator::new(Some(cmd),       Code::KeyZ))),
        edit_copy:        MenuItem::new(s.edit_copy,       false, Some(Accelerator::new(Some(cmd),       Code::KeyC))),
        edit_paste:       MenuItem::new(s.edit_paste,      false, Some(Accelerator::new(Some(cmd),       Code::KeyV))),
        view_dark_mode:   CheckMenuItem::new(s.view_dark_mode, true, true, None),
        palette_open_pal: MenuItem::new(s.pal_open_pal,   true, None),
        palette_open_dat: MenuItem::new(s.pal_open_dat,   true, None),
        palette_save_dat: MenuItem::new(s.pal_save_dat,   true, None),
        palette_reset:    MenuItem::new(s.pal_reset,      true, None),
        sub_file:         Submenu::new(s.menu_file,    true),
        sub_edit:         Submenu::new(s.menu_edit,    true),
        sub_view:         Submenu::new(s.menu_view,    true),
        sub_palette:      Submenu::new(s.menu_palette, true),
    };

    // ── ファイル
    h.sub_file.append(&h.file_new).unwrap();
    h.sub_file.append(&PredefinedMenuItem::separator()).unwrap();
    h.sub_file.append(&h.file_open).unwrap();
    h.sub_file.append(&h.file_import_png).unwrap();
    h.sub_file.append(&PredefinedMenuItem::separator()).unwrap();
    h.sub_file.append(&h.file_save).unwrap();
    h.sub_file.append(&h.file_save_as).unwrap();

    // ── 編集
    h.sub_edit.append(&h.edit_undo).unwrap();
    h.sub_edit.append(&PredefinedMenuItem::separator()).unwrap();
    h.sub_edit.append(&h.edit_copy).unwrap();
    h.sub_edit.append(&h.edit_paste).unwrap();

    // ── 表示
    h.sub_view.append(&h.view_dark_mode).unwrap();

    // ── パレット
    h.sub_palette.append(&h.palette_open_pal).unwrap();
    h.sub_palette.append(&h.palette_open_dat).unwrap();
    h.sub_palette.append(&PredefinedMenuItem::separator()).unwrap();
    h.sub_palette.append(&h.palette_save_dat).unwrap();
    h.sub_palette.append(&PredefinedMenuItem::separator()).unwrap();
    h.sub_palette.append(&h.palette_reset).unwrap();

    // ── macOS: 先頭はアプリ名メニュー（省略するとファイルメニューがそこに入る）
    let app_menu = Submenu::new("R-CHR", true);
    app_menu.append(&h.about).unwrap();
    app_menu.append(&PredefinedMenuItem::separator()).unwrap();
    app_menu.append(&h.lang_english).unwrap();
    app_menu.append(&PredefinedMenuItem::separator()).unwrap();
    app_menu.append(&PredefinedMenuItem::quit(None)).unwrap();

    // ── ルートメニューに追加してアプリのメニューバーへ
    let menu = Menu::new();
    menu.append(&app_menu).unwrap();
    menu.append(&h.sub_file).unwrap();
    menu.append(&h.sub_edit).unwrap();
    menu.append(&h.sub_view).unwrap();
    menu.append(&h.sub_palette).unwrap();
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
        if      id == h.about.id()            { Some(MenuAction::About) }
        else if id == h.lang_english.id()     { Some(MenuAction::LangEnglish(h.lang_english.is_checked())) }
        else if id == h.file_new.id()         { Some(MenuAction::FileNew) }
        else if id == h.file_open.id()        { Some(MenuAction::FileOpen) }
        else if id == h.file_import_png.id()  { Some(MenuAction::FileImportPng) }
        else if id == h.file_save.id()        { Some(MenuAction::FileSave) }
        else if id == h.file_save_as.id()     { Some(MenuAction::FileSaveAs) }
        else if id == h.edit_undo.id()        { Some(MenuAction::EditUndo) }
        else if id == h.edit_copy.id()        { Some(MenuAction::EditCopy) }
        else if id == h.edit_paste.id()       { Some(MenuAction::EditPaste) }
        else if id == h.view_dark_mode.id()   { Some(MenuAction::ViewDarkMode(h.view_dark_mode.is_checked())) }
        else if id == h.palette_open_pal.id() { Some(MenuAction::PaletteOpenPal) }
        else if id == h.palette_open_dat.id() { Some(MenuAction::PaletteOpenDat) }
        else if id == h.palette_save_dat.id() { Some(MenuAction::PaletteSaveDat) }
        else if id == h.palette_reset.id()    { Some(MenuAction::PaletteReset) }
        else { None }
    })
}

// ── 未保存変更ダイアログ ──────────────────────────────────────────

/// 未保存変更ダイアログの選択結果
pub enum UnsavedChoice {
    Save,
    Discard,
    Cancel,
}

/// NSAlert で「未保存の変更があります」ダイアログを表示する。
///
/// ボタン: 「保存して閉じる」「保存せず閉じる」「キャンセル」
pub fn unsaved_changes_dialog(file_name: &str, lang: crate::editor::i18n::Lang) -> UnsavedChoice {
    let Some(mtm) = MainThreadMarker::new() else {
        return UnsavedChoice::Cancel;
    };
    let s = crate::editor::i18n::t(lang);
    let alert = unsafe { NSAlert::new(mtm) };
    unsafe {
        alert.setMessageText(&NSString::from_str(s.unsaved_title));
        alert.setInformativeText(&NSString::from_str(&lang.fmt_unsaved_body(file_name)));
        alert.setAlertStyle(NSAlertStyle::Warning);
        alert.addButtonWithTitle(&NSString::from_str(s.save_and_close));
        alert.addButtonWithTitle(&NSString::from_str(s.discard_btn));
        alert.addButtonWithTitle(&NSString::from_str(s.cancel_btn2));
        let response = alert.runModal();
        // NSAlertFirstButtonReturn=1000, Second=1001, Third=1002
        match response {
            1000 => UnsavedChoice::Save,
            1001 => UnsavedChoice::Discard,
            _    => UnsavedChoice::Cancel,
        }
    }
}

/// enabled / checked 状態をアプリ側の状態に合わせて更新する。毎フレーム呼ぶ。
pub fn sync_state(can_save: bool, can_undo: bool, can_copy: bool, can_paste: bool, dark_mode: bool, lang: crate::editor::i18n::Lang) {
    HANDLES.with(|slot| {
        let borrow = slot.borrow();
        let Some(h) = borrow.as_ref() else { return };
        h.file_save.set_enabled(can_save);
        h.edit_undo.set_enabled(can_undo);
        h.edit_copy.set_enabled(can_copy);
        h.edit_paste.set_enabled(can_paste);
        if h.view_dark_mode.is_checked() != dark_mode {
            h.view_dark_mode.set_checked(dark_mode);
        }
        let is_en = lang == crate::editor::i18n::Lang::En;
        if h.lang_english.is_checked() != is_en {
            h.lang_english.set_checked(is_en);
        }
    });
}

/// 言語切替時にすべてのメニューアイテムのテキストを更新する。
pub fn set_menu_lang(lang: crate::editor::i18n::Lang) {
    let s = crate::editor::i18n::t(lang);
    HANDLES.with(|slot| {
        let borrow = slot.borrow();
        let Some(h) = borrow.as_ref() else { return };
        h.about.set_text(s.about);
        h.file_new.set_text(s.file_new);
        h.file_open.set_text(s.file_open);
        h.file_import_png.set_text(s.file_import_png);
        h.file_save.set_text(s.file_save);
        h.file_save_as.set_text(s.file_save_as);
        h.edit_undo.set_text(s.edit_undo);
        h.edit_copy.set_text(s.edit_copy);
        h.edit_paste.set_text(s.edit_paste);
        h.view_dark_mode.set_text(s.view_dark_mode);
        h.palette_open_pal.set_text(s.pal_open_pal);
        h.palette_open_dat.set_text(s.pal_open_dat);
        h.palette_save_dat.set_text(s.pal_save_dat);
        h.palette_reset.set_text(s.pal_reset);
        h.sub_file.set_text(s.menu_file);
        h.sub_edit.set_text(s.menu_edit);
        h.sub_view.set_text(s.menu_view);
        h.sub_palette.set_text(s.menu_palette);
    });
}