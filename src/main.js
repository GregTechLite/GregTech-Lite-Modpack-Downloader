import { invoke } from "@tauri-apps/api/core";
import "./styles.css";

const app = document.querySelector("#app");

app.innerHTML = `
  <main class="layout">
    <section class="card">
      <h1>GregTech Lite 导出工具</h1>
      <form id="install-form" class="form">
        <label>
          工作目录
          <input id="work-dir" type="text"/>
          <small>留空时默认使用系统下载目录（Windows: %USERPROFILE%\\Downloads\\GTLite）。</small>
        </label>

        <label>
          输出目录
          <input id="output-dir" type="text"/>
          <small>留空时默认输出到工作目录。</small>
        </label>

        <label>
          输出文件名
          <input id="output-name" type="text" value="GregTech-Lite-Modpack.cf.zip" />
        </label>

        <button id="run-btn" type="submit">开始下载并导出</button>
      </form>
    </section>

    <section class="card">
      <div class="status-row">
        <h2>执行日志</h2>
        <span id="status" class="status idle">空闲</span>
      </div>
      <pre id="logs" class="logs">等待执行...</pre>
      <p id="result" class="result"></p>
    </section>
  </main>
`;

const form = document.querySelector("#install-form");
const workDirInput = document.querySelector("#work-dir");
const outputDirInput = document.querySelector("#output-dir");
const outputNameInput = document.querySelector("#output-name");
const logsEl = document.querySelector("#logs");
const resultEl = document.querySelector("#result");
const statusEl = document.querySelector("#status");
const runBtn = document.querySelector("#run-btn");

function setStatus(kind, text) {
  statusEl.className = `status ${kind}`;
  statusEl.textContent = text;
}

form.addEventListener("submit", async (event) => {
  event.preventDefault();

  const outputFilename = outputNameInput.value.trim();
  if (!outputFilename) {
    setStatus("error", "参数错误");
    logsEl.textContent = "输出文件名不能为空。";
    return;
  }

  setStatus("running", "执行中");
  runBtn.disabled = true;
  resultEl.textContent = "";
  logsEl.textContent = "任务已启动...";

  try {
    const res = await invoke("run_install", {
      workDir: workDirInput.value.trim() || null,
      outputDir: outputDirInput.value.trim() || null,
      outputFilename,
    });

    setStatus("success", "完成");
    logsEl.textContent = (res.logs || []).join("\n");
    resultEl.textContent = `导出文件：${res.output_path}`;
  } catch (error) {
    setStatus("error", "失败");
    logsEl.textContent = String(error);
  } finally {
    runBtn.disabled = false;
  }
});
