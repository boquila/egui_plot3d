"""Exercise the actual WASM gallery in Chromium; save visual evidence and fail on errors."""
import argparse
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from io import BytesIO
import json
from pathlib import Path
import threading

from PIL import Image, ImageChops
from playwright.sync_api import sync_playwright


class QuietHandler(SimpleHTTPRequestHandler):
    def log_message(self, *_):
        pass


def changed_pixels(a, b):
    diff = ImageChops.difference(Image.open(BytesIO(a)).convert("RGB"),
                                 Image.open(BytesIO(b)).convert("RGB"))
    return sum(max(pixel) > 15 for pixel in diff.get_flattened_data())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--url", default="http://127.0.0.1:8080/")
    parser.add_argument("--serve-dir", type=Path)
    parser.add_argument("--base-path", default="/", help="URL path when serving a prefixed Pages build")
    parser.add_argument("--browser-path")
    parser.add_argument("--output", type=Path, default=Path("artifacts"))
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    server = None
    if args.serve_dir:
        server = ThreadingHTTPServer(("127.0.0.1", 0), partial(QuietHandler, directory=str(args.serve_dir.resolve())))
        threading.Thread(target=server.serve_forever, daemon=True).start()
        args.url = f"http://127.0.0.1:{server.server_port}/{args.base_path.strip('/')}/"
        args.url = args.url.replace(f":{server.server_port}//", f":{server.server_port}/")
    errors, checks = [], []
    try:
        with sync_playwright() as p:
            options = {"headless": True, "args": ["--enable-webgl", "--ignore-gpu-blocklist", "--enable-unsafe-swiftshader"]}
            if args.browser_path:
                options["executable_path"] = args.browser_path
            browser = p.chromium.launch(**options)
            page = browser.new_page(viewport={"width": 1440, "height": 900}, device_scale_factor=1, color_scheme="light")
            page.on("pageerror", lambda error: errors.append(str(error)))
            page.on("response", lambda response: errors.append(f"HTTP {response.status}: {response.url}") if response.status >= 400 else None)
            page.on("console", lambda message: errors.append(message.text) if message.type == "error" else None)

            def settle():
                page.mouse.move(1000, 850)
                page.wait_for_timeout(350)

            def snapshot(name):
                settle()
                page.screenshot(path=str(args.output / f"{name}.png"))

            def plot_image():
                return page.screenshot(clip={"x": 430, "y": 160, "width": 400, "height": 700})

            def expect_change(before, action, minimum=300):
                settle()
                difference = changed_pixels(before, plot_image())
                assert difference > minimum, f"{action} did not change plot pixels ({difference})"
                checks.append({"action": action, "changed_pixels": difference})

            page.goto(args.url)
            page.locator("#loading").wait_for(state="detached", timeout=60000)
            snapshot("gallery-home")
            page.mouse.click(60, 110)
            snapshot("cloud")
            cloud = plot_image()
            # Only the plot region contributes to these differences, not widgets or tooltips.
            page.mouse.move(620, 480)
            page.mouse.down()
            page.mouse.move(700, 520, steps=12)
            page.mouse.up()
            expect_change(cloud, "orbit cloud")
            before = plot_image()
            page.mouse.move(620, 480)
            page.mouse.down(button="right")
            page.mouse.move(655, 505, steps=8)
            page.mouse.up(button="right")
            expect_change(before, "pan cloud")
            before = plot_image()
            page.mouse.move(620, 480)
            page.mouse.wheel(0, -180)
            expect_change(before, "zoom cloud")
            before = plot_image()
            page.mouse.click(660, 40)
            expect_change(before, "reset cloud")
            before = plot_image()
            page.mouse.click(770, 105)
            expect_change(before, "hide cloud legend series")
            page.mouse.click(770, 105)
            settle()
            page.mouse.click(190, 110)
            snapshot("lorenz")
            expect_change(cloud, "select Lorenz")
            before = plot_image()
            page.mouse.click(550, 60)  # Nearby initial condition.
            expect_change(before, "nearby Lorenz trajectory")
            for name, x, y in [("waves", 310,110), ("kepler",60,230), ("gaussian",190,230), ("harmonics",310,230), ("dipole",60,350)]:
                before = plot_image()
                page.mouse.click(x,y)
                snapshot(name)
                expect_change(before, f"select {name}")
                before = plot_image()
                page.mouse.click(480,60)  # Compare setting, changes a model parameter.
                expect_change(before, f"change {name} parameter")
                snapshot(f"{name}-comparison")
                page.mouse.click(480,60)
                settle()
                if name in ("waves", "kepler", "harmonics"):
                    page.mouse.click(545,60)  # Animate.
                    settle()
                    before = plot_image()
                    page.wait_for_timeout(650)
                    # A moving 6 px marker changes roughly 200 pixels; the rest of
                    # the orbit stays fixed. Wave animation changes the surface.
                    expect_change(before, f"animate {name}", minimum=80)
                    page.mouse.click(545,60)
                    settle()
                    before = plot_image()
                    page.wait_for_timeout(500)
                    difference = changed_pixels(before, plot_image())
                    assert difference < 30, f"{name} kept animating after pause ({difference})"
                    checks.append({"action": f"pause {name}", "changed_pixels": difference})
                if name in ("waves", "harmonics", "dipole"):
                    # Keep the camera and particle stationary. Changes must come
                    # from the function values across the plotted geometry.
                    motion_y = 97 if name == "waves" else 81
                    original = plot_image()
                    page.mouse.click(430, motion_y)
                    settle()
                    before = plot_image()
                    page.wait_for_timeout(900)
                    expect_change(before, f"animate {name} values", minimum=1500)
                    snapshot(f"{name}-value-animation")
                    page.mouse.click(430, motion_y)
                    settle()
                    before = plot_image()
                    page.wait_for_timeout(500)
                    difference = changed_pixels(before, plot_image())
                    assert difference < 30, f"{name} values did not freeze ({difference})"
                    checks.append({"action": f"freeze {name} values", "changed_pixels": difference})
                    page.mouse.click(575, motion_y)  # Reset motion.
                    settle()
                    difference = changed_pixels(original, plot_image())
                    assert difference < 30, f"{name} reset did not restore base geometry ({difference})"
                    checks.append({"action": f"reset {name} values", "changed_pixels": difference})
            page.mouse.click(85, 10)
            snapshot("gallery-dark")
            page.set_viewport_size({"width": 390, "height": 844})
            snapshot("mobile")
            mobile = Image.open(args.output / "mobile.png").convert("RGB")
            colorful = sum(max(c)-min(c)>50 and max(c)>90 for c in mobile.crop((160,200,390,844)).get_flattened_data())
            assert colorful > 100, "Resizing hid the active plot"
            checks.append({"action": "mobile plot visible", "colored_pixels": colorful})
            assert not errors, errors
            report = {"url": args.url, "title": page.title(), "checks": checks, "errors": errors}
            (args.output / "browser-report.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
            print(json.dumps(report, indent=2))
            browser.close()
    finally:
        if server:
            server.shutdown()
            server.server_close()


if __name__ == "__main__":
    main()
