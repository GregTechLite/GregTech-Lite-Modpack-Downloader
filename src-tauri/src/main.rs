#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::{
  fs::{self, File},
  io::{self, copy, Write},
  path::{Path, PathBuf},
  process::Command,
};

use anyhow::{anyhow, Context};
use serde::Serialize;
use tauri::command;
use zip::ZipArchive;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

const PACKWIZ_URL: &str = "https://nightly.link/packwiz/packwiz/workflows/go/main/Windows%2064-bit.zip";
const MODPACK_URL: &str =
  "https://github.com/GregTechLite/GregTech-Lite-Modpack/archive/refs/heads/main.zip";
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Serialize)]
struct InstallResult {
  output_path: String,
  logs: Vec<String>,
}

#[command]
async fn run_install(
  work_dir: Option<String>,
  output_dir: Option<String>,
  output_filename: Option<String>,
) -> Result<InstallResult, String> {
  let work_dir = work_dir
    .map(PathBuf::from)
    .unwrap_or_else(default_work_dir);

  let output_dir = output_dir.map(PathBuf::from).unwrap_or_else(|| work_dir.clone());
  let output_filename = output_filename
    .unwrap_or_else(|| "GregTech-Lite-Modpack.cf.zip".to_string())
    .trim()
    .to_string();

  if output_filename.is_empty() {
    return Err("导出文件名不能为空".to_string());
  }

  let task = tauri::async_runtime::spawn_blocking(move || {
    let mut logs = Vec::new();
    logs.push(format!("工作目录: {}", work_dir.display()));

    fs::create_dir_all(&work_dir).context("创建工作目录失败")?;
    fs::create_dir_all(&output_dir).context("创建输出目录失败")?;

    let packwiz_archive = work_dir.join("packwiz.zip");
    let modpack_archive = work_dir.join("modpack.zip");

    maybe_download(PACKWIZ_URL, &packwiz_archive, &mut logs)?;
    let packwiz_extract_dir = work_dir.join("packwiz");
    unpack_zip(&packwiz_archive, &packwiz_extract_dir, &mut logs)?;

    maybe_download(MODPACK_URL, &modpack_archive, &mut logs)?;
    let modpack_extract_dir = work_dir.join("modpack");
    unpack_zip(&modpack_archive, &modpack_extract_dir, &mut logs)?;

    let modpack_root = find_modpack_root(&modpack_extract_dir)?;
    let packwiz_exe = packwiz_extract_dir.join("packwiz.exe");
    if !packwiz_exe.exists() {
      return Err(anyhow!("未找到 packwiz.exe: {}", packwiz_exe.display()));
    }

    let output_path = output_dir.join(output_filename);
    logs.push(format!("开始导出: {}", output_path.display()));

    let mut command = Command::new(&packwiz_exe);
    command
      .args(["curseforge", "export", "-y", "-o"])
      .arg(&output_path)
      .current_dir(&modpack_root);
    #[cfg(target_os = "windows")]
    command.creation_flags(CREATE_NO_WINDOW);

    let result = command.output().context("执行 packwiz 导出失败")?;

    if !result.status.success() {
      let stderr = String::from_utf8_lossy(&result.stderr);
      return Err(anyhow!("packwiz 导出失败: {}", stderr.trim()));
    }

    let stdout = String::from_utf8_lossy(&result.stdout).trim().to_string();
    if !stdout.is_empty() {
      logs.push(format!("packwiz 输出: {}", stdout));
    }
    logs.push("导出完成".to_string());

    Ok::<InstallResult, anyhow::Error>(InstallResult {
      output_path: output_path.display().to_string(),
      logs,
    })
  });

  task
    .await
    .map_err(|e| format!("任务执行失败: {e}"))?
    .map_err(|e| e.to_string())
}

fn default_work_dir() -> PathBuf {
  dirs::download_dir()
    .map(|p| p.join("GTLite"))
    .or_else(|| std::env::current_dir().ok())
    .unwrap_or_else(|| PathBuf::from("."))
}

fn maybe_download(url: &str, target: &Path, logs: &mut Vec<String>) -> anyhow::Result<()> {
  if target.exists() {
    logs.push(format!("已存在，跳过下载: {}", target.display()));
    return Ok(());
  }

  logs.push(format!("下载中: {url}"));
  let response = reqwest::blocking::get(url).context("下载请求失败")?;
  if !response.status().is_success() {
    return Err(anyhow!("下载失败({}): {url}", response.status()));
  }

  let mut file = File::create(target).context("创建下载文件失败")?;
  let mut content = io::Cursor::new(response.bytes().context("读取下载内容失败")?);
  copy(&mut content, &mut file).context("写入下载文件失败")?;
  file.flush().ok();

  logs.push(format!("下载完成: {}", target.display()));
  Ok(())
}

fn unpack_zip(zip_path: &Path, extract_to: &Path, logs: &mut Vec<String>) -> anyhow::Result<()> {
  logs.push(format!("解压: {} -> {}", zip_path.display(), extract_to.display()));
  if extract_to.exists() {
    fs::remove_dir_all(extract_to).context("清理旧解压目录失败")?;
  }
  fs::create_dir_all(extract_to).context("创建解压目录失败")?;

  let file = File::open(zip_path).context("打开 zip 文件失败")?;
  let mut archive = ZipArchive::new(file).context("读取 zip 文件失败")?;

  for i in 0..archive.len() {
    let mut item = archive.by_index(i).context("读取 zip 条目失败")?;
    let enclosed = item
      .enclosed_name()
      .ok_or_else(|| anyhow!("zip 条目路径非法"))?;
    let out_path = extract_to.join(enclosed);

    if item.is_dir() {
      fs::create_dir_all(&out_path).context("创建目录失败")?;
      continue;
    }

    if let Some(parent) = out_path.parent() {
      fs::create_dir_all(parent).context("创建父目录失败")?;
    }

    let mut out_file = File::create(&out_path).context("创建输出文件失败")?;
    copy(&mut item, &mut out_file).context("写入解压文件失败")?;
  }

  logs.push("解压完成".to_string());
  Ok(())
}

fn find_modpack_root(modpack_extract_dir: &Path) -> anyhow::Result<PathBuf> {
  let exact = modpack_extract_dir.join("GregTech-Lite-Modpack-main");
  if exact.exists() {
    return Ok(exact);
  }

  let found = fs::read_dir(modpack_extract_dir)
    .context("读取 modpack 解压目录失败")?
    .filter_map(Result::ok)
    .find_map(|entry| {
      let path = entry.path();
      let name = path.file_name()?.to_string_lossy().to_lowercase();
      if path.is_dir() && name.starts_with("gregtech-lite-modpack-") {
        Some(path)
      } else {
        None
      }
    });

  found.ok_or_else(|| anyhow!("未找到模组包目录"))
}

fn main() {
  tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![run_install])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
