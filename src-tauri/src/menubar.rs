use crate::{
    model::{Command, Snapshot},
    state::SharedState,
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager,
};

pub struct MenuBar {
    status: MenuItem<tauri::Wry>,
    pending: MenuItem<tauri::Wry>,
    open: MenuItem<tauri::Wry>,
    settings: MenuItem<tauri::Wry>,
    disable: MenuItem<tauri::Wry>,
    quit: MenuItem<tauri::Wry>,
    feedback: MenuItem<tauri::Wry>,
}
fn text<'a>(locale: &str, ja: &'a str, en: &'a str, zh: &'a str) -> &'a str {
    match locale {
        "en" => en,
        "zh-CN" => zh,
        _ => ja,
    }
}
fn status(snapshot: &Snapshot) -> &'static str {
    let s = &snapshot.extension;
    if s.busy {
        return "busy";
    }
    if ["disabled", "removed"].contains(&s.stage.as_str()) && s.signed {
        return "disabled";
    }
    if ["failed", "disconnected", "stale", "configuration_conflict"].contains(&s.stage.as_str()) {
        return "uncertain";
    }
    if s.signed && s.stage == "applied" && s.instance_id.is_some() && s.provider_revision.is_some()
    {
        return "receipt";
    }
    "observation"
}
fn can_disable(snapshot: &Snapshot) -> bool {
    let s = &snapshot.extension;
    s.signed
        && s.packaged
        && s.runtime_available
        && snapshot.updates.stage != "installing"
        && !(s.busy && s.stage == "disabling")
        && !(!s.busy && ["disabled", "removed"].contains(&s.stage.as_str()))
}
fn show(app: &tauri::AppHandle, view: Option<&str>) {
    if let Some(view) = view {
        if let Ok(s) = app
            .state::<SharedState>()
            .dispatch(Command::SetView { view: view.into() })
        {
            let _ = app.emit("state-changed", s);
        }
    }
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}
pub fn install(app: &tauri::AppHandle) -> tauri::Result<()> {
    let item = |id, label, enabled| MenuItem::with_id(app, id, label, enabled, None::<&str>);
    let bar = MenuBar {
        status: item("tray-status", "FlowGuard", false)?,
        pending: item("tray-pending", "…", true)?,
        open: item("tray-open", "FlowGuard", true)?,
        settings: item("tray-settings", "Settings", true)?,
        disable: item("tray-disable", "Recovery", false)?,
        quit: item("tray-quit", "Quit", true)?,
        feedback: item("tray-feedback", "", false)?,
    };
    let menu = Menu::with_items(
        app,
        &[
            &bar.status,
            &bar.pending,
            &bar.open,
            &bar.settings,
            &bar.disable,
            &bar.feedback,
            &bar.quit,
        ],
    )?;
    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| tauri::Error::AssetNotFound("default icon".into()))?;
    TrayIconBuilder::with_id("flowguard-menubar")
        .icon(icon)
        .icon_as_template(true)
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "tray-open" => show(app, None),
            "tray-pending" => show(app, Some("map")),
            "tray-settings" => show(app, Some("settings")),
            "tray-disable" => {
                let result = crate::disable_extension(app.clone(), false);
                if let Some(bar) = app.try_state::<MenuBar>() {
                    let locale = app
                        .state::<SharedState>()
                        .snapshot()
                        .map(|s| s.document.preferences.locale)
                        .unwrap_or_default();
                    let _ = bar.feedback.set_text(if result.is_err() {
                        text(
                            &locale,
                            "復旧要求を開始できません。設定を確認してください。",
                            "Could not start recovery. Check Settings.",
                            "无法启动恢复，请检查设置。",
                        )
                    } else {
                        ""
                    });
                }
                show(app, None);
            }
            "tray-quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    app.manage(bar);
    refresh(app);
    Ok(())
}
pub fn refresh(app: &tauri::AppHandle) {
    let Some(bar) = app.try_state::<MenuBar>() else {
        return;
    };
    let Ok(s) = app.state::<SharedState>().snapshot() else {
        return;
    };
    let locale = s.document.preferences.locale.as_str();
    let label = match status(&s) {
        "busy" => text(
            locale,
            "拡張機能を操作中・適用未確認",
            "Extension busy · application unconfirmed",
            "扩展处理中 · 应用未确认",
        ),
        "disabled" => text(
            locale,
            "フィルター設定は無効・通信復旧は未確認",
            "Filter disabled · traffic recovery unverified",
            "过滤器已停用 · 通信恢复未确认",
        ),
        "uncertain" => text(
            locale,
            "適用状態を確認できません",
            "Application state unconfirmed",
            "应用状态未确认",
        ),
        "receipt" => text(
            locale,
            "ポリシー受領確認・個別通信は未検証",
            "Policy receipt confirmed · traffic unverified",
            "策略接收已确认 · 通信未验证",
        ),
        _ => text(
            locale,
            "観測のみ・遮断の適用未確認",
            "Observation only · blocking unconfirmed",
            "仅观测 · 阻止应用未确认",
        ),
    };
    let count = s.asks.requests.len();
    let pending = format!(
        "{}: {}",
        text(
            locale,
            "通信の確認待ち",
            "Pending flow decisions",
            "待确认连接"
        ),
        count
    );
    let _ = bar.status.set_text(label);
    let _ = bar.pending.set_text(&pending);
    let _ = bar.open.set_text(text(
        locale,
        "FlowGuardを開く",
        "Open FlowGuard",
        "打开FlowGuard",
    ));
    let _ = bar
        .settings
        .set_text(text(locale, "設定", "Settings", "设置"));
    let _ = bar.disable.set_text(text(
        locale,
        "緊急復旧：フィルターを無効化",
        "Emergency: disable filter",
        "紧急恢复：停用过滤器",
    ));
    let _ = bar.disable.set_enabled(can_disable(&s));
    let _ = bar.quit.set_text(text(
        locale,
        "終了（フィルター設定は保持）",
        "Quit (filter settings retained)",
        "退出（保留过滤器设置）",
    ));
    if let Some(tray) = app.tray_by_id("flowguard-menubar") {
        let _ = tray.set_tooltip(Some(format!("FlowGuard · {label} · {pending}")));
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn uncertain_states_never_claim_protection_and_emergency_preempts_busy() {
        let dir = tempfile::tempdir().unwrap();
        let state = SharedState::load(dir.path().join("document.json")).unwrap();
        let mut s = state.snapshot().unwrap();
        s.extension.stage = "applied".into();
        assert_ne!(status(&s), "receipt");
        s.extension.signed = true;
        s.extension.instance_id = Some("boot".into());
        s.extension.provider_revision = Some(1);
        assert_eq!(status(&s), "receipt");
        s.extension.packaged = true;
        s.extension.runtime_available = true;
        s.extension.busy = true;
        s.extension.stage = "applying".into();
        assert_eq!(status(&s), "busy");
        assert!(can_disable(&s));
        s.extension.stage = "disabling".into();
        assert!(!can_disable(&s));
        s.extension.busy = false;
        s.extension.stage = "disconnected".into();
        assert_eq!(status(&s), "uncertain");
        s.updates.stage = "installing".into();
        assert!(!can_disable(&s));
    }
    #[test]
    fn locales_have_distinct_labels() {
        assert_eq!(text("en", "日", "English", "中"), "English");
        assert_eq!(text("zh-CN", "日", "English", "中"), "中");
    }
}
