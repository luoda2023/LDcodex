// LDCodex Freebuff 集成：检测 Freebuff 桌面版安装与汉化补丁状态、
// 管理自定义模型（custom-models.json）、并调用内置补丁引擎执行汉化/还原。
//
// 补丁引擎复用 freebuff 汉化项目打磨好的 freebuff-zh-patch.mjs（自包含，
// 内嵌全部词条/技能包/注入资源），通过 Freebuff 自带 bun（或系统 node）
// 运行。本模块只做：找目录、探状态、管模型、跑引擎。
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;
use serde_json::{Value, json};

/// 仓库内随 LDCodex 分发的补丁脚本（自包含，无外部 txt 依赖）。
pub const FREE_BUFF_PATCH_SCRIPT: &str =
    include_str!("../../../assets/freebuff/freebuff-zh-patch.mjs");

/// Freebuff 桌面版在 %LOCALAPPDATA%\Programs 下的安装目录名。
const FREE_BUFF_INSTALL_DIR_NAME: &str = "@codebufffreebuff-desktop";

/// orchestrator.js 中「离线/登录/防断/重试」等注入的标记前缀。
const ORCH_MARKERS: &[&str] = &[
    "LUODA_OFFLINE_GENERIC_V2",
    "LUODA_AUTHHDR_V1",
    "LUODA_SESSION_KEEPALIVE_V1",
    "LUODA_LOGOUT_HARDEN_V1",
    "LUODA_TIER_UNLOCK_V1",
    "LUODA_CUSTOM_MODEL_REGISTRY_V2",
    "LUODA_GENERIC_BRIDGE_V2",
    "LUODA_MODEL_ALLOWLIST_V1",
    "LUODA_RETRY_FOREVER_V1",
    "LUODA_TURN_RETRY_V1",
    "LUODA_MODEL_API_V1",
    "LUODA_MULTIDEVICE_V1",
];

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct FreeBuffModel {
    pub id: String,
    #[serde(rename = "displayName", default)]
    pub display_name: String,
    #[serde(default)]
    pub tagline: String,
    #[serde(default)]
    pub availability: String,
    #[serde(default)]
    pub data_use: String,
    #[serde(default)]
    pub premium: bool,
    #[serde(default)]
    pub multimodal: bool,
    #[serde(rename = "baseURL", default)]
    pub base_url: String,
    #[serde(rename = "apiKey", default)]
    pub api_key: String,
    #[serde(default)]
    pub provider: String,
}

impl FreeBuffModel {
    fn from_value(value: &Value) -> Option<Self> {
        if !value.is_object() {
            return None;
        }
        let id = value.get("id").and_then(Value::as_str)?.to_string();
        if id.trim().is_empty() {
            return None;
        }
        let s = |key: &str| -> String {
            value
                .get(key)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string()
        };
        let b = |key: &str| -> bool {
            value
                .get(key)
                .and_then(Value::as_bool)
                .unwrap_or(false)
        };
        Some(Self {
            id,
            display_name: s("displayName"),
            tagline: s("tagline"),
            availability: s("availability"),
            data_use: s("dataUse"),
            premium: b("premium"),
            multimodal: b("multimodal"),
            base_url: s("baseURL"),
            api_key: s("apiKey"),
            provider: s("provider"),
        })
    }

}

/// Freebuff 默认安装目录（%LOCALAPPDATA%\Programs\@codebufffreebuff-desktop）。
pub fn default_install_dir() -> PathBuf {
    let local = env::var_os("LOCALAPPDATA").map(PathBuf::from);
    match local {
        Some(local_app_data) => local_app_data
            .join("Programs")
            .join(FREE_BUFF_INSTALL_DIR_NAME),
        None => PathBuf::from(FREE_BUFF_INSTALL_DIR_NAME),
    }
}

/// 探测实际安装目录：默认路径存在即返回；否则在常见盘符下做浅层查找。
pub fn detect_install_dir() -> Option<PathBuf> {
    let default = default_install_dir();
    if is_install_dir(&default) {
        return Some(default);
    }
    // 常见候选根目录（Program Files / 盘符根目录下的 Programs 与直接目录）
    let mut candidates: Vec<PathBuf> = Vec::new();
    for drive in ["C:\\", "D:\\", "E:\\", "F:\\"] {
        candidates.push(PathBuf::from(drive).join("Program Files").join(FREE_BUFF_INSTALL_DIR_NAME));
        candidates.push(PathBuf::from(drive).join("Program Files (x86)").join(FREE_BUFF_INSTALL_DIR_NAME));
        candidates.push(PathBuf::from(drive).join("Programs").join(FREE_BUFF_INSTALL_DIR_NAME));
        candidates.push(PathBuf::from(drive).join(FREE_BUFF_INSTALL_DIR_NAME));
    }
    candidates.into_iter().find(|p| is_install_dir(p))
}

fn is_install_dir(dir: &Path) -> bool {
    dir.join("Freebuff.exe").exists() && dir.join("resources").is_dir()
}

/// orchestrator 目录（resources/orchestrator）。
fn orchestrator_dir(install_dir: &Path) -> PathBuf {
    install_dir.join("resources").join("orchestrator")
}

/// custom-models.json 路径。
pub fn custom_models_path(install_dir: &Path) -> PathBuf {
    orchestrator_dir(install_dir).join("custom-models.json")
}

/// 读取当前安装目录的补丁状态（各功能层标记探测，纯只读）。
pub fn patch_status(install_dir: &Path) -> Value {
    let orb = orchestrator_dir(install_dir).join("orchestrator.js");
    let orb_text = fs::read_to_string(&orb).unwrap_or_default();
    let orb_has = |marker: &str| orb_text.contains(marker);

    let asar_path = install_dir.join("resources").join("app.asar");
    let asar_text = fs::read(&asar_path)
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .unwrap_or_default();
    let asar_has = |marker: &str| asar_text.contains(marker);

    // 渲染层 bundle
    let assets_dir = orchestrator_dir(install_dir).join("ui").join("assets");
    let mut bundle_text = String::new();
    if let Ok(entries) = fs::read_dir(&assets_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with("index-")
                && name.ends_with(".js")
                && !name.contains(".bak")
            {
                bundle_text = fs::read_to_string(entry.path()).unwrap_or_default();
                break;
            }
        }
    }
    let bundle_has = |marker: &str| bundle_text.contains(marker);

    let css_has = |marker: &str| {
        if let Ok(entries) = fs::read_dir(&assets_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.ends_with(".css") && !name.contains(".bak") {
                    if let Ok(css) = fs::read_to_string(entry.path()) {
                        if css.contains(marker) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    };

    let orchestrator_ok = ORCH_MARKERS.iter().all(|m| orb_has(m));
    let renderer_zh = bundle_has("头脑风暴") || bundle_has("队列");
    let ads_closed = bundle_has("__fb_ads_off__")
        || orb_has("__fb_ads_off__")
        || orb_has("async slotAd(threadId) {return null;");
    let shell_zh = asar_has("__fb_zh_inject__") || asar_has("重命名标签页");
    let shell_light = asar_has("nativeTheme.themeSource") || asar_has("#f5f5f7");
    let update_disabled = asar_has("LUODA_NO_UPDATE") || asar_has("FREEBUFF_DISABLE_UPDATE_CHECK");
    let hardware_accel_off = asar_has("app.disableHardwareAcceleration()");
    let width_1200 = css_has("--chat-content-max-width:1200px");
    let light_theme = css_has("/* Freebuff 中文界面补丁");
    let zh_inject = asar_has("__fb_zh_inject__");

    let features: Vec<Value> = vec![
        feature("rendererZh", "界面汉化（渲染层）", renderer_zh, "index-*.js 词条替换为中文"),
        feature("adsClosed", "关闭对话广告", ads_closed, "广告三层防御短路"),
        feature("orchestrator", "后台增强（登录/防断/重试）", orchestrator_ok, "orchestrator.js 全标记"),
        feature("customModels", "自定义模型注册表", orb_has("LUODA_CUSTOM_MODEL_REGISTRY_V2") || orb_has("LUODA_MODEL_API_V1"), "custom-models.json 注册表 + API"),
        feature("shellZh", "原生外壳汉化", shell_zh, "app.asar 菜单/对话框中文"),
        feature("shellLight", "原生外壳浅色", shell_light, "app.asar 原生窗口浅色"),
        feature("updateDisabled", "禁用自动升级", update_disabled, "FREEBUFF_DISABLE_UPDATE_CHECK"),
        feature("hardwareAccelOff", "关闭硬件加速", hardware_accel_off, "app.disableHardwareAcceleration()"),
        feature("width1200", "输入框加宽 1200px", width_1200, "CSS --chat-content-max-width"),
        feature("lightTheme", "浅色主题覆盖", light_theme, "CSS 浅色块"),
        feature("zhInject", "悬停翻译/辅助工具注入", zh_inject, "__fb_zh_inject__ 运行时"),
    ];

    let applied_count = features
        .iter()
        .filter(|f| f.get("applied").and_then(Value::as_bool).unwrap_or(false))
        .count();

    let (detected_version, version_ok) = detect_freebuff_version(install_dir);
    json!({
        "status": "ok",
        "installDir": install_dir.to_string_lossy().into_owned(),
        "installed": true,
        "detectedVersion": detected_version,
        "versionCompatible": version_ok,
        "applied": applied_count,
        "total": features.len(),
        "complete": applied_count >= features.len(),
        "features": features,
    })
}

/// 补丁适配的目标 Freebuff 版本。
const FREE_BUFF_TARGET_VERSION: &str = "0.0.77";

/// 读取 Freebuff.exe 的 ProductVersion 并判断是否适配本补丁（0.0.77）。
/// 返回 (展示用版本号, 是否适配)。读取失败时展示版本为空、适配为 true（不阻塞操作）。
pub fn detect_freebuff_version(install_dir: &Path) -> (String, bool) {
    let exe = install_dir.join("Freebuff.exe");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        if let Ok(output) = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command"])
            .arg(format!(
                "(Get-Item -LiteralPath '{0}').VersionInfo.ProductVersion",
                exe.to_string_lossy().replace("'", "''")
            ))
            .creation_flags(crate::windows_integration::CREATE_NO_WINDOW)
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !stdout.is_empty() && output.status.success() {
                // 0.0.77.0 -> 0.0.77
                let version = stdout.split('.').take(3).collect::<Vec<_>>().join(".");
                let ok = version == FREE_BUFF_TARGET_VERSION;
                return (version, ok);
            }
        }
    }
    (String::new(), true)
}


fn feature(id: &str, label: &str, applied: bool, note: &str) -> Value {
    json!({
        "id": id,
        "label": label,
        "applied": applied,
        "note": note,
    })
}

/// 列出 custom-models.json 中的自定义模型。
pub fn list_models(install_dir: &Path) -> Value {
    let path = custom_models_path(install_dir);
    let models = read_models(&path);
    json!({
        "status": "ok",
        "models": models,
        "path": path.to_string_lossy().into_owned(),
    })
}

fn read_models(path: &Path) -> Vec<FreeBuffModel> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(_) => return Vec::new(),
    };
    let parsed: Value = match serde_json::from_str(&text) {
        Ok(parsed) => parsed,
        Err(_) => return Vec::new(),
    };
    match parsed {
        Value::Array(items) => items
            .iter()
            .filter_map(FreeBuffModel::from_value)
            .collect(),
        _ => Vec::new(),
    }
}

fn write_models(path: &Path, models: &[FreeBuffModel]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json_text = serde_json::to_string_pretty(models).map_err(std::io::Error::other)?;
    fs::write(path, json_text)
}

/// 新增或更新一个自定义模型（id 相同则覆盖）。返回结果 JSON。
pub fn upsert_model(install_dir: &Path, model: &FreeBuffModel) -> Value {
    let path = custom_models_path(install_dir);
    let mut models = read_models(&path);
    let exists = models.iter().any(|m| m.id == model.id);
    models.retain(|m| m.id != model.id);
    models.push(model.clone());
    match write_models(&path, &models) {
        Ok(()) => json!({
            "status": "ok",
            "message": if exists { "模型已更新。" } else { "模型已添加。" },
            "models": models,
        }),
        Err(error) => json!({
            "status": "failed",
            "message": format!("写入 custom-models.json 失败：{error}"),
            "models": models,
        }),
    }
}

/// 删除指定 id 的自定义模型。返回结果 JSON。
pub fn delete_model(install_dir: &Path, id: &str) -> Value {
    let path = custom_models_path(install_dir);
    let mut models = read_models(&path);
    let before = models.len();
    models.retain(|m| m.id != id);
    let removed = models.len() != before;
    match write_models(&path, &models) {
        Ok(()) => json!({
            "status": "ok",
            "message": if removed { "模型已删除。" } else { "未找到该模型。" },
            "models": models,
        }),
        Err(error) => json!({
            "status": "failed",
            "message": format!("写入 custom-models.json 失败：{error}"),
            "models": models,
        }),
    }
}

/// 查找可用的 JS 运行时：优先 Freebuff 自带 bun，其次系统 node。
pub fn find_runtime(install_dir: &Path) -> Option<PathBuf> {
    let bun = install_dir
        .join("resources")
        .join("bun")
        .join("bun.exe");
    if bun.exists() {
        return Some(bun);
    }
    find_executable_on_path("node.exe").or_else(|| find_executable_on_path("bun.exe"))
}

fn find_executable_on_path(name: &str) -> Option<PathBuf> {
    let path_var = env::var_os("PATH")?;
    for dir in env::split_paths(&path_var) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// 把内嵌补丁脚本释放到临时目录，返回脚本路径（调用方负责保留到运行结束）。
pub fn materialize_patch_script() -> std::io::Result<PathBuf> {
    let dir = env::temp_dir().join("ldcodex-freebuff");
    fs::create_dir_all(&dir)?;
    let path = dir.join("freebuff-zh-patch.mjs");
    fs::write(&path, FREE_BUFF_PATCH_SCRIPT)?;
    Ok(path)
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct PatchRunResult {
    pub status: String,
    pub message: String,
    pub exit_code: i32,
    pub output: String,
}

/// 运行补丁引擎（打补丁 / --check / --restore）。
/// 返回结构化结果与合并后的 stdout+stderr 文本。
pub fn run_patch(install_dir: &Path, action: &str) -> PatchRunResult {
    let runtime = match find_runtime(install_dir) {
        Some(runtime) => runtime,
        None => {
            return PatchRunResult {
                status: "failed".to_string(),
                message: "未找到可用的 JS 运行时（Freebuff 自带 bun 或系统 Node.js）。".to_string(),
                exit_code: -1,
                output: String::new(),
            };
        }
    };
    let script = match materialize_patch_script() {
        Ok(script) => script,
        Err(error) => {
            return PatchRunResult {
                status: "failed".to_string(),
                message: format!("释放补丁脚本失败：{error}"),
                exit_code: -1,
                output: String::new(),
            };
        }
    };
    let mut command = Command::new(&runtime);
    command.arg(&script);
    if action == "restore" {
        command.arg("--restore");
    } else if action == "check" {
        command.arg("--check");
    }
    command.arg(install_dir);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(crate::windows_integration::CREATE_NO_WINDOW);
    }
    let output = match command.output() {
        Ok(output) => output,
        Err(error) => {
            return PatchRunResult {
                status: "failed".to_string(),
                message: format!("启动补丁引擎失败：{error}"),
                exit_code: -1,
                output: String::new(),
            };
        }
    };
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.trim().is_empty() {
        if !text.trim().is_empty() {
            text.push('\n');
        }
        text.push_str(&stderr);
    }
    let exit_code = output.status.code().unwrap_or(-1);
    let (status, message) = classify_exit(action, exit_code, &text);
    PatchRunResult {
        status,
        message,
        exit_code,
        output: text,
    }
}

fn classify_exit(action: &str, exit_code: i32, output: &str) -> (String, String) {
    if action == "check" {
        return match exit_code {
            0 => ("ok".to_string(), "补丁完整，无需重打。".to_string()),
            1 => ("partial".to_string(), "补丁部分缺失，建议重跑汉化。".to_string()),
            2 => ("missing".to_string(), "未检测到补丁标记（可能已还原或从未汉化）。".to_string()),
            _ => ("failed".to_string(), summarize_output(output)),
        };
    }
    if exit_code == 0 {
        (
            "ok".to_string(),
            if action == "restore" {
                "已还原官方版。请完全退出并重新打开 Freebuff 查看效果。".to_string()
            } else {
                "汉化完成。请完全退出并重新打开 Freebuff 查看效果。".to_string()
            },
        )
    } else {
        ("failed".to_string(), summarize_output(output))
    }
}

fn summarize_output(output: &str) -> String {
    let tail = output
        .lines()
        .rev()
        .take(6)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n");
    if tail.trim().is_empty() {
        "补丁引擎执行失败。".to_string()
    } else {
        tail
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// 构造一个伪 Freebuff 安装目录（仅含运行所需的最小文件）。
    fn fake_install_dir(root: &Path) -> PathBuf {
        let install = root.join("@codebufffreebuff-desktop");
        let orb = install.join("resources").join("orchestrator");
        fs::create_dir_all(&orb).unwrap();
        fs::write(install.join("Freebuff.exe"), b"MZ").unwrap();
        fs::create_dir_all(install.join("resources")).unwrap();
        install
    }

    #[test]
    fn detect_install_dir_prefers_default_path() {
        let root = tempfile::tempdir().unwrap();
        let install = fake_install_dir(root.path());
        // 手动置入资源目录校验
        fs::create_dir_all(install.join("resources")).unwrap();
        assert!(is_install_dir(&install));
    }

    #[test]
    fn patch_status_reports_missing_when_unpatched() {
        let root = tempfile::tempdir().unwrap();
        let install = fake_install_dir(root.path());
        fs::create_dir_all(install.join("resources")).unwrap();
        // 无 orchestrator.js / app.asar / bundle → 全部未应用
        let status = patch_status(&install);
        assert_eq!(status["status"], "ok");
        assert_eq!(status["installed"], true);
        assert_eq!(status["applied"], 0);
    }

    #[test]
    fn models_roundtrip_upsert_and_delete() {
        let root = tempfile::tempdir().unwrap();
        let install = fake_install_dir(root.path());
        let model = FreeBuffModel {
            id: "codexAPI".to_string(),
            display_name: "codexAPI".to_string(),
            tagline: "LUODA免费模型".to_string(),
            availability: "always".to_string(),
            data_use: "service".to_string(),
            premium: false,
            multimodal: false,
            base_url: "http://47.114.75.115:40000".to_string(),
            api_key: "sk-test".to_string(),
            provider: "codebuff".to_string(),
        };
        let added = upsert_model(&install, &model);
        assert_eq!(added["status"], "ok");
        let listed = list_models(&install);
        let models = listed["models"].as_array().unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0]["id"], "codexAPI");
        assert_eq!(models[0]["baseURL"], "http://47.114.75.115:40000");

        // 更新同 id → 覆盖
        let mut updated = model.clone();
        updated.display_name = "codexAPI 改".to_string();
        let up = upsert_model(&install, &updated);
        assert_eq!(up["status"], "ok");
        let listed2 = list_models(&install);
        assert_eq!(listed2["models"].as_array().unwrap().len(), 1);
        assert_eq!(listed2["models"][0]["displayName"], "codexAPI 改");

        // 删除
        let removed = delete_model(&install, "codexAPI");
        assert_eq!(removed["status"], "ok");
        let listed3 = list_models(&install);
        assert_eq!(listed3["models"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn list_models_ignores_missing_file() {
        let root = tempfile::tempdir().unwrap();
        let install = fake_install_dir(root.path());
        let listed = list_models(&install);
        assert_eq!(listed["status"], "ok");
        assert_eq!(listed["models"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn custom_models_path_points_under_orchestrator() {
        let root = tempfile::tempdir().unwrap();
        let install = fake_install_dir(root.path());
        let path = custom_models_path(&install);
        assert!(path.ends_with("custom-models.json"));
        assert!(path.to_string_lossy().contains("orchestrator"));
    }

    #[test]
    fn feature_flag_shape() {
        let f = feature("a", "测试", true, "说明");
        assert_eq!(f["id"], "a");
        assert_eq!(f["label"], "测试");
        assert_eq!(f["applied"], true);
        assert_eq!(f["note"], "说明");
    }
}
