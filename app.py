# -*- coding: utf-8 -*-
"""YSN 部署助手 — 桌面 GUI（pywebview）
两个子页：文件部署（拖拽→自动定位→确认替换→自动备份）、构建发布（内嵌构建命令 + 中文实时日志）。
"""
import os
import sys
import json
import base64
import shutil
import threading
import subprocess
import datetime
import tempfile

try:
    import webview
except Exception:
    webview = None

# 优先识别的两个项目根目录
CANDIDATE_ROOTS = [
    os.path.expandvars(r"C:\Users\ysn\Desktop\zzjt"),
    r"D:\Desktop\自制截图",
]
# 当文件名以 .txt 结尾且去掉 .txt 后仍是代码后缀时，还原真实后缀
CODE_EXTS = (".tsx", ".ts", ".rs", ".css", ".js", ".jsx", ".json", ".html", ".py", ".md", ".toml")
IGNORE_DIRS = set(["node_modules", ".git", "dist", "build", "out", "target", ".next", ".cache", ".ysn_backup", "__pycache__", ".vscode", ".idea"])
STATE_FILE = os.path.join(os.path.expanduser("~"), ".ysn_deploy_gui.json")
HTML_PATH = os.path.join(os.path.dirname(os.path.abspath(__file__)), "ui.html")

# 内嵌的构建目标（不再依赖 .bat），均在 根目录/tauri-client 下执行
BUILD_TARGETS = {
    "dev": dict(title="日常自测 · 开发模式", desc="启动 Tauri 开发模式，热重载调试（会持续运行）", args=["run", "tauri", "dev"], out=""),
    "exe": dict(title="构建 EXE · 不打安装包", desc="只编译可执行文件，速度最快，构建成功后自动启动", args=["run", "tauri", "build", "--", "--no-bundle"], out="tauri-client\\src-tauri\\target\\release", launch=True),
    "msi": dict(title="构建 MSI 安装包", desc="生成 Windows MSI 安装包", args=["run", "tauri", "build", "--", "--bundles", "msi"], out="tauri-client\\src-tauri\\target\\release\\bundle\\msi"),
    "nsis": dict(title="构建 NSIS 安装包", desc="生成 NSIS (.exe) 安装包", args=["run", "tauri", "build", "--", "--bundles", "nsis"], out="tauri-client\\src-tauri\\target\\release\\bundle\\nsis"),
    "full": dict(title="完整发布 · 双安装包", desc="同时生成 MSI 与 NSIS 两种安装包", args=["run", "tauri", "build", "--", "--bundles", "msi,nsis"], out="tauri-client\\src-tauri\\target\\release\\bundle"),
}
BUILD_ORDER = ["dev", "exe", "msi", "nsis", "full"]


def normalize_name(path):
    """Foo.tsx.txt -> Foo.tsx；真正的 notes.txt 保持不变。"""
    base = os.path.basename(path)
    if base.lower().endswith(".txt"):
        stem = base[:-4]
        slow = stem.lower()
        for ext in CODE_EXTS:
            if slow.endswith(ext):
                return stem
    return base


def build_index(root):
    """遍历项目根目录，跳过 node_modules 等，建立 小写文件名 -> [绝对路径] 索引。"""
    index = {}
    if not root or not os.path.isdir(root):
        return index
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [d for d in dirnames if d not in IGNORE_DIRS]
        for fn in filenames:
            index.setdefault(fn.lower(), []).append(os.path.join(dirpath, fn))
    return index


class StagedItem:
    def __init__(self, item_id, source, name):
        self.id = item_id
        self.source = source
        self.name = name
        self.matches = []
        self.action = "unfound"
        self.target = None


class App:
    def __init__(self):
        self._lock = threading.Lock()
        self._buf = []
        self._code = None
        self._proc = None
        self._build_running = False
        self._cancel_build = False
        self._window = None
        self.staged = {}
        self._next_id = 1
        self._stage_dir = tempfile.mkdtemp(prefix="ysn_stage_")
        self.root = self._initial_root()
        self.index = build_index(self.root)

    # ---------- 根目录 ----------
    def _initial_root(self):
        saved = None
        try:
            if os.path.isfile(STATE_FILE):
                with open(STATE_FILE, "r", encoding="utf-8") as f:
                    saved = json.load(f).get("root")
        except Exception:
            saved = None
        if saved and os.path.isdir(saved):
            return saved
        for c in CANDIDATE_ROOTS:
            if os.path.isdir(c):
                return c
        return ""

    def _save_root(self):
        try:
            with open(STATE_FILE, "w", encoding="utf-8") as f:
                json.dump(dict(root=self.root), f, ensure_ascii=False)
        except Exception:
            pass

    def _rel(self, p):
        if not p:
            return p
        try:
            ap = os.path.abspath(p)
            ar = os.path.abspath(self.root) if self.root else None
            if ar and os.path.commonpath([ap, ar]) == ar:
                return os.path.relpath(ap, ar)
        except Exception:
            pass
        return p

    # ---------- 状态 ----------
    def get_state(self):
        return dict(
            root=self.root,
            candidates=list(CANDIDATE_ROOTS),
            detected=[c for c in CANDIDATE_ROOTS if os.path.isdir(c)],
            client_ok=bool(self.root) and os.path.isfile(os.path.join(self.root, "tauri-client", "package.json")),
            items=[self._item_json(it) for it in self.staged.values()],
            builds=[dict(key=k, title=BUILD_TARGETS[k]["title"], desc=BUILD_TARGETS[k]["desc"]) for k in BUILD_ORDER],
        )

    def _item_json(self, it):
        return dict(
            id=it.id,
            name=it.name,
            action=it.action,
            target=self._rel(it.target) if it.target else None,
            matches=[self._rel(m) for m in it.matches],
            matches_abs=list(it.matches),
        )

    def set_root(self, path):
        if not path or not os.path.isdir(path):
            return dict(ok=False, msg="目录不存在")
        self.root = path
        self.index = build_index(path)
        self._save_root()
        for it in self.staged.values():
            self._rematch(it)
        return dict(ok=True, state=self.get_state())

    def browse_root(self):
        if not self._window:
            return dict(ok=False, msg="窗口未就绪")
        res = self._window.create_file_dialog(webview.FOLDER_DIALOG)
        if res:
            return self.set_root(res[0])
        return dict(ok=False, msg="未选择")

    # ---------- 文件暂存 ----------
    def _add_source(self, src_path, display_name):
        it = StagedItem(self._next_id, src_path, normalize_name(display_name))
        self._next_id += 1
        self._rematch(it)
        self.staged[it.id] = it
        return it

    def _rematch(self, it):
        matches = self.index.get(it.name.lower(), []) if self.index else []
        it.matches = list(matches)
        if it.action == "skip":
            return
        if len(matches) == 1:
            it.action = "replace"
            it.target = matches[0]
        elif len(matches) > 1:
            it.action = "choose"
            it.target = None
        else:
            it.action = "unfound"
            it.target = None

    def add_files_via_dialog(self):
        if not self._window:
            return self.get_state()
        ftypes = ("代码文件 (*.tsx;*.ts;*.rs;*.css;*.js;*.jsx;*.json;*.html;*.txt;*.toml;*.md)", "所有文件 (*.*)")
        res = self._window.create_file_dialog(webview.OPEN_DIALOG, allow_multiple=True, file_types=ftypes)
        if res:
            for p in res:
                self._add_source(p, os.path.basename(p))
        return self.get_state()

    def add_dropped(self, files):
        for f in files or []:
            try:
                data = base64.b64decode(f.get("b64", ""))
                name = f.get("name", "file")
                safe = str(self._next_id) + "_" + os.path.basename(name)
                tmp = os.path.join(self._stage_dir, safe)
                with open(tmp, "wb") as out:
                    out.write(data)
                self._add_source(tmp, name)
            except Exception:
                pass
        return self.get_state()

    def pick_match(self, item_id, abspath):
        it = self.staged.get(int(item_id))
        if it:
            it.target = abspath
            it.action = "replace"
        return self.get_state()

    def choose_target_dir(self, item_id):
        it = self.staged.get(int(item_id))
        if not it or not self._window:
            return self.get_state()
        res = self._window.create_file_dialog(webview.FOLDER_DIALOG)
        if res:
            tgt = os.path.join(res[0], it.name)
            it.target = tgt
            it.action = "replace" if os.path.exists(tgt) else "new"
        return self.get_state()

    def skip_item(self, item_id):
        it = self.staged.get(int(item_id))
        if it:
            it.action = "skip"
            it.target = None
        return self.get_state()

    def remove_item(self, item_id):
        self.staged.pop(int(item_id), None)
        return self.get_state()

    def clear_staged(self):
        self.staged = {}
        return self.get_state()

    def apply_all(self):
        results = []
        applied_ids = []
        stamp = datetime.datetime.now().strftime("%Y%m%d_%H%M%S")
        backup_root = os.path.join(self.root, ".ysn_backup", stamp)
        did_backup = False
        for it in list(self.staged.values()):
            if it.action in ("skip", "unfound", "choose") or not it.target:
                results.append(dict(name=it.name, ok=False, msg="跳过（未指定目标）"))
                continue
            try:
                tgt = it.target
                if os.path.exists(tgt):
                    rel = self._rel(tgt)
                    if os.path.isabs(rel):
                        bpath = os.path.join(backup_root, os.path.basename(tgt))
                    else:
                        bpath = os.path.join(backup_root, rel)
                    os.makedirs(os.path.dirname(bpath), exist_ok=True)
                    shutil.copy2(tgt, bpath)
                    did_backup = True
                    msg = "已替换（已备份原文件）"
                else:
                    os.makedirs(os.path.dirname(tgt), exist_ok=True)
                    msg = "已新增"
                shutil.copy2(it.source, tgt)
                results.append(dict(name=it.name, ok=True, msg=msg, target=self._rel(tgt)))
                applied_ids.append(it.id)
            except Exception as e:
                results.append(dict(name=it.name, ok=False, msg="失败：" + str(e)))
        for i in applied_ids:
            self.staged.pop(i, None)
        self.index = build_index(self.root)
        return dict(results=results, backup=(self._rel(backup_root) if did_backup else None), state=self.get_state())

    # ---------- 构建 ----------
    def _log(self, line):
        with self._lock:
            self._buf.append(line)

    def start_build(self, key):
        with self._lock:
            running = self._build_running or (self._proc is not None and self._proc.poll() is None)
        if running:
            return dict(ok=False, msg="已有构建在运行，请先停止")
        t = BUILD_TARGETS.get(key)
        if not t:
            return dict(ok=False, msg="未知的构建类型")
        if not self.root or not os.path.isdir(self.root):
            return dict(ok=False, msg="请先选择项目根目录")
        client = os.path.join(self.root, "tauri-client")
        if not os.path.isfile(os.path.join(client, "package.json")):
            return dict(ok=False, msg="未找到 tauri-client\\package.json，请确认项目根目录")
        with self._lock:
            self._buf = []
            self._code = None
            self._proc = None
            self._build_running = True
            self._cancel_build = False
        th = threading.Thread(target=self._run_build, args=(key, t, client), daemon=True)
        th.start()
        return dict(ok=True)

    def _is_build_cancelled(self):
        with self._lock:
            return self._cancel_build

    def _run_build(self, key, t, client):
        code = -1
        try:
            self._log("【开始】" + t["title"])
            self._log("【目录】" + client)
            if key != "dev":
                self._close_running_app_before_build()
            if self._is_build_cancelled():
                self._log("【已停止】用户在准备阶段中断了构建")
                return
            self._log("【执行】npm " + " ".join(t["args"]))
            self._log("────────────────────────")
            try:
                if os.name == "nt":
                    full = ["cmd", "/c", "npm"] + t["args"]
                else:
                    full = ["npm"] + t["args"]
                proc = subprocess.Popen(
                    full, cwd=client, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                    text=True, encoding="utf-8", errors="replace", bufsize=1,
                )
                with self._lock:
                    self._proc = proc
                for line in proc.stdout:
                    self._log(line.rstrip("\n"))
                proc.wait()
                code = proc.returncode
            except Exception as e:
                self._log("【异常】" + str(e))
                code = -1
            self._log("────────────────────────")
            if self._is_build_cancelled():
                self._log("【已停止】构建进程已退出，退出码 " + str(code))
            elif code == 0:
                self._log("【完成】" + t["title"] + " 成功")
                out_abs = os.path.join(self.root, t["out"]) if t.get("out") else None
                if out_abs:
                    self._log("【产物】" + out_abs)
                if t.get("launch") and out_abs:
                    self._launch_artifact(out_abs)
            else:
                self._log("【失败】退出码 " + str(code))
        finally:
            with self._lock:
                self._code = code
                self._build_running = False
                self._proc = None

    def _close_running_app_before_build(self):
        self._log("【准备】尝试关闭正在运行的 YsnTrans.exe …")
        if os.name != "nt":
            self._log("【准备】非 Windows 环境，跳过关闭 YsnTrans.exe")
            return
        try:
            result = subprocess.run(
                ["taskkill", "/F", "/T", "/IM", "YsnTrans.exe"],
                capture_output=True,
                text=True,
                errors="replace",
                timeout=5,
            )
        except subprocess.TimeoutExpired:
            self._log("【准备警告】关闭 YsnTrans.exe 超时，继续构建")
            return
        except Exception as e:
            self._log("【准备警告】关闭 YsnTrans.exe 失败，继续构建：" + str(e))
            return

        output = "\n".join(
            part.strip() for part in (result.stdout, result.stderr) if part and part.strip()
        )
        if result.returncode == 0:
            self._log("【准备】已关闭正在运行的 YsnTrans.exe")
            return

        lower_output = output.lower()
        if (
            "not found" in lower_output
            or "no instance" in lower_output
            or "没有找到" in output
            or "找不到" in output
        ):
            self._log("【准备】未发现正在运行的 YsnTrans.exe，继续构建")
            return

        self._log("【准备警告】关闭 YsnTrans.exe 返回码 " + str(result.returncode) + "，继续构建")
        if output:
            self._log(output)

    def _launch_artifact(self, out_abs):
        """构建成功后从产物目录定位并启动 EXE（detached，不阻塞 GUI）。
        优先从 tauri.conf.json 读 productName，避免硬编码应用名。"""
        self._log("【启动】正在启动构建产物 …")
        if not out_abs or not os.path.isdir(out_abs):
            self._log("【启动失败】找不到产物目录：" + str(out_abs))
            return
        exe_name = "YsnTrans.exe"
        conf = os.path.join(self.root, "tauri-client", "src-tauri", "tauri.conf.json")
        try:
            if os.path.isfile(conf):
                with open(conf, "r", encoding="utf-8") as f:
                    data = json.load(f)
                product = data.get("productName")
                if product:
                    exe_name = product + ".exe"
        except Exception as e:
            self._log("【启动警告】读取 tauri.conf.json 失败，使用默认名：" + str(e))
        exe_path = os.path.join(out_abs, exe_name)
        if not os.path.isfile(exe_path):
            self._log("【启动失败】找不到 EXE：" + exe_path)
            return
        try:
            if os.name == "nt":
                DETACHED_PROCESS = 0x00000008
                CREATE_NEW_PROCESS_GROUP = 0x00000200
                CREATE_BREAKAWAY_FROM_JOB = 0x01000000
                subprocess.Popen(
                    ["cmd", "/c", "start", "", exe_path],
                    cwd=out_abs,
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL,
                    creationflags=DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_BREAKAWAY_FROM_JOB,
                    close_fds=True,
                )
            else:
                subprocess.Popen([exe_path], cwd=out_abs, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            self._log("【已启动】" + exe_path)
        except Exception as e:
            self._log("【启动失败】" + str(e))

    def poll_build(self, since=0):
        with self._lock:
            try:
                since = int(since)
            except Exception:
                since = 0
            lines = self._buf[since:]
            total = len(self._buf)
            running = self._build_running or (self._proc is not None and self._proc.poll() is None)
            code = self._code
        return dict(lines=lines, total=total, running=running, code=code)

    def stop_build(self):
        with self._lock:
            proc = self._proc
            running = self._build_running or (proc is not None and proc.poll() is None)
            if running:
                self._cancel_build = True
        if running:
            try:
                if proc is None or proc.poll() is not None:
                    self._log("【停止】构建正在准备中，已请求停止")
                elif os.name == "nt":
                    subprocess.run(
                        ["taskkill", "/F", "/T", "/PID", str(proc.pid)],
                        capture_output=True,
                        timeout=5,
                    )
                else:
                    proc.terminate()
            except subprocess.TimeoutExpired:
                self._log("【停止警告】终止构建进程超时，进程可能仍在退出")
            except Exception as e:
                self._log("【停止警告】终止构建进程失败：" + str(e))
            self._log("【已停止】用户中断了构建")
        return dict(ok=True)


def find_free_port():
    import socket
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(('', 0))
        return s.getsockname()[1]


def main():
    if webview is None:
        print("缺少 pywebview，请先运行： pip install pywebview")
        sys.exit(1)
    if not os.path.isfile(HTML_PATH):
        print("找不到 ui.html，请确保它与 app.py 在同一文件夹。")
        sys.exit(1)
    api = App()
    win = webview.create_window("YSN 部署助手", url=HTML_PATH, js_api=api, width=1100, height=740, min_size=(920, 620))
    api._window = win
    
    # 动态获取空闲端口，避免与本地其他服务（如 AlibabaProtect 等）端口冲突
    port = find_free_port()
    webview.start(http_server=True, http_port=port)


if __name__ == "__main__":
    main()
