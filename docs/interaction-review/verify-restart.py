"""Run from the worktree root with Python 3.11+ and Pillow on Windows."""

import argparse
import copy
import csv
import ctypes as c
import json
import os
import subprocess
import sys
import tempfile
import time
import tomllib
from ctypes import wintypes as w
from pathlib import Path

from PIL import Image, ImageChops, ImageGrab, ImageStat

u = c.windll.user32
u.SetProcessDpiAwarenessContext(c.c_void_p(-4))

parser = argparse.ArgumentParser(
    description="Exercise a standalone Mushak Settings process on Windows."
)
parser.add_argument(
    "bounds", type=Path, help="Dark/bounds.csv from the matching GPU gallery"
)
parser.add_argument("--exe", type=Path, default=Path("target/release/mushak.exe"))
parser.add_argument("--output", type=Path, default=Path("target/settings-restart"))
parser.add_argument(
    "--check-window-pick",
    action="store_true",
    help="Capture a temporary test window into the sample application profile",
)
parser.add_argument(
    "--cycles-only",
    action="store_true",
    help="Capture and check all six tab switches, then exit",
)
parser.add_argument(
    "--compact-sections",
    action="store_true",
    help="Click through the available Settings sections at 720 by 540 logical pixels",
)
parser.add_argument(
    "--follow-foreground",
    action="store_true",
    help="If focus moves during window picking, verify the actual foreground executable",
)
parser.add_argument("--resume-only", action="store_true", help="Check Resume using a private Windows event and both old and current resident status")
parser.add_argument("--long-profile", action="store_true", help="Exercise compact profile titles with a third application profile")
args = parser.parse_args()
if args.long_profile and not args.compact_sections:
    parser.error("--long-profile requires --compact-sections")
out = args.output
out.mkdir(parents=True, exist_ok=True)
scratch = tempfile.TemporaryDirectory(prefix="mushak-settings-")
db = Path(scratch.name) / "Mushak"
db.mkdir()
sample = Path(__file__).with_name("sample-config.toml").read_text(encoding="utf-8")
if args.long_profile:
    sample = sample.replace('name = "Photo editor"', 'name = "Photo editing and color correction"')
    sample += '\n[[profiles]]\nname = "Audio tools"\nmatch_processes = ["audio.exe"]\n'
(db / "config.toml").write_text(sample, encoding="utf-8")
fixture = json.loads(Path("docs/validation/mx-master-2s-bluetooth.json").read_text())[
    "status"
]
(db / "status.json").write_text(json.dumps(dict(paused=False, device=fixture, active_profile="Preview app")))
theme = args.bounds.parent.name.lower()
assert theme in ("light", "dark")
(db / "ui.toml").write_text(f'theme = "{theme}"\naccent = "marigold"')


def config():
    return tomllib.loads((db / "config.toml").read_text())


source = config()
env = dict(os.environ, APPDATA=str(db.parent.resolve()))


proc = None
h = None


def launch():
    global proc, h, scale
    h = None
    proc = subprocess.Popen(
        [str(args.exe.resolve()), "--settings"],
        env=env,
        creationflags=subprocess.CREATE_NO_WINDOW,
    )
    handles = []

    @c.WINFUNCTYPE(w.BOOL, w.HWND, w.LPARAM)
    def enum(h, _):
        pid = w.DWORD()
        u.GetWindowThreadProcessId(h, c.byref(pid))
        if pid.value == proc.pid and u.IsWindowVisible(h):
            title = c.create_unicode_buffer(128)
            u.GetWindowTextW(h, title, 128)
            if title.value == "Mushak Settings":
                handles.append(h)
        return True

    for i in range(100):
        u.EnumWindows(enum, 0)
        if handles:
            break
        time.sleep(0.1)
    assert handles, "Settings did not open in the child process"
    h = handles[0]
    time.sleep(1)
    scale = u.GetDpiForWindow(h) / 96
    # These checks deliver input to one HWND and capture its client directly.
    # Leave other apps above it so physical input cannot land on the test fixture.
    u.ShowWindow(h, 8)
    assert u.SetWindowPos(h, 1, 20, 20, 0, 0, 0x1 | 0x10)
    # The HWND exists before the renderer has applied the theme and frameless layout.
    until = time.monotonic() + 8
    while time.monotonic() < until:
        r = w.RECT()
        u.GetClientRect(h, c.byref(r))
        if (r.right, r.bottom) == (round(968 * scale), round(668 * scale)) and matches_capture(
            "seq-06-cancel", (12, 5, 126, 39)
        ):
            break
        time.sleep(0.1)
    else:
        raise AssertionError("Settings did not reach the expected size and theme")


def stop():
    if h:
        u.PostMessageW(h, 0x10, 0, 0)
    try:
        proc.wait(timeout=2)
    except subprocess.TimeoutExpired:
        proc.terminate()
        proc.wait()


def resize(width, height):
    client, window = w.RECT(), w.RECT()
    u.GetClientRect(h, c.byref(client))
    u.GetWindowRect(h, c.byref(window))
    dx = window.right - window.left - client.right
    dy = window.bottom - window.top - client.bottom
    assert u.SetWindowPos(
        h, 0, 0, 0, round(width * scale) + dx, round(height * scale) + dy, 0x2 | 0x4
    )
    time.sleep(0.25)


def client_pixels():
    r = w.RECT()
    u.GetClientRect(h, c.byref(r))
    p = w.POINT()
    u.ClientToScreen(h, c.byref(p))
    wr = w.RECT()
    u.GetWindowRect(h, c.byref(wr))
    im = ImageGrab.grab(window=h)
    if im.size != (r.right, r.bottom):
        x, y = p.x - wr.left, p.y - wr.top
        im = im.crop((x, y, x + r.right, y + r.bottom))
    return im.resize((round(r.right / scale), round(r.bottom / scale))).convert("RGB")


def shot(name):
    client_pixels().save(out / (name + ".webp"), lossless=True)


def matches_capture(reference, rect):
    with Image.open(args.bounds.parent / (reference + ".webp")) as expected:
        actual = client_pixels().crop(rect)
        difference = ImageChops.difference(actual, expected.convert("RGB").crop(rect))
    return max(ImageStat.Stat(difference).mean) < 6.0


def click(x, y):
    loc = round(scale * x) | (round(scale * y) << 16)
    assert u.PostMessageW(h, 0x200, 0, loc)
    time.sleep(0.08)
    for msg, buttons_before, buttons_after, wait in [(0x201, 0, 1, 0.09), (0x202, 1, 0, 0.14)]:
        # Miniquad reads the last WM_MOUSEMOVE position when handling button messages.
        assert u.PostMessageW(h, 0x200, buttons_before, loc)
        assert u.PostMessageW(h, msg, buttons_after, loc)
        # Keep this test window's pointer at the target until its next render.
        # A real mouse move can otherwise replace the message coordinates between frames.
        until = time.monotonic() + wait
        while time.monotonic() < until:
            assert u.PostMessageW(h, 0x200, buttons_after, loc)
            time.sleep(0.01)


def key(vk, scan):
    for msg, lp in [(0x100, 1 | (scan << 16)), (0x101, 0xC0000001 | (scan << 16))]:
        assert u.PostMessageW(h, msg, vk, lp)
        time.sleep(0.15)


with args.bounds.open(encoding="utf-8") as handle:
    bounds = list(csv.DictReader(handle))


def control(id, frame):
    b = next(b for b in bounds if b["control"] == id and b["frame"] == frame)
    click(float(b["x"]) + float(b["w"]) / 2, float(b["y"]) + float(b["h"]) / 2)


def check_action_selection():
    coordinates = args.bounds.with_name("action-controls.csv")
    if not coordinates.is_file():
        return None
    with coordinates.open() as stream:
        centers = {row["id"]: (float(row["x"]), float(row["y"])) for row in csv.DictReader(stream)}
    def open_back():
        control("mouse_back", "seq-06-cancel")
    open_back()
    click(*centers["copy"])
    assert config() == source, "Selecting Copy wrote an unsaved mapping"
    click(*centers["cancel"])
    assert config() == source, "Cancel changed the mapping"
    open_back()
    click(*centers["copy"])
    click(*centers["save"])
    time.sleep(0.7)
    expected = copy.deepcopy(source)
    expected["profiles"][0]["buttons"]["back"] = {"kind": "Key", "mods": ["Ctrl"], "vk": 67}
    assert config() == expected, "Save action did not persist only Back = Copy"
    open_back()
    click(*centers["common"])
    click(*centers["factory"])
    click(*centers["save"])
    time.sleep(0.7)
    assert config() == source, "Restoring the native action changed the sample"
    return True


def nav_y(section, fallback):
    coordinates = args.bounds.with_name("navigation.csv")
    if coordinates.exists():
        with coordinates.open() as stream:
            return round(float(next(row["y"] for row in csv.DictReader(stream) if row["section"] == section)))
    return fallback


def profile():
    box = next(
        (
            b
            for b in bounds
            if b["control"] == "profile_btn" and b["frame"] == "seq-06-cancel"
        ),
        None,
    )
    if box:
        x, y, w, height = (float(box[k]) for k in ("x", "y", "w", "h"))
        label = tuple(round(v) for v in (x + 53, y + 6, x + w - 24, y + height - 6))
    else:
        x, y, w, height = 771, 87, 168, 38
        label = (778, 93, 903, 116)
    for _ in range(3):
        if matches_capture("seq-06-cancel", label):
            return
        click(x + w / 2, y + height / 2)
        option_path = args.bounds.with_name("profile-option.csv")
        if option_path.exists():
            with option_path.open() as stream:
                option = next(csv.DictReader(stream))
            click(float(option["x"]), float(option["y"]))
        else:
            offset = (
                101 if args.bounds.parent.parent.name in ("baseline", "round-1") else 53
            )
            click(x + w / 2, y + offset)
        time.sleep(0.3)
    if not matches_capture("seq-06-cancel", label):
        client_pixels().save(Path("target") / f"unselected-profile-{theme}.png")
    assert matches_capture("seq-06-cancel", label), (
        "The application profile was not selected"
    )


def binding():
    return config()["profiles"][0]["buttons"]["back"]


def foreground_executable(pid):
    kernel = c.windll.kernel32
    kernel.OpenProcess.restype = w.HANDLE
    handle = kernel.OpenProcess(0x1000, False, pid)
    if not handle:
        return None
    try:
        path = c.create_unicode_buffer(32768)
        size = w.DWORD(len(path))
        if kernel.QueryFullProcessImageNameW(w.HANDLE(handle), 0, path, c.byref(size)):
            return Path(path.value).name.lower()
    finally:
        kernel.CloseHandle(w.HANDLE(handle))
    return None


def check_window_pick():
    import tkinter as tk

    click(918, 176)
    click(88, nav_y("Profiles", 208))
    assert matches_capture("profiles", (229, 68, 367, 102)), "Profiles did not open"
    click(434, 247)
    helper = tk.Tk()
    helper.title("Mushak test window")
    helper.geometry("340x72+72+72")
    tk.Label(helper, text="Mushak is testing foreground application selection.").pack(
        pady=14
    )
    helper.attributes("-topmost", True)
    previous_cursor = w.POINT()
    u.GetCursorPos(c.byref(previous_cursor))
    point = None
    try:
        helper.update()
        helper.focus_force()
        helper_hwnd = helper.winfo_id()
        u.GetAncestor.restype = w.HWND
        root_hwnd = u.GetAncestor(helper_hwnd, 2)
        u.SetForegroundWindow(root_hwnd)
        client = w.RECT()
        assert u.GetClientRect(helper_hwnd, c.byref(client))
        point = w.POINT(client.right // 2, client.bottom // 2)
        assert u.ClientToScreen(helper_hwnd, c.byref(point))
        u.WindowFromPoint.argtypes = [w.POINT]
        u.WindowFromPoint.restype = w.HWND
        owner = w.DWORD()
        u.GetWindowThreadProcessId(u.WindowFromPoint(point), c.byref(owner))
        assert owner.value == os.getpid(), "Test window covered by another process"
        assert u.SetCursorPos(point.x, point.y)
        u.mouse_event(2, 0, 0, 0, 0)
        u.mouse_event(4, 0, 0, 0, 0)
        helper.update()
        previous = None
        deadline = time.monotonic() + 8
        while time.monotonic() < deadline:
            helper.update()
            pid = w.DWORD()
            foreground = u.GetForegroundWindow()
            u.GetWindowThreadProcessId(foreground, c.byref(pid))
            if pid.value != os.getpid() and not args.follow_foreground:
                raise AssertionError("The test window lost foreground ownership")
            current = foreground_executable(pid.value) if pid.value else None
            changed = config()
            added = set(changed["profiles"][0]["match_processes"]) - set(
                expected["profiles"][0]["match_processes"]
            )
            if added:
                assert len(added) == 1
                exe = added.pop().lower()
                assert exe in (previous, current), "Saved app differs from observed foreground"
                break
            previous = current
            time.sleep(0.05)
        else:
            client_pixels().save(Path("target") / "window-pick-timeout.png")
            raise AssertionError(f"The foreground test window was not captured. Last executable: {previous!r}")
        wanted = copy.deepcopy(expected)
        matches = changed["profiles"][0]["match_processes"]
        wanted["profiles"][0]["match_processes"].append(
            next(v for v in matches if v.lower() == exe)
        )
        assert changed == wanted, "Window capture changed an unrelated setting"
    finally:
        cursor = w.POINT()
        u.GetCursorPos(c.byref(cursor))
        if point and (cursor.x, cursor.y) == (point.x, point.y):
            u.SetCursorPos(previous_cursor.x, previous_cursor.y)
        helper.destroy()
    time.sleep(0.3)
    shot("04-picked-window")
    print(f"Foreground selection added {exe} to Preview app.")
    img = client_pixels()
    chip = img.getpixel((390, 236))
    end = next(
        x for x in range(396, img.width - 1)
        if max(abs(a - b) for a, b in zip(img.getpixel((x, 236)), chip)) > 5
    )
    click(end - 13, 247)
    assert config() == expected, "Removing the captured executable changed another setting"
    click(882, 88)
    profiles = config()["profiles"]
    assert [p["name"] for p in profiles] == ["Preview app", "Photo editor", "New profile"]
    if args.bounds.with_name("profile-rename.webp").is_file():
        before_name = config()
        click(339, 465)
        for char in "Trial tools":
            assert u.PostMessageW(h, 0x102, ord(char), 1)
            time.sleep(0.04)
        click(883, 465)
        before_name["profiles"][-1]["name"] = "Trial tools"
        current = config()
        if current != before_name:
            client_pixels().save(Path("target/native-rename-result.png"))
            raise AssertionError(f"Unexpected configuration after rename; profile names: {[p['name'] for p in current['profiles']]}")
        shot("05-renamed-profile")
        print("Typed and saved Trial tools in the standalone Settings window.")
    click(880, 465)
    assert config() == expected, "Removing the new profile changed another setting"
    print("Added and removed a profile; the sample settings returned to their previous contents.")


def check_resume_command():
    coordinates = args.bounds.with_name("resume-button.csv")
    if not coordinates.exists():
        return False
    kernel = c.windll.kernel32
    kernel.CreateEventW.restype = w.HANDLE
    kernel.CreateEventW.argtypes = [c.c_void_p, w.BOOL, w.BOOL, w.LPCWSTR]
    kernel.WaitForSingleObject.argtypes = [w.HANDLE, w.DWORD]
    kernel.WaitForSingleObject.restype = w.DWORD
    kernel.CloseHandle.argtypes = [w.HANDLE]
    endpoint = kernel.CreateEventW(None, False, False, f"Local\\Mushak.ResumeRemapping.{os.getpid()}")
    assert endpoint
    status_path = db / "status.json"
    original = json.loads(status_path.read_text(encoding="utf-8"))
    before = config()

    def publish(status):
        temporary = status_path.with_suffix(".tmp")
        temporary.write_text(json.dumps(status), encoding="utf-8")
        temporary.replace(status_path)

    def wait_for_header(reference):
        deadline = time.monotonic() + 4
        while time.monotonic() < deadline:
            if matches_capture(reference, (240, 10, 375, 36)):
                return
            time.sleep(0.1)
        frame = client_pixels()
        frame.save(Path("target") / f"resume-observed-{theme}-{reference}.png")
        raise AssertionError("Settings did not reflect the resident's " + reference + " state")

    try:
        publish(dict(original, paused=True, resume_pid=os.getpid()))
        wait_for_header("paused-dismissed")
        shot("06-paused-settings")
        with coordinates.open() as stream:
            row = next(csv.DictReader(stream))
        click(float(row["x"]), float(row["y"]))
        assert kernel.WaitForSingleObject(endpoint, 2500) == 0, "Settings did not send Resume"
        wait_for_header("paused-dismissed")
        publish(dict(original, paused=False, resume_pid=os.getpid()))
        wait_for_header("resumed")
        shot("07-resumed-settings")
        assert kernel.CloseHandle(endpoint)
        endpoint = None
        publish(dict(original, paused=True, resume_pid=os.getpid()))
        wait_for_header("paused-dismissed")
        click(float(row["x"]), float(row["y"]))
        has_recovery_help = args.bounds.with_name("resume-error.webp").exists()
        help_region = (50, 54, 700, 88)
        help_bounds = args.bounds.with_name("resume-help.csv")
        if help_bounds.exists():
            with help_bounds.open() as stream:
                box = next(csv.DictReader(stream))
            x, y = round(float(box["x"])), round(float(box["y"]))
            width, height = round(float(box["w"])), round(float(box["h"]))
            # The help page begins with a section-specific Back button. Compare its heading.
            offset = 40 if args.bounds.with_name("tray-help-button.csv").is_file() else 0
            help_region = (x + 4, y + offset + 2, x + min(width - 4, 220), y + min(height - 2, offset + 38))

        def wait_for_help(reference):
            deadline = time.monotonic() + 4
            while time.monotonic() < deadline:
                # The pointer can still be over Retry. Compare the status label beside it.
                if matches_capture(reference, help_region) and matches_capture(reference, (246, 11, 373, 35)):
                    return
                time.sleep(0.1)
            client_pixels().save(Path("target") / f"help-observed-{theme}-{reference}.png")
            raise AssertionError(f"Settings did not show {reference} instructions")
        if has_recovery_help:
            wait_for_help("resume-error")
        else:
            assert not matches_capture("paused-dismissed", (245, 10, 462, 36)), "A stale endpoint gave no feedback"
        checkpoint = client_pixels().crop((245, 10, 462, 36))
        shot("08-resume-failed")
        if has_recovery_help:
            endpoint = kernel.CreateEventW(None, False, False, f"Local\\Mushak.ResumeRemapping.{os.getpid()}")
            assert endpoint
            click(float(row["x"]), float(row["y"]))
            assert kernel.WaitForSingleObject(endpoint, 2500) == 0, "Retry did not reach the new event"
            publish(dict(original, paused=False, resume_pid=os.getpid()))
            wait_for_header("resumed")
            shot("08b-retry-confirmed")
        publish(dict(original, paused=True))
        tray_help = args.bounds.with_name("tray-help-button.csv")
        if tray_help.is_file():
            with tray_help.open() as stream:
                help_button = next(csv.DictReader(stream))
            wait_for_header("resume-legacy")
            click(float(help_button["x"]), float(help_button["y"]))
        if has_recovery_help:
            wait_for_help("resume-legacy")
        else:
            deadline = time.monotonic() + 4
            while time.monotonic() < deadline:
                if ImageChops.difference(checkpoint, client_pixels().crop((245, 10, 462, 36))).getbbox():
                    break
                time.sleep(0.1)
            else:
                raise AssertionError("Settings left Retry visible for a legacy resident")
        shot("09-tray-fallback")
        assert config() == before, "Resume changed a saved mapping or profile"
        return True
    finally:
        publish(original)
        if endpoint:
            assert kernel.CloseHandle(endpoint)


try:
    launch()
    if args.resume_only:
        assert check_resume_command()
        print("Verified Resume, stale endpoint and legacy status:", theme)
        sys.exit(0)
    if args.compact_sections:
        resize(720, 540)
        previous = None
        sections = [("Buttons", 85), ("Scroll", 127), ("Gestures", 169), ("Profiles", 211), ("Device", 253)]
        navigation = args.bounds.with_name("navigation.csv")
        if navigation.exists():
            with navigation.open() as stream:
                sections = [(row["section"], round(float(row["y"]))) for row in csv.DictReader(stream)]
        for section, y in sections:
            name = {"Scroll": "wheel", "Mouse controls": "buttons"}.get(section, section.lower())
            click(74, y)
            deadline = time.monotonic() + 5
            while time.monotonic() < deadline:
                frame = client_pixels()
                assert frame.size == (720, 540)
                heading = frame.crop((179, 63, 330, 108))
                red, _, blue = frame.getpixel((10, y))
                heading_changed = previous is None or max(
                    ImageStat.Stat(ImageChops.difference(heading, previous)).mean
                ) > 3
                if red - blue > 100 and heading_changed:
                    break
                time.sleep(0.05)
            else:
                frame.save(Path("target") / f"section-not-selected-{theme}-{name}.png")
                raise AssertionError(f"{name} did not show its heading and selected navigation row")
            frame.save(out / (name + ".webp"), lossless=True)
            previous = heading
        assert config() == source
        print(f"Visited {len(sections)} sections at 720 by 540 in {theme}")
        sys.exit(0)
    profile()
    action_saved_and_canceled = check_action_selection() if not args.cycles_only else None
    control("mouse_back", "seq-06-cancel")
    for cycle in range(1, 4):
        for tab, frame in [
            ("shortcut_tab", "seq-02-shortcut"),
            ("preset_tab", "seq-01-actions"),
        ]:
            control(tab, frame)
            if args.cycles_only:
                b = next(
                    r for r in bounds if r["control"] == tab and r["frame"] == frame
                )
                x, y, width, height = (float(b[k]) for k in ("x", "y", "w", "h"))
                rect = tuple(
                    round(v) for v in (x + 2, y + 2, x + width - 2, y + height - 2)
                )
                assert matches_capture(frame, rect), (
                    f"{tab} did not become selected on cycle {cycle}"
                )
                shot(f"{cycle}-{tab}")
    if args.cycles_only:
        assert config() == source
        print("Captured all six tab switches at", theme, scale)
        sys.exit(0)
    control("shortcut_tab", "seq-02-shortcut")
    control("capture", "seq-02-shortcut")
    key(0x4D, 0x32)
    shot("01-recorded-M")
    if args.bounds.with_name("draft-return.webp").is_file():
        key_rect = (537, 430, 553, 450)
        assert matches_capture("seq-04-draft", key_rect), "M was not visible after recording"
        before_switch = client_pixels().crop(key_rect)
        control("preset_tab", "seq-01-actions")
        assert matches_capture("seq-01-actions", (535, 205, 720, 237))
        control("shortcut_tab", "seq-02-shortcut")
        after_switch = client_pixels().crop(key_rect)
        assert ImageChops.difference(before_switch, after_switch).getbbox() is None, "Switching tabs lost the recorded M"
        assert config() == source, "Tab navigation committed the unsaved draft"
    resize(720, 360)
    shot("01b-resized-M")
    control("cancel_shortcut", "seq-05-resize")
    resize(968, 668)
    assert config() == source
    control("mouse_back", "seq-06-cancel")
    control("shortcut_tab", "seq-02-shortcut")
    control("capture", "seq-02-shortcut")
    key(0x4B, 0x25)
    shot("02-ready-K")
    resize(720, 360)
    control("assign", "seq-05-resize")
    assert binding() == {"kind": "Key", "mods": [], "vk": 75}, binding()
    expected = copy.deepcopy(source)
    expected["profiles"][0]["buttons"]["back"] = {"kind": "Key", "mods": [], "vk": 75}
    assert config() == expected, "Saving changed another profile or setting"
    first = proc.pid
    stop()
    launch()
    profile()
    control("mouse_back", "seq-06-cancel")
    shot("03-restarted-K")
    assert matches_capture("seq-09-reopen", (523, 252, 690, 288)), (
        "Reopened editor did not show the saved binding"
    )
    if args.check_window_pick:
        check_window_pick()
    resumed = check_resume_command()
    receipt = {
        "display_scale": scale,
        "resume_command_received_by_test_process": resumed,
        "process_ids": [first, proc.pid],
        "sample_profile": "Preview app",
        "cancel_preserved_sample_config": True,
        "action_selection_cancel_save_and_restore": action_saved_and_canceled,
        "save_changed_only": "profiles[0].buttons.back",
        "reopened_binding": binding(),
        "draft_m_survived_tab_switch": True if args.bounds.with_name("draft-return.webp").is_file() else None,
    }
    (out / "result.json").write_text(json.dumps(receipt, indent=2), encoding="utf-8")
    print(
        f"Native settings PIDs {first} then {proc.pid}, scale {scale}, saved key {binding()}."
    )
finally:
    if proc is not None and proc.poll() is None:
        stop()
    scratch.cleanup()
