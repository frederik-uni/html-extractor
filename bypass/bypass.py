import argparse
import random
import sys
import time
from io import BytesIO

from cloakbrowser import launch
from PIL import Image


def get_pixel_color(page, x, y):
    clip = {
        "x": max(0, x - 2),
        "y": max(0, y - 2),
        "width": 4,
        "height": 4,
    }

    screenshot_bytes = page.screenshot(clip=clip)
    img = Image.open(BytesIO(screenshot_bytes)).convert("RGB")
    return img.getpixel((2, 2))


def bypass(url, headless):
    browser = launch(
        geoip=True, headless=headless, humanize=True, human_preset="careful"
    )
    page = browser.new_page()
    ua = str(page.evaluate("() => navigator.userAgent"))
    page.goto(url)
    parent = page.locator('input[name="cf-turnstile-response"]').locator("xpath=..")
    parent.wait_for(state="attached", timeout=10_000)
    box = parent.bounding_box()
    if box is None:
        raise RuntimeError("Parent element is not visible or has no bounding box")

    x = box["x"] + box["height"] / 6 * 2
    y = box["y"] + box["height"] / 2
    error = box["height"] / 4

    max_click_attempts = 10
    max_title_checks = 10
    for attempt in range(max_click_attempts):
        while get_pixel_color(page, x, y) != (255, 255, 255):
            time.sleep(0.2)

        x_ = x + random.uniform(-error, error)
        y_ = y + random.uniform(-error, error)
        page.mouse.move(x_, y_)
        page.mouse.click(x_, y_)

        for _ in range(max_title_checks):
            if "Just a moment" not in page.title():
                break
            time.sleep(0.1)
        else:
            continue
        break

    else:
        raise RuntimeError("Still stuck on 'Just a moment' after 10 click attempts")

    cookies = page.context.cookies(page.url)

    bypass = next((c for c in cookies if c["name"] == "cf_clearance"), None)
    if bypass is None:
        raise RuntimeError("no cookie returned")
    browser.close()
    return bypass["value"], ua


def read_url(positional_url: str | None) -> str:
    if positional_url:
        return positional_url.strip()

    if not sys.stdin.isatty():
        return sys.stdin.read().strip()

    raise SystemExit("error: provide URL as positional argument or pipe it via stdin")


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Bypass a URL and print two returned strings."
    )
    parser.add_argument(
        "url",
        nargs="?",
        help="URL to bypass. Can also be piped via stdin.",
    )
    parser.add_argument(
        "--headless",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="Run browser headless. Default: true. Use --no-headless to see browser.",
    )

    args = parser.parse_args()
    url = read_url(args.url)
    str1, str2 = bypass(url, args.headless)
    print(str1)
    print(str2)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
