"""Exercise the installed WebView2 UI and its native file dialogs on a Windows desktop."""
import argparse
import ctypes
from ctypes import wintypes
import json
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import time
import urllib.request

import win32con
import win32clipboard
import win32gui
import win32process
from playwright.sync_api import expect, sync_playwright


def edit_text(hwnd):
    # GetWindowText cannot fetch another process's edit-control contents.
    send = ctypes.windll.user32.SendMessageW
    send.argtypes = (wintypes.HWND, wintypes.UINT, wintypes.WPARAM, wintypes.LPARAM)
    send.restype = wintypes.LPARAM
    text = ctypes.create_unicode_buffer(32768)
    send(hwnd, win32con.WM_GETTEXT, len(text), ctypes.addressof(text))
    return text.value


def wait_endpoint(url):
    for _ in range(120):
        try:
            with urllib.request.urlopen(url + "/json/version", timeout=.5):
                return
        except OSError:
            time.sleep(.2)
    raise RuntimeError("WebView2 debugging endpoint did not start")


def dialog_path(pid, path):
    """Fill only a file dialog owned by the application under test."""
    deadline = time.monotonic() + 12
    while time.monotonic() < deadline:
        dialogs = []
        def inspect(hwnd, _):
            if (win32gui.IsWindowVisible(hwnd) and win32gui.GetClassName(hwnd) == "#32770"
                    and win32process.GetWindowThreadProcessId(hwnd)[1] == pid):
                dialogs.append(hwnd)
        win32gui.EnumWindows(inspect, None)
        for hwnd in dialogs:
            edits = []
            def inspect_child(child, _):
                if win32gui.GetClassName(child) == "Edit" and win32gui.IsWindowVisible(child):
                    edits.append(child)
            win32gui.EnumChildWindows(hwnd, inspect_child, None)
            confirm = win32gui.GetDlgItem(hwnd, 1)
            if not edits or not confirm:
                continue
            def filename_edit(edit):
                if edit_text(edit).startswith("EnvLens-"):
                    return True
                parent = win32gui.GetParent(edit)
                while parent and parent != hwnd:
                    if win32gui.GetDlgCtrlID(parent) == 1148:
                        return True
                    parent = win32gui.GetParent(parent)
                return False
            editor = next((e for e in edits if filename_edit(e)), edits[-1])
            # Character input delivers the change notifications expected by IFileDialog.
            win32gui.SendMessage(editor, win32con.EM_SETSEL, 0, -1)
            for letter in str(path):
                win32gui.SendMessage(editor, win32con.WM_CHAR, ord(letter), 0)
            assert edit_text(editor) == str(path), "File-name control did not accept the path"
            win32gui.PostMessage(confirm, win32con.BM_CLICK, 0, 0)
            for _ in range(35):
                time.sleep(.1)
                if not win32gui.IsWindow(hwnd) or not win32gui.IsWindowVisible(hwnd):
                    return
        time.sleep(.15)
    raise RuntimeError("Application file dialog did not accept its path")


def invoke(page, name, params=None):
    return page.evaluate("([cmd,args])=>window.__TAURI_INTERNALS__.invoke(cmd,args)", [name, params or {}])


def run(args, work):
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        port = sock.getsockname()[1]
    url = f"http://127.0.0.1:{port}"
    env = dict(os.environ, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=f"--remote-debugging-port={port}",
               WEBVIEW2_USER_DATA_FOLDER=str(work / "webview"))
    proc = subprocess.Popen([str(args.exe.resolve())], env=env)
    try:
        wait_endpoint(url)
        with sync_playwright() as runtime:
            browser = runtime.chromium.connect_over_cdp(url)
            page = browser.contexts[0].pages[0]
            page.set_default_timeout(15000)
            errors = []
            page.on("pageerror", lambda error: errors.append(str(error)))
            nav = lambda name: page.locator(f'[data-nav="{name}"]').click()
            nav("settings")
            version = json.loads((Path(__file__).resolve().parents[1] / "package.json").read_text(encoding="utf-8"))["version"]
            expect(page.get_by_text(version, exact=True)).to_be_visible()
            theme = page.get_by_role("radiogroup", name="外观主题")
            theme.get_by_role("radio", name="深色").focus()
            page.keyboard.press("ArrowRight")
            expect(theme.get_by_role("radio", name="浅色")).to_be_checked()
            expect(theme.get_by_role("radio", name="浅色")).to_be_focused()
            expect(page.locator(".app")).to_have_attribute("data-theme", "light")
            page.get_by_label("界面语言").select_option("en")
            expect(page.get_by_role("heading", name="Settings")).to_be_visible()
            if args.screens:
                args.screens.mkdir(parents=True, exist_ok=True)
                page.screenshot(path=str(args.screens / "settings-light.png"), scale="css")
            page.get_by_label("Language").select_option("zh")
            theme.get_by_role("radio", name="深色", exact=True).click()
            page.get_by_label("隐藏屏幕上的个人路径", exact=False).check()
            if args.gpu_python:
                page.get_by_label("单元素 GPU 验算", exact=False).check()
                nav("python")
                page.get_by_role("button", name="选择解释器文件").click()
                dialog_path(proc.pid, args.gpu_python.resolve())
                expect(page.locator(".candidate")).to_have_count(1)
            nav("overview")
            def scan():
                old = page.locator(".report-foot code").all_text_contents()
                page.get_by_test_id("scan").click()
                footer = page.locator(".report-foot code")
                expect(footer).to_be_visible(timeout=110000)
                if old:
                    expect(footer).not_to_have_text(old[0], timeout=110000)
                expect(page.get_by_test_id("scan")).to_be_enabled(timeout=110000)
                prefix = footer.inner_text()
                history = invoke(page, "list_history")
                created = next(r["id"] for r in history["reports"] if r["id"].startswith(prefix))
                print("Smoke report", created, flush=True)
            scan()
            expect(page.locator(".hardware").first).not_to_contain_text("—")
            if args.screens:
                args.screens.mkdir(parents=True, exist_ok=True)
                page.screenshot(path=str(args.screens / "overview.png"), scale="css")
            page.locator(".hardware.memory").click()
            expect(page.get_by_role("dialog")).to_contain_text("total_mib")
            page.keyboard.press("Escape")
            page.locator(".legend button").last.click()
            expect(page.get_by_role("radio", name="未检查", exact=True)).to_be_checked()
            nav("gpu")
            page.locator('[data-check="torch.cuda"]').click()
            expect(page.get_by_role("dialog")).to_be_visible()
            if args.gpu_python:
                expect(page.get_by_role("dialog")).to_contain_text("torch.cuda.is_available")
            if args.screens:
                page.screenshot(path=str(args.screens / "cuda-evidence.png"), scale="css")
            page.keyboard.press("Escape")
            nav("diagnostics")
            filters = page.get_by_role("radiogroup", name="筛选状态")
            filters.get_by_role("radio", name="未检查", exact=True).click()
            expect(filters.get_by_role("radio", name="未检查")).to_be_checked()
            expect(page.locator(".check-row .pill:not(.skipped)")).to_have_count(0)
            nav("overview")
            page.get_by_test_id("copy").click()
            expect(page.get_by_text("已复制到剪贴板", exact=True)).to_be_visible()
            win32clipboard.OpenClipboard()
            try:
                summary = win32clipboard.GetClipboardData(win32con.CF_UNICODETEXT)
                assert "EnvLens" in summary and "已脱敏" in summary
            finally:
                win32clipboard.CloseClipboard()
            for title, ext in (("Markdown", "md"), ("JSON", "json"), ("HTML", "html")):
                page.get_by_test_id("share").click()
                modal = page.get_by_role("dialog")
                modal.get_by_role("radio", name=title, exact=True).click()
                expect(modal.get_by_role("radio", name=title, exact=True)).to_be_checked()
                preview = modal.get_by_test_id("export-preview")
                expect(preview).not_to_have_text("正在生成预览")
                if title == "JSON":
                    full = modal.get_by_role("checkbox")
                    full.uncheck()
                    expect(preview).to_contain_text('"redacted": false')
                    full.check()
                    expect(preview).to_contain_text('"redacted": true')
                expected = preview.text_content()
                modal.get_by_role("button", name="保存文件").click()
                destination = work / ("report." + ext)
                dialog_path(proc.pid, destination)
                expect(modal).not_to_be_visible()
                assert destination.read_text(encoding="utf-8") == expected
            report = json.loads((work / "report.json").read_text(encoding="utf-8"))
            assert report["redacted"] and len(report["checks"]) == 18
            if args.gpu_python:
                assert next(c["status"] for c in report["checks"] if c["id"] == "torch.smoke") == "ok"
                assert str(args.gpu_python.resolve()) not in json.dumps(report, ensure_ascii=False)
            nav("python")
            empty = work / "empty-project"
            empty.mkdir()
            page.get_by_role("button", name="选择项目目录").click()
            dialog_path(proc.pid, empty)
            expect(page.get_by_text("该项目的约定位置中没有解释器", exact=False)).to_be_visible()
            scan()
            nav("overview")
            page.locator(".finding-row").first.click()
            expect(page.get_by_role("dialog")).to_contain_text("处理与复检")
            page.keyboard.press("Escape")
            nav("history")
            page.get_by_role("button", name="比较状态").first.click()
            expect(page.locator(".diff").first).to_be_visible()
            page.get_by_role("button", name="查看报告").first.click()
            expect(page.get_by_text("本地归档", exact=False).first).to_be_visible()
            # Exercise startup hydration independently of the live React state.
            page.reload()
            expect(page.locator(".report-foot")).to_contain_text("本地归档")
            for width, height in ((1024, 720), (800, 600)):
                page.set_viewport_size({"width": width, "height": height})
                expect(page.locator('[data-nav="diagnostics"]')).to_be_visible()
                assert page.evaluate("document.documentElement.scrollWidth <= window.innerWidth")
            page.emulate_media(reduced_motion="reduce")
            nav("overview")
            field = page.locator(".optical-field")
            field.hover()
            assert field.locator("img").evaluate("el => getComputedStyle(el).transform") == "none"
            assert page.locator(".nav-indicator").evaluate("el => getComputedStyle(el).transitionDuration") == "0s"
            denied = page.evaluate("""async()=>{try{await window.__TAURI_INTERNALS__.invoke('run_scan',
                {target:'unregistered-file',verifyGpu:false})}catch(e){return String(e)}}""")
            assert denied == "invalid_target", denied
            assert not errors, errors
            print("Desktop smoke passed: native target selection, scan, evidence, themes, locales, clipboard, "
                  "three exact file exports, redaction, project scoping, comparison, persistence and IPC validation.")
    finally:
        proc.terminate()
        proc.wait(timeout=15)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("exe", type=Path)
    parser.add_argument("--gpu-python", type=Path, help="Trusted interpreter with working CUDA PyTorch")
    parser.add_argument("--screens", type=Path, help="Save two screenshots of the real desktop")
    args = parser.parse_args()
    # Keep the user-data directory alive until WebView2 finishes shutting down.
    with tempfile.TemporaryDirectory(prefix="envlens-ui-", ignore_cleanup_errors=True) as tmp:
        run(args, Path(tmp))
