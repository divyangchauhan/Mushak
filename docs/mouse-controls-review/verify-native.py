"""Exercise the release Settings window with a temporary APPDATA and a recorded device."""
from pathlib import Path
import sys

runner = Path("docs/interaction-review/verify-restart.py").resolve()
__file__ = str(runner)
sys.argv = [__file__, "docs/interaction-review/round-42/Dark/bounds.csv",
            "--exe", "target/native/release/mushak.exe", "--output", "target/native-results"]
exec(compile(runner.read_text(encoding="utf8").split("with args.bounds.open")[0], str(runner), "exec"))

try:
    launch()
    first = proc.pid
    shot("01-overview")
    click(310, 340)
    shot("02-wheel")
    click(880, 299)
    click(845, 208)
    shot("03-middle-click")
    click(851, 273)
    shot("04-wheel-shortcut")
    click(625, 580)
    assert config() == source
    click(720, 205)
    click(880, 299)
    click(845, 208)
    click(796, 431)
    click(673, 208)
    click(865, 580)
    time.sleep(.5)
    expected = copy.deepcopy(source)
    expected["device"]["smartshift"] = "SmartShift"
    expected["default_profile"]["buttons"]["middle"] = {"kind": "Key", "mods":["Ctrl"], "vk":67}
    assert config() == expected, ("wheel save",config()["default_profile"]["buttons"]["middle"],config()["device"]["smartshift"])
    click(480, 252)
    for cycle in range(3):
        click(830, 225)
        shot(f"05-{cycle}-shortcut")
        click(628, 225)
        shot(f"06-{cycle}-actions")
    click(830, 225)
    click(847, 433)
    key(0x4D,0x32)
    shot("07-recorded-M")
    resize(720,360)
    shot("08-resized")
    click(72, 317)
    assert config() == expected
    resize(968,668)
    click(480,252)
    click(830,225)
    click(847,433)
    key(0x4B,0x25)
    shot("08b-save-K")
    click(865,581)
    time.sleep(.4)
    expected["default_profile"]["buttons"]["back"] = {"kind":"Key","mods":[],"vk":75}
    assert config() == expected, ("shortcut save",config()["default_profile"]["buttons"]["back"])
    stop()
    launch()
    assert first != proc.pid and config() == expected
    click(311,340)
    shot("09-reopened-wheel")
    def gold_count(rect):
        return sum(r>175 and r-g>20 and g-b>30 for r,g,b in client_pixels().crop(rect).getdata())
    assert gold_count((822,312,930,322)) > 50, "saved SmartShift highlight missing"
    click(915,170)
    click(480,252)
    shot("10-reopened-back")
    assert gold_count((730,233,927,246)) > 70,"saved shortcut tab did not reopen"
    receipt = {"processes":[first,proc.pid],"scale":scale,"wheel_mode":"SmartShift","middle_click":"Ctrl+C",
        "back_button":"K","tab_cycles":3,"canceled_shortcut":"M","config_exact_match":True}
    (out/"receipt.json").write_text(json.dumps(receipt,indent=2))
    print(receipt)
finally:
    if proc and proc.poll() is None:
        stop()
    scratch.cleanup()
