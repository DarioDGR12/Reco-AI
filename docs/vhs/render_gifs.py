#!/usr/bin/env python3
"""Render README GIFs that look like a VHS/terminal recording.

Used when VHS (charmbracelet) is not installed. The .tape files remain the
source of truth for anyone who wants to regenerate with `vhs`.
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[2]
ASSETS = ROOT / "docs" / "assets"
FONT_DIR = Path("/usr/share/fonts/truetype/jetbrains-mono")

# Catppuccin Mocha-ish
BG = (24, 24, 37)
FG = (205, 214, 244)
DIM = (108, 112, 134)
CYAN = (137, 220, 235)
GREEN = (166, 227, 161)
PEACH = (250, 179, 135)
MAUVE = (203, 166, 247)
RED = (243, 139, 168)
SURFACE = (49, 50, 68)
CRUST = (17, 17, 27)
OVERLAY = (69, 71, 90)
YELLOW = (249, 226, 175)

W, H = 980, 640
PAD = 22
LINE_H = 22
COLS = 86


def font(size: int, bold: bool = False) -> ImageFont.FreeTypeFont:
    name = "JetBrainsMono-Bold.ttf" if bold else "JetBrainsMono-Regular.ttf"
    return ImageFont.truetype(str(FONT_DIR / name), size)


def new_frame(height: int | None = None) -> Image.Image:
    img = Image.new("RGB", (W, height or H), CRUST)
    draw = ImageDraw.Draw(img)
    draw.rounded_rectangle((10, 10, W - 10, (height or H) - 10), 14, fill=BG)
    # traffic lights
    for i, color in enumerate([(243, 139, 168), (249, 226, 175), (166, 227, 161)]):
        draw.ellipse((26 + i * 18, 22, 38 + i * 18, 34), fill=color)
    draw.text((W // 2 - 40, 18), "reco  —  zsh", font=font(12), fill=DIM)
    return img


def text_size(draw: ImageDraw.ImageDraw, text: str, fnt: ImageFont.FreeTypeFont) -> tuple[int, int]:
    box = draw.textbbox((0, 0), text, font=fnt)
    return box[2] - box[0], box[3] - box[1]


def prompt_prefix() -> list[tuple[str, tuple[int, int, int]]]:
    return [
        ("reco", MAUVE),
        ("@", DIM),
        ("local", CYAN),
        (" ", FG),
        ("~/Reco-AI", GREEN),
        (" ", FG),
        ("❯ ", PEACH),
    ]


def draw_colored(
    draw: ImageDraw.ImageDraw,
    x: int,
    y: int,
    parts: list[tuple[str, tuple[int, int, int]]],
    fnt: ImageFont.FreeTypeFont,
) -> int:
    for text, color in parts:
        draw.text((x, y), text, font=fnt, fill=color)
        x += text_size(draw, text, fnt)[0]
    return x


def capture(args: list[str]) -> str:
    binary = ROOT / "target" / "debug" / "reco"
    env = {
        **__import__("os").environ,
        "NO_COLOR": "1",
        # Empty cache dir so --offline uses the bundled seed (stable GIF).
        "RECO_CACHE_DIR": str(ROOT / "target" / "reco-gif-cache"),
    }
    proc = subprocess.run(
        [str(binary), *args],
        cwd=ROOT,
        capture_output=True,
        text=True,
        env=env,
    )
    return (proc.stdout or "") + (proc.stderr or "")


def frames_typing(command: str, output_lines: list[str], height: int) -> list[Image.Image]:
    fnt = font(15)
    frames: list[Image.Image] = []

    def render(typed: str, lines: list[str], cursor: bool) -> Image.Image:
        img = new_frame(height)
        draw = ImageDraw.Draw(img)
        x, y = PAD + 8, 48
        x = draw_colored(draw, x, y, prompt_prefix(), fnt)
        draw.text((x, y), typed, font=fnt, fill=FG)
        x += text_size(draw, typed, fnt)[0]
        if cursor:
            draw.rectangle((x + 1, y + 2, x + 9, y + 18), fill=FG)
        y += LINE_H + 6
        for line in lines:
            color = FG
            stripped = line
            if stripped.startswith("╭") or stripped.startswith("╰") or stripped.startswith("├") or stripped.startswith("│"):
                color = DIM if stripped[0] in "╭╰├" else FG
            if stripped.startswith("Recomendaciones"):
                color = MAUVE
            if "reco hw" in stripped or stripped.strip().startswith("{") or stripped.strip().startswith("}"):
                color = CYAN if "reco hw" in stripped else FG
            if stripped.lstrip().startswith('"'):
                color = GREEN
            draw.text((PAD + 8, y), line[:COLS], font=fnt, fill=color)
            y += LINE_H
        return img

    frames.append(render("", [], True))
    for i in range(len(command) + 1):
        frames.append(render(command[:i], [], True))
    frames.append(render(command, [], False))
    shown: list[str] = []
    for line in output_lines:
        shown.append(line.rstrip("\n"))
        frames.append(render(command, shown, False))
    # hold
    hold = render(command, shown, False)
    frames.extend([hold] * 8)
    return frames


def save_gif(path: Path, frames: list[Image.Image], duration: int = 70) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    frames[0].save(
        path,
        save_all=True,
        append_images=frames[1:],
        duration=duration,
        loop=0,
        optimize=True,
    )
    print(f"wrote {path} ({path.stat().st_size // 1024} KB, {len(frames)} frames)")


def gif_reco_ai() -> None:
    raw = capture(["ai", "--offline", "--fixture", "rtx4060", "--limit", "5", "--list"])
    cleaned = [strip_ansi(ln) for ln in raw.splitlines()]
    frames = frames_typing(
        "reco ai --offline --fixture rtx4060 --limit 5 --list", cleaned, 640
    )
    save_gif(ASSETS / "demo-reco-ai.gif", frames, duration=65)


def gif_reco_hw() -> None:
    raw = strip_ansi(capture(["hw", "--json"]))
    frames = frames_typing("reco hw --json", raw.splitlines(), 560)
    save_gif(ASSETS / "demo-reco-hw.gif", frames, duration=60)


def strip_ansi(text: str) -> str:
    out = []
    i = 0
    while i < len(text):
        if text[i] == "\x1b":
            i += 1
            if i < len(text) and text[i] == "[":
                i += 1
                while i < len(text) and not text[i].isalpha():
                    i += 1
                i += 1
            continue
        out.append(text[i])
        i += 1
    return "".join(out)


def gif_reco_run() -> None:
    raw = strip_ansi(
        capture(
            [
                "run",
                "Llama-3.1-8B",
                "--offline",
                "--fixture",
                "rtx4060",
                "--dry-run",
            ]
        )
    )
    frames = frames_typing(
        "reco run Llama-3.1-8B --offline --fixture rtx4060 --dry-run",
        raw.splitlines(),
        420,
    )
    save_gif(ASSETS / "demo-reco-run.gif", frames, duration=60)


def gif_tui_preview() -> None:
    frames = []
    models = [
        ("›  Qwen2.5-7B-Instruct Q4_K_M", "7.6 GB   cabe en 8 GB VRAM   ★ 94"),
        ("   Llama-3.1-8B-Instruct Q4_K_M", "4.9 GB   rápido en RTX 4060   ★ 91"),
        ("   Mistral-7B-Instruct Q5_K_M", "5.3 GB   calidad alta         ★ 88"),
        ("   Phi-3.5-mini Q8_0", "4.1 GB   liviano / CPU ok     ★ 80"),
        ("   Gemma-2-9B Q3_K_M", "4.8 GB   justo de VRAM        ★ 77"),
    ]
    fnt = font(15)
    small = font(13)
    for highlight in range(3):
        img = new_frame(420)
        draw = ImageDraw.Draw(img)
        y = 50
        draw.text((PAD + 8, y), "Reco AI  ·  recomendaciones para RTX 4060 8 GB", font=font(15, True), fill=MAUVE)
        y += 28
        draw.text((PAD + 8, y), "↑↓ navegar   enter descargar   / buscar   q salir", font=small, fill=DIM)
        y += 30
        for i, (title, meta) in enumerate(models):
            selected = i == highlight
            box_y = y + i * 48
            if selected:
                draw.rounded_rectangle((PAD, box_y - 4, W - PAD, box_y + 40), 8, fill=SURFACE)
            color = PEACH if selected else FG
            draw.text((PAD + 14, box_y), title if not selected else title.replace("   ", "›  ", 1), font=fnt, fill=color)
            draw.text((PAD + 36, box_y + 20), meta, font=small, fill=CYAN if selected else DIM)
        draw.text((PAD + 8, 380), "reco ai  ·  TUI Ratatui", font=small, fill=YELLOW)
        frames.extend([img] * 10)
    save_gif(ASSETS / "preview-tui.gif", frames, duration=180)


def gif_prueba_preview() -> None:
    fnt = font(15)
    small = font(13)
    bubbles = [
        ("user", "Explícame la cuantización Q4_K_M como si tuviera una RTX 4060 de 8 GB."),
        ("ai", "Q4_K_M guarda los pesos en ~4.5 bits. Un 7B cabe en ~4.5–5 GB, así que te queda VRAM para el contexto. En tu 4060 es el sweet spot: calidad alta sin paginar a RAM."),
        ("user", "¿Y si quiero más calidad?"),
        ("ai", "Prueba Q5_K_M si el modelo es 7B. Q8_0 solo si bajas a 3B–4B. Reco te lo va a marcar cuando el catálogo esté indexado."),
    ]
    frames = []
    shown: list[tuple[str, str]] = []
    for who, text in bubbles:
        shown.append((who, text))
        img = new_frame(500)
        draw = ImageDraw.Draw(img)
        draw.text((PAD + 8, 48), "Prueba", font=font(16, True), fill=MAUVE)
        draw.text((PAD + 90, 50), "Qwen2.5-7B-Instruct · Q4_K_M · local", font=small, fill=DIM)
        y = 86
        for role, msg in shown:
            wrap = wrap_text(msg, 68)
            h = 16 + len(wrap) * 20
            if role == "user":
                draw.rounded_rectangle((W - PAD - 640, y, W - PAD, y + h), 10, fill=SURFACE)
                color = FG
                x = W - PAD - 624
            else:
                draw.rounded_rectangle((PAD, y, PAD + 700, y + h), 10, fill=(30, 41, 59))
                color = GREEN
                x = PAD + 16
            for i, line in enumerate(wrap):
                draw.text((x, y + 8 + i * 20), line, font=fnt, fill=color)
            y += h + 12
        draw.rounded_rectangle((PAD, 440, W - PAD, 476), 8, fill=SURFACE)
        draw.text((PAD + 16, 450), "Escribe un mensaje…                          preview · próximamente", font=small, fill=DIM)
        frames.extend([img] * 12)
    save_gif(ASSETS / "preview-prueba.gif", frames, duration=200)


def gif_serve_preview() -> None:
    lines = [
        "reco serve Qwen2.5-7B-Instruct",
        "",
        "Servidor local en http://127.0.0.1:11434",
        "Modelo     Qwen2.5-7B-Instruct (Q4_K_M)",
        "API key    sk-reco-8f3a2c91e0b74d1a",
        "",
        "curl http://127.0.0.1:11434/v1/chat/completions \\",
        "  -H \"Authorization: Bearer sk-reco-8f3a2c91e0b74d1a\"",
        "",
        "preview · reco serve · próximamente",
    ]
    fnt = font(15)
    frames = []
    typed = ""
    cmd = lines[0]
    for i in range(len(cmd) + 1):
        img = new_frame(420)
        draw = ImageDraw.Draw(img)
        x = draw_colored(draw, PAD + 8, 52, prompt_prefix(), fnt)
        draw.text((x, 52), cmd[:i], font=fnt, fill=FG)
        frames.append(img)
    shown = [cmd]
    for extra in lines[1:]:
        shown.append(extra)
        img = new_frame(420)
        draw = ImageDraw.Draw(img)
        x = draw_colored(draw, PAD + 8, 52, prompt_prefix(), fnt)
        y = 52
        for idx, line in enumerate(shown):
            color = YELLOW if "preview" in line else (CYAN if "sk-reco" in line or line.startswith("curl") else FG)
            if idx == 0:
                draw.text((x, y), line, font=fnt, fill=FG)
            else:
                draw.text((PAD + 8, y), line, font=fnt, fill=color)
            y += LINE_H
        frames.append(img)
    frames.extend([frames[-1]] * 10)
    save_gif(ASSETS / "preview-serve.gif", frames, duration=70)


def wrap_text(text: str, width: int) -> list[str]:
    words = text.split()
    lines: list[str] = []
    cur = ""
    for word in words:
        trial = f"{cur} {word}".strip()
        if len(trial) <= width:
            cur = trial
        else:
            if cur:
                lines.append(cur)
            cur = word
    if cur:
        lines.append(cur)
    return lines


def banner() -> None:
    img = Image.new("RGB", (1280, 280), CRUST)
    draw = ImageDraw.Draw(img)
    draw.rounded_rectangle((16, 16, 1264, 264), 20, fill=BG)
    draw.text((56, 58), "Reco AI", font=font(52, True), fill=MAUVE)
    draw.text((56, 130), "From Hugging Face GGUF to a local chat —", font=font(22), fill=FG)
    draw.text((56, 166), "picked for your actual GPU, RAM, and CPU.", font=font(22), fill=CYAN)
    draw.text((56, 214), "Windows  ·  macOS  ·  Linux    reco ai", font=font(16), fill=DIM)
    path = ASSETS / "banner.png"
    img.save(path)
    print(f"wrote {path}")


def main() -> int:
    ASSETS.mkdir(parents=True, exist_ok=True)
    banner()
    gif_reco_ai()
    gif_reco_hw()
    gif_reco_run()
    gif_tui_preview()
    gif_prueba_preview()
    gif_serve_preview()
    return 0


if __name__ == "__main__":
    sys.exit(main())
