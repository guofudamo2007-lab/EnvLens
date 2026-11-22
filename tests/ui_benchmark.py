"""Measure a real Windows desktop with one existing archived report. Run once per binary."""

import argparse
from contextlib import contextmanager
import json
import math
import os
from pathlib import Path
import socket
import statistics
import subprocess
import time
import urllib.request

import win32con
import win32gui
import win32process
from playwright.sync_api import TimeoutError as PlaywrightTimeout, sync_playwright

TIMINGS = """(() => {
  window.envlensTimings = {};
  const observer = new MutationObserver(() => {
    const mark = (name, selector) => {
      if (!(name in window.envlensTimings) && document.querySelector(selector)) {
        window.envlensTimings[name] = performance.now();
      }
    };
    mark('shell', 'h1');
    mark('report', '.report-foot code');
    mark('targets', '[data-testid=scan]:not(:disabled)');
    if (Object.keys(window.envlensTimings).length === 3) observer.disconnect();
  });
  observer.observe(document, {subtree:true, childList:true, attributes:true});
})()"""

NAVIGATION = """async () => {
  const paint = () => new Promise(done => requestAnimationFrame(() => requestAnimationFrame(done)));
  const samples = [];
  for (let cycle=0; cycle<5; cycle++) {
    for (const name of ['diagnostics','settings','gpu','overview','history','python']) {
      await paint();
      const start = performance.now();
      document.querySelector('[data-nav="'+name+'"]').click();
      await paint();
      if (!document.querySelector('[data-nav="'+name+'"][aria-current="page"]')) throw Error(name);
      samples.push(performance.now()-start);
    }
  }
  return samples;
}"""


def quantiles(samples):
    ordered = sorted(samples)
    return {
        "p50": round(statistics.median(ordered), 1),
        "p95": round(ordered[math.ceil(0.95 * len(ordered)) - 1], 1),
    }


@contextmanager
def desktop(runtime, exe, profile):
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        port = sock.getsockname()[1]
    url = f"http://127.0.0.1:{port}"
    env = dict(
        os.environ,
        WEBVIEW2_USER_DATA_FOLDER=str(profile),
        WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=f"--remote-debugging-address=127.0.0.1 --remote-debugging-port={port}",
    )
    start = time.perf_counter()
    proc = subprocess.Popen([str(exe)], env=env)
    try:
        while time.perf_counter() - start < 20:
            try:
                with urllib.request.urlopen(url + "/json/version", timeout=0.25):
                    break
            except OSError:
                time.sleep(0.02)
        else:
            raise RuntimeError("Desktop WebView debugging endpoint did not start")
        browser = runtime.chromium.connect_over_cdp(url)
        context = browser.contexts[0]
        # WebView2 can expose a temporary about:blank target before the app navigates.
        deadline = time.monotonic() + 15
        while time.monotonic() < deadline:
            selected = next(
                (
                    p
                    for p in context.pages
                    if not p.is_closed()
                    and p.url.startswith(
                        (
                            "http://tauri.localhost/",
                            "https://tauri.localhost/",
                            "tauri://localhost/",
                        )
                    )
                ),
                None,
            )
            if selected is not None:
                break
            try:
                context.wait_for_event("page", timeout=30)
            except PlaywrightTimeout:
                pass
        else:
            raise RuntimeError("Application target did not navigate")
        yield selected, start
    finally:
        # Close only this launcher's top-level window so its reusable browser profile can flush.
        def close_window(hwnd, _):
            if win32process.GetWindowThreadProcessId(hwnd)[
                1
            ] == proc.pid and win32gui.IsWindowVisible(hwnd):
                win32gui.PostMessage(hwnd, win32con.WM_CLOSE, 0, 0)

        win32gui.EnumWindows(close_window, None)
        try:
            proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            proc.terminate()
            proc.wait(timeout=10)
        for _ in range(80):
            try:
                with urllib.request.urlopen(url + "/json/version", timeout=0.1):
                    pass
            except OSError:
                break
            time.sleep(0.025)


def ready(page):
    page.locator(".report-foot code").wait_for(timeout=15000)
    page.locator("[data-testid=scan]:not(:disabled)").wait_for(timeout=20000)


def measure(exe, output):
    output.mkdir(parents=True, exist_ok=True)
    profile = output / "webview"
    launches = []
    loads = []
    with sync_playwright() as runtime:
        for n in range(6):
            with desktop(runtime, exe, profile) as (page, start):
                ready(page)
                if n:
                    launches.append((time.perf_counter() - start) * 1000)
                if n == 5:
                    page.set_viewport_size({"width": 1230, "height": 810})
                    page.add_init_script(TIMINGS)
                    for i in range(12):
                        page.reload(wait_until="domcontentloaded")
                        ready(page)
                        if i >= 2:
                            loads.append(page.evaluate("window.envlensTimings"))
                    nav = page.evaluate(NAVIGATION)
                    page.locator('[data-nav="overview"]').click()
                    page.wait_for_timeout(450)
                    idle = page.evaluate(
                        "document.getAnimations().filter(a=>a.playState==='running').length"
                    )
                    page.screenshot(path=str(output / "overview.png"), scale="css")
    result = {
        "launch_to_observed_ready_ms": quantiles(launches),
        "reload_ms": {
            key: quantiles([sample[key] for sample in loads])
            for key in ("shell", "report", "targets")
        },
        "navigation_two_frames_ms": quantiles(nav),
        "idle_running_animations": idle,
        "raw": {"launches": launches, "reloads": loads, "navigation": nav},
    }
    (output / "metrics.json").write_text(json.dumps(result, indent=2), encoding="utf-8")
    print(json.dumps({k: v for k, v in result.items() if k != "raw"}))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("exe", type=Path)
    parser.add_argument(
        "--output",
        required=True,
        type=Path,
        help="Folder for the isolated WebView profile, measurements and screenshot.",
    )
    args = parser.parse_args()
    measure(args.exe.resolve(strict=True), args.output.resolve())
