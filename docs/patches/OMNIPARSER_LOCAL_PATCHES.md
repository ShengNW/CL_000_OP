# OmniParser Local Patches (Non-Upstream)

This repo does not vendor OmniParser. The local changes required to run within the guarded environments are provided as a patch:

- Patch file: `docs/patches/omniparser-local.patch`
- Apply from OmniParser repo root:

```bash
git apply docs/patches/omniparser-local.patch
```

## Patch Summary

1) `util/utils.py`
- Make `openai` import optional (avoid hard crash when package not installed).
- Make `paddleocr` optional and fall back to EasyOCR when missing or misconfigured.
- Respect `EASYOCR_HOME`/`PADDLE_PDX_CACHE_HOME` for model cache placement.
- Lazy-init EasyOCR and return empty OCR results if init fails (keeps parse alive on older torch stacks).
- Normalize `filtered_boxes` to dict form when OCR is empty (avoids list/dict mismatch).
- Force EasyOCR to CPU and float32 to avoid mixed-precision errors.
- If PaddleOCR runtime fails, fall back to EasyOCR.

2) `util/omniparser.py`
- Catch caption model load failures (Florence2 / transformers mismatch) and fall back to `use_local_semantics=False`.
- Prefer PaddleOCR when available; otherwise keep EasyOCR fallback.

These changes are purely defensive and keep the upstream API intact.
