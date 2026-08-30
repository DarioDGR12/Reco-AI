# README GIFs

Source of truth for regenerating the landing GIFs.

```bash
# Preferred, if you have charmbracelet/vhs:
vhs docs/vhs/reco-ai.tape
vhs docs/vhs/reco-hw.tape

# Fallback used in this repo (Pillow + JetBrains Mono):
cargo build -p reco-cli
python3 docs/vhs/render_gifs.py
```

`render_gifs.py` also writes the coming-soon previews (`preview-tui.gif`, `preview-prueba.gif`, `preview-serve.gif`) and `banner.png`.
