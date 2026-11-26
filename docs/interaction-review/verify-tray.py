"""Capture both isolated native menu states on the review host's light Windows menu."""

import argparse
import ctypes as ct
import subprocess
import time
from ctypes import wintypes as wt
from pathlib import Path

from PIL import ImageChops, ImageGrab

user32 = ct.windll.user32


def popup_for(pid):
    matches = []

    @ct.WINFUNCTYPE(wt.BOOL, wt.HWND, wt.LPARAM)
    def visit(hwnd, _):
        owner = wt.DWORD()
        user32.GetWindowThreadProcessId(hwnd, ct.byref(owner))
        if owner.value == pid and user32.IsWindowVisible(hwnd):
            name = ct.create_unicode_buffer(256)
            user32.GetClassNameW(hwnd, name, len(name))
            if name.value == "#32768":
                matches.append(hwnd)
        return True

    deadline = time.monotonic() + 15
    while time.monotonic() < deadline:
        matches.clear()
        user32.EnumWindows(visit, 0)
        if len(matches) == 1:
            return matches[0]
        time.sleep(0.01)
    raise RuntimeError("The isolated process did not show a unique Windows menu")


def finished_painting(image):
    luminance = image.convert("L")
    for top, bottom in [(0, 0.21), (0.27, 0.49), (0.51, 0.72), (0.79, 0.99)]:
        # Inspect the label column, excluding check marks and the menu border.
        box = (int(image.width * 0.13), int(image.height * top),
               int(image.width * 0.85), int(image.height * bottom))
        if sum(luminance.crop(box).histogram()[:145]) <= 100:
            return False
    margins = [(8, 24), (8, 80), (8, image.height - 15),
               (image.width - 12, image.height - 15)]
    return all(min(image.getpixel(point)) >= 246 for point in margins)


def capture(executable, folder, state):
    command = [str(executable.resolve()), "--ui-smoke", "--review-tray"]
    if state == "paused":
        command.append("--paused")
    process = subprocess.Popen(command, creationflags=subprocess.CREATE_NO_WINDOW)
    try:
        hwnd = popup_for(process.pid)
        last = None
        for _ in range(100):
            if not user32.IsWindow(hwnd):
                break
            image = ImageGrab.grab(window=hwnd).convert("RGB")
            if finished_painting(image) and last is not None:
                if ImageChops.difference(last, image).getbbox() is None:
                    dest = folder / f"tray-{state}.webp"
                    image.save(dest, lossless=True)
                    print(dest, image.size)
                    return
            last = image
            time.sleep(0.025)
        raise RuntimeError("Windows did not finish painting all four commands")
    finally:
        process.terminate()
        process.wait(timeout=5)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--exe", type=Path, default=Path("target/debug/mushak.exe"))
    parser.add_argument("--output", type=Path, default=Path("target/tray-review"))
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    user32.SetProcessDpiAwarenessContext(ct.c_void_p(-4))
    for state in ["active", "paused"]:
        for attempt in range(8):
            try:
                capture(args.exe, args.output, state)
                break
            except (OSError, RuntimeError):
                if attempt == 7:
                    raise
                time.sleep(0.2)


if __name__ == "__main__":
    main()
